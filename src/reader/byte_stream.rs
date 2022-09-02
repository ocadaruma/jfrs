//! Provides functionality to read primitives from JFR byte stream.
//!
//! Related JMC code: [SeekableInputStream.java](https://github.com/openjdk/jmc/blob/8.2.0-ga/core/org.openjdk.jmc.flightrecorder/src/main/java/org/openjdk/jmc/flightrecorder/internal/parser/v1/SeekableInputStream.java)

use crate::reader::Result;
use crate::reader::Error;
use std::io::{Read, Seek, SeekFrom};

const STRING_ENCODING_NULL: i8 = 0;
const STRING_ENCODING_EMPTY_STRING: i8 = 1;
const STRING_ENCODING_CONSTANT_POOL: i8 = 2;
const STRING_ENCODING_UTF8_BYTE_ARRAY: i8 = 3;
const STRING_ENCODING_CHAR_ARRAY: i8 = 4;
const STRING_ENCODING_LATIN1_BYTE_ARRAY: i8 = 5;

#[derive(Debug, Eq, PartialEq)]
pub enum StringType {
    Null,
    Empty,
    Raw(String),
    ConstantPool(i64),
}

#[derive(Debug)]
pub enum IntEncoding {
    Raw,
    Compressed, // varint encoding, but not ZigZag
}

#[macro_use]
mod macros {
    macro_rules! read_num {
        ($self:ident, $ty:ty, $size:expr) => {
            return $self.read_exact().map(<$ty>::from_be_bytes)
        };
    }
}

pub struct ByteStream<T> {
    inner: T,
    int_encoding: IntEncoding,
}

impl<T: Read> ByteStream<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            int_encoding: IntEncoding::Raw,
        }
    }

    pub fn set_int_encoding(&mut self, encoding: IntEncoding) {
        self.int_encoding = encoding;
    }

    pub fn read_exact<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0; N];
        self.inner.read_exact(&mut buf).map_err(Error::IoError)?;
        Ok(buf)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        read_num!(self, u8, 1);
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        read_num!(self, i8, 1);
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        match self.int_encoding {
            IntEncoding::Raw => read_num!(self, i16, 2),
            IntEncoding::Compressed => self.read_var_i64().map(|i| i as i16),
        }
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        match self.int_encoding {
            IntEncoding::Raw => read_num!(self, i32, 4),
            IntEncoding::Compressed => self.read_var_i64().map(|i| i as i32),
        }
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        match self.int_encoding {
            IntEncoding::Raw => read_num!(self, i64, 4),
            IntEncoding::Compressed => self.read_var_i64(),
        }
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        self.read_exact().map(f32::from_be_bytes)
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        self.read_exact().map(f64::from_be_bytes)
    }

    fn read_var_i64(&mut self) -> Result<i64> {
        let mut ret = 0i64;
        for i in 0..8 {
            let b = self.read_i8()? as i64;
            ret += (b & 0x7f) << (7 * i);
            if b >= 0 {
                return Ok(ret);
            }
        }
        Ok(ret + ((self.read_i8()? as i64 & 0xff) << 56))
    }

    pub fn read_string(&mut self) -> Result<StringType> {
        let encoding = self.read_i8()?;
        if encoding == STRING_ENCODING_NULL {
            return Ok(StringType::Null);
        }
        if encoding == STRING_ENCODING_EMPTY_STRING {
            return Ok(StringType::Empty);
        }
        if encoding == STRING_ENCODING_CONSTANT_POOL {
            return self.read_i64().map(StringType::ConstantPool);
        }

        let size = self.read_i32()? as usize;
        if encoding == STRING_ENCODING_CHAR_ARRAY {
            let mut buf = Vec::with_capacity(size);
            for _ in 0..size {
                let c = self.read_i16()? as u32;
                buf.push(char::try_from(c).map_err(|_| Error::InvalidString)?);
            }
            return Ok(StringType::Raw(buf.iter().collect()));
        }

        let mut buf = Vec::with_capacity(size);
        for _ in 0..size {
            buf.push(self.read_i8()? as u8);
        }
        if encoding == STRING_ENCODING_LATIN1_BYTE_ARRAY {
            return Ok(StringType::Raw(buf.iter().map(|&c| c as char).collect()));
        }
        if encoding == STRING_ENCODING_UTF8_BYTE_ARRAY {
            return Ok(StringType::Raw(
                String::from_utf8(buf).map_err(|_| Error::InvalidString)?,
            ));
        }

        Err(Error::InvalidString)
    }
}

impl<T: Read + Seek> ByteStream<T> {
    pub fn seek(&mut self, position: u64) -> Result<()> {
        self.inner
            .seek(SeekFrom::Start(position))
            .map(drop)
            .map_err(Error::IoError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_i64_compressed() {
        let bytes = [0x85u8, 0xb0, 0x3];
        let mut s = ByteStream::new(Cursor::new(bytes));
        s.int_encoding = IntEncoding::Compressed;
        assert_eq!(55301, s.read_i64().unwrap());
    }

    #[test]
    fn test_read_string_null() {
        let bytes = [STRING_ENCODING_NULL as u8];
        let mut s = ByteStream::new(Cursor::new(bytes));
        s.int_encoding = IntEncoding::Compressed;
        assert_eq!(StringType::Null, s.read_string().unwrap());
    }

    #[test]
    fn test_read_string_empty() {
        let bytes = [STRING_ENCODING_EMPTY_STRING as u8];
        let mut s = ByteStream::new(Cursor::new(bytes));
        s.int_encoding = IntEncoding::Compressed;
        assert_eq!(StringType::Empty, s.read_string().unwrap());
    }

    #[test]
    fn test_read_string_constant_pool() {
        let mut bytes = vec![STRING_ENCODING_CONSTANT_POOL as u8];
        bytes.append(&mut vec![0x85, 0xb0, 0x3]);
        let mut s = ByteStream::new(Cursor::new(bytes));
        s.int_encoding = IntEncoding::Compressed;
        assert_eq!(StringType::ConstantPool(55301), s.read_string().unwrap());
    }

    #[test]
    fn test_read_string_utf8() {
        let mut bytes = vec![STRING_ENCODING_UTF8_BYTE_ARRAY as u8];
        bytes.push(11); // length of "hello,world" in varint encoding
        bytes.extend_from_slice("hello,world".as_bytes());
        let mut s = ByteStream::new(Cursor::new(bytes));
        s.int_encoding = IntEncoding::Compressed;
        assert_eq!(
            StringType::Raw("hello,world".to_string()),
            s.read_string().unwrap()
        );
    }
}

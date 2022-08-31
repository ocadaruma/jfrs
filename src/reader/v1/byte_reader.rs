use crate::reader::{Error, Result};
use byteorder::{ReadBytesExt, BE};
use std::io::Read;

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
pub enum ByteReader {
    Raw,
    CompressedInts,
}

impl ByteReader {
    pub fn read_i8(&self, r: &mut impl Read) -> Result<i8> {
        ReadBytesExt::read_i8(r).map_err(Error::IoError)
    }

    pub fn read_i16(&self, r: &mut impl Read) -> Result<i16> {
        match self {
            ByteReader::Raw => ReadBytesExt::read_i16::<BE>(r).map_err(Error::IoError),
            ByteReader::CompressedInts => Self::read_var_i64(self, r).map(|i| i as i16),
        }
    }

    pub fn read_i32(&self, r: &mut impl Read) -> Result<i32> {
        match self {
            ByteReader::Raw => ReadBytesExt::read_i32::<BE>(r).map_err(Error::IoError),
            ByteReader::CompressedInts => Self::read_var_i64(self, r).map(|i| i as i32),
        }
    }

    pub fn read_i64(&self, r: &mut impl Read) -> Result<i64> {
        match self {
            ByteReader::Raw => ReadBytesExt::read_i64::<BE>(r).map_err(Error::IoError),
            ByteReader::CompressedInts => Self::read_var_i64(self, r),
        }
    }

    pub fn read_f32(&self, r: &mut impl Read) -> Result<f32> {
        ReadBytesExt::read_f32::<BE>(r).map_err(Error::IoError)
    }

    pub fn read_f64(&self, r: &mut impl Read) -> Result<f64> {
        ReadBytesExt::read_f64::<BE>(r).map_err(Error::IoError)
    }

    pub fn read_string(&self, r: &mut impl Read) -> Result<StringType> {
        let encoding = self.read_i8(r)?;
        if encoding == STRING_ENCODING_NULL {
            return Ok(StringType::Null);
        }
        if encoding == STRING_ENCODING_EMPTY_STRING {
            return Ok(StringType::Empty);
        }
        if encoding == STRING_ENCODING_CONSTANT_POOL {
            return Ok(StringType::ConstantPool(self.read_i64(r)?));
        }

        let size = self.read_i32(r)? as usize;
        if encoding == STRING_ENCODING_CHAR_ARRAY {
            let mut buf = Vec::with_capacity(size);
            for _ in 0..size {
                let c = self.read_i16(r)? as u32;
                buf.push(char::try_from(c).map_err(|_| Error::InvalidString)?);
            }
            return Ok(StringType::Raw(buf.iter().collect()));
        }

        let mut buf = Vec::with_capacity(size);
        for _ in 0..size {
            buf.push(self.read_i8(r)? as u8);
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

    fn read_var_i64(&self, r: &mut impl Read) -> Result<i64> {
        let mut ret = 0i64;
        for i in 0..8 {
            let b = Self::read_i8(self, r)? as i64;
            ret += (b & 0x7f) << (7 * i);
            if b >= 0 {
                return Ok(ret);
            }
        }
        Ok(ret + ((Self::read_i8(self, r)? as i64 & 0xff) << 56))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_i64_compressed() {
        let bytes = [0x85u8, 0xb0, 0x3];
        assert_eq!(
            55301,
            ByteReader::CompressedInts
                .read_i64(&mut Cursor::new(bytes))
                .unwrap()
        );
    }

    #[test]
    fn test_read_string_null() {
        let bytes = [STRING_ENCODING_NULL as u8];
        assert_eq!(
            StringType::Null,
            ByteReader::CompressedInts
                .read_string(&mut Cursor::new(bytes))
                .unwrap()
        );
    }

    #[test]
    fn test_read_string_empty() {
        let bytes = [STRING_ENCODING_EMPTY_STRING as u8];
        assert_eq!(
            StringType::Empty,
            ByteReader::CompressedInts
                .read_string(&mut Cursor::new(bytes))
                .unwrap()
        );
    }

    #[test]
    fn test_read_string_constant_pool() {
        let mut bytes = vec![STRING_ENCODING_CONSTANT_POOL as u8];
        bytes.append(&mut vec![0x85, 0xb0, 0x3]);
        assert_eq!(
            StringType::ConstantPool(55301),
            ByteReader::CompressedInts
                .read_string(&mut Cursor::new(bytes))
                .unwrap()
        );
    }

    #[test]
    fn test_read_string_utf8() {
        let mut bytes = vec![STRING_ENCODING_UTF8_BYTE_ARRAY as u8];
        bytes.push(11); // length of "hello,world" in varint encoding
        bytes.extend_from_slice("hello,world".as_bytes());
        assert_eq!(
            StringType::Raw("hello,world".to_string()),
            ByteReader::CompressedInts
                .read_string(&mut Cursor::new(bytes))
                .unwrap()
        );
    }
}

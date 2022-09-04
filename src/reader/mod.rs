//! Module to read JFR files and parse as Rust data structures.

use crate::reader::byte_stream::{ByteStream, IntEncoding};
use crate::reader::constant_pool::ConstantPool;
use crate::reader::event::EventIterator;
use crate::reader::metadata::Metadata;
use crate::{Version, MAGIC, VERSION_1, VERSION_2};
use std::fmt::Formatter;
use std::io::{Cursor, Read, Seek};
use std::{fmt, io};

mod byte_stream;
mod constant_pool;
mod de;
mod event;
mod metadata;
mod type_descriptor;
pub mod types;
pub mod value_descriptor;

#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    InvalidStringIndex(i32),
    InvalidString,
    InvalidChar(std::char::CharTryFromError),
    UnsupportedVersion(Version),
    ClassNotFound(i64),
    IoError(io::Error),
    DeserializeError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidFormat => write!(f, "Invalid format"),
            Error::InvalidStringIndex(i) => write!(f, "Invalid string index in pool: {}", i),
            Error::InvalidString => write!(f, "Invalid string"),
            Error::InvalidChar(e) => write!(f, "Invalid char: {}", e),
            Error::UnsupportedVersion(v) => write!(f, "Unsupported version: {}", v),
            Error::ClassNotFound(i) => write!(f, "Class not found for id: {}", i),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::DeserializeError(msg) => write!(f, "Failed to deserialize: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
type HeapByteStream = ByteStream<Cursor<Vec<u8>>>;

#[derive(Debug)]
pub struct ChunkHeader {
    pub chunk_size: i64,
    constant_pool_offset: i64,
    metadata_offset: i64,
    pub start_time_nanos: i64,
    pub duration_nanos: i64,
    pub start_ticks: i64,
    pub ticks_per_second: i64,
    features: i32,
}

impl ChunkHeader {
    /// The size from the beginning of the chunk (right before MAGIC) to the header end
    const HEADER_SIZE: u64 = 68;
    const FEATURES_COMPRESSED_INTS: i32 = 1;

    fn int_encoding(&self) -> IntEncoding {
        if self.features & Self::FEATURES_COMPRESSED_INTS != 0 {
            IntEncoding::Compressed
        } else {
            IntEncoding::Raw
        }
    }

    fn chunk_body_size(&self) -> u64 {
        self.chunk_size as u64 - Self::HEADER_SIZE
    }

    fn body_start_offset(&self) -> u64 {
        Self::HEADER_SIZE
    }
}

pub struct Chunk {
    pub header: ChunkHeader,
    metadata: Metadata,
    constant_pool: ConstantPool,
}

pub struct ChunkReader {
    stream: HeapByteStream,
}

impl ChunkReader {
    pub fn events<'a>(self, chunk: &'a Chunk) -> EventIterator<'a> {
        EventIterator::new(chunk, self.stream)
    }
}

pub struct ChunkIterator<'a, T> {
    reader: &'a mut JfrReader<T>,
}

impl<'a, T: Read + Seek> Iterator for ChunkIterator<'a, T> {
    type Item = Result<(ChunkReader, Chunk)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_next() {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<'a, T: Read + Seek> ChunkIterator<'a, T> {
    fn internal_next(&mut self) -> Result<Option<(ChunkReader, Chunk)>> {
        self.reader.stream.set_int_encoding(IntEncoding::Raw);
        self.reader.stream.seek(self.reader.chunk_start_position)?;
        match self.reader.stream.read_u8() {
            Ok(magic_head) => {
                let mut magic = [magic_head, 0, 0, 0];
                let magic_tail: [u8; 3] = self.reader.stream.read_exact()?;
                magic[1..].clone_from_slice(&magic_tail);

                if magic != MAGIC {
                    return Err(Error::InvalidFormat);
                }
            }
            // Reaching EOF at the beginning of the chunk means just we reached the end of the file
            // normally, so just returns Ok(None)
            Err(Error::IoError(e)) if e.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok(None);
            }
            Err(e) => {
                return Err(e);
            }
        }

        let version = Version {
            major: self.reader.stream.read_i16()?,
            minor: self.reader.stream.read_i16()?,
        };
        match version {
            VERSION_1 | VERSION_2 => {}
            _ => {
                return Err(Error::UnsupportedVersion(version));
            }
        }

        let chunk_size = self.reader.stream.read_i64()?;

        // To reduce the overhead of read against the file, we load entire chunk into memory
        // and do all further operations on it.
        self.reader.stream.seek(self.reader.chunk_start_position)?;
        let mut heap_stream = ByteStream::new(Cursor::new(
            self.reader.stream.read_as_bytes(chunk_size as usize)?,
        ));
        // magic + version + chunk_size
        heap_stream.seek(4 + 4 + 8)?;

        let header = Self::read_chunk_header(&mut heap_stream, chunk_size)?;
        heap_stream.set_int_encoding(header.int_encoding());

        let metadata = Metadata::try_new(&mut heap_stream, &header)?;
        let constant_pool = ConstantPool::try_new(&mut heap_stream, &header, &metadata)?;

        // update to next chunk start
        self.reader.chunk_start_position += chunk_size as u64;

        Ok(Some((
            ChunkReader {
                stream: heap_stream,
            },
            Chunk {
                header,
                metadata,
                constant_pool,
            },
        )))
    }

    fn read_chunk_header(stream: &mut HeapByteStream, chunk_size: i64) -> Result<ChunkHeader> {
        Ok(ChunkHeader {
            chunk_size: chunk_size,
            constant_pool_offset: stream.read_i64()?,
            metadata_offset: stream.read_i64()?,
            start_time_nanos: stream.read_i64()?,
            duration_nanos: stream.read_i64()?,
            start_ticks: stream.read_i64()?,
            ticks_per_second: stream.read_i64()?,
            features: stream.read_i32()?,
        })
    }
}

pub struct JfrReader<T> {
    stream: ByteStream<T>,
    chunk_start_position: u64,
}

impl<T> JfrReader<T>
where
    T: Read + Seek,
{
    pub fn new(inner: T) -> Self {
        Self {
            stream: ByteStream::new(inner),
            chunk_start_position: 0,
        }
    }

    pub fn chunks(&mut self) -> ChunkIterator<T> {
        ChunkIterator { reader: self }
    }
}

pub use de::from_event;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    use crate::reader::types::jdk::ExecutionSample;
    use crate::reader::value_descriptor::{Primitive, ValueDescriptor};

    use std::path::PathBuf;

    #[test]
    fn test_read_single_chunk() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/profiler-wall.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());

        let mut chunk_count = 0;
        for res in reader.chunks() {
            let res = res.unwrap();
            let (reader, chunk) = res;
            chunk_count += 1;

            // You can see these values on JMC
            assert_eq!(chunk.constant_pool.inner.len(), 9);

            // class_id:30 = jdk.types.Symbol
            assert_eq!(128, chunk.constant_pool.inner.get(&30).unwrap().inner.len());

            // constant_index: 203 for jdk.types.Symbol
            let field = chunk
                .constant_pool
                .get(&30, &203)
                .and_then(|c| c.get_field("string", &chunk))
                .unwrap();
            if let ValueDescriptor::Primitive(Primitive::String(s)) = field {
                assert_eq!(s, "CompileBroker::compiler_thread_loop");
            } else {
                panic!("Unexpected value type: {:?}", field);
            }

            let count = reader
                .events(&chunk)
                .flatten()
                .filter(|e| e.class.name.as_ref() == "jdk.ExecutionSample")
                .fold(0, |a, _| a + 1);
            assert_eq!(count, 8836);
        }

        assert_eq!(chunk_count, 1);
    }

    #[test]
    fn test_read_multiple_chunk() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/profiler-multichunk.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());
        let chunk_count = reader.chunks().flatten().fold(0, |a, _| a + 1);

        assert_eq!(chunk_count, 3);
    }

    #[test]
    fn test_read_recording() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/recording.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());

        let mut chunk_count = 0;
        for (_reader, chunk) in reader.chunks().flatten() {
            // class_id:20 = java.lang.Class
            assert_eq!(52, chunk.constant_pool.inner.get(&20).unwrap().inner.len());
            chunk_count += 1;
        }

        assert_eq!(chunk_count, 1);
    }

    #[test]
    fn test_de() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/profiler-wall.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());

        let mut chunk_count = 0;
        for (reader, chunk) in reader.chunks().flatten() {
            chunk_count += 1;
            for event in reader
                .events(&chunk)
                .flatten()
                .filter(|e| e.class.name.as_ref() == "jdk.ExecutionSample")
            {
                let sample: ExecutionSample = from_event(&chunk, &event).unwrap();
                println!("{}", sample.sampled_thread.unwrap().os_name.unwrap());
                // match event.value.get_field("sampledThread", &chunk)
                //     .and_then(|t| t.get_field("osName", &chunk)) {
                //     Some(ValueDescriptor::Primitive(Primitive::String(s))) => {
                //
                //     }
                //     _ => {}
                // }
            }
        }

        assert_eq!(chunk_count, 1);
    }
}

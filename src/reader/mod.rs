//! Module to read JFR files and parse as Rust data structures.

use crate::reader::byte_stream::{ByteStream, IntEncoding};
use crate::reader::constant_pool::ConstantPool;
use crate::reader::metadata::Metadata;
use crate::{Version, MAGIC, VERSION_1, VERSION_2};
use byteorder::{ReadBytesExt, BE};
use std::io;
use std::io::{BufReader, Read, Seek};
use std::marker::PhantomData;

mod byte_stream;
mod constant_pool;
mod metadata;
mod type_descriptor;
mod value_descriptor;

#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    InvalidStringIndex(i32),
    InvalidString,
    UnsupportedVersion(Version),
    IoError(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ChunkHeader {
    chunk_size: i64,
    constant_pool_offset: i64,
    metadata_offset: i64,
    start_time_nanos: i64,
    duration_nanos: i64,
    start_ticks: i64,
    ticks_per_second: i64,
    features: i32,
    absolute_chunk_start_position: u64,
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

    fn absolute_metadata_start_position(&self) -> u64 {
        self.absolute_chunk_start_position + self.metadata_offset as u64
    }

    fn absolute_body_start_position(&self) -> u64 {
        self.absolute_chunk_start_position + Self::HEADER_SIZE
    }
}

#[derive(Debug)]
pub struct Chunk {
    header: ChunkHeader,
    metadata: Metadata,
    constant_pool: ConstantPool,
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

    pub fn next(&mut self) -> Option<Result<Chunk>> {
        match self.next_chunk() {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    pub fn events(&mut self, chunk: Chunk) -> EventIterator<T> {
        EventIterator {
            chunk,
            stream: &mut self.stream,
        }
    }

    fn next_chunk(&mut self) -> Result<Option<Chunk>> {
        self.stream.set_int_encoding(IntEncoding::Raw);
        self.stream.seek(self.chunk_start_position)?;
        match self.stream.read_u8() {
            Ok(magic_head) => {
                let mut magic = [magic_head, 0, 0, 0];
                let mut magic_tail: [u8; 3] = self.stream.read_exact()?;
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
            major: self.stream.read_i16()?,
            minor: self.stream.read_i16()?,
        };
        match version {
            VERSION_1 | VERSION_2 => {}
            _ => {
                return Err(Error::UnsupportedVersion(version));
            }
        }

        self.read_chunk().map(Some)
    }

    fn read_chunk(&mut self) -> Result<Chunk> {
        let header = self.read_chunk_header()?;

        self.stream.set_int_encoding(header.int_encoding());

        let metadata = Metadata::try_new(&mut self.stream, &header)?;
        let constant_pool = ConstantPool::try_new(&mut self.stream, &header, &metadata)?;

        // update to next chunk start
        self.chunk_start_position += header.chunk_size as u64;

        Ok(Chunk {
            header,
            metadata,
            constant_pool,
        })
    }

    fn read_chunk_header(&mut self) -> Result<ChunkHeader> {
        Ok(ChunkHeader {
            chunk_size: self.stream.read_i64()?,
            constant_pool_offset: self.stream.read_i64()?,
            metadata_offset: self.stream.read_i64()?,
            start_time_nanos: self.stream.read_i64()?,
            duration_nanos: self.stream.read_i64()?,
            start_ticks: self.stream.read_i64()?,
            ticks_per_second: self.stream.read_i64()?,
            features: self.stream.read_i32()?,
            absolute_chunk_start_position: self.chunk_start_position,
        })
    }
}

pub struct EventIterator<'a, T> {
    chunk: Chunk,
    stream: &'a mut ByteStream<T>,
}

impl<'a, T> Iterator for EventIterator<'a, T>
where
    T: Read + Seek,
{
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Cursor;
    use std::path::PathBuf;

    #[test]
    fn test_read_chunk() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/profiler-wall.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());

        while let Some(chunk) = reader.next() {
            let chunk = chunk.unwrap();
            // println!("header: {:#?}", chunk.header);
            // println!("metadata: {:#?}", chunk.metadata);
            // println!("constant pool: {:#?}", chunk.constant_pool);
            // for event in reader.events(chunk.unwrap()) {}
        }
    }
}

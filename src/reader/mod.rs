//! Module to read JFR files and parse as Rust data structures.

use std::arch::aarch64::vreinterpret_u8_f64;
use std::io;
use crate::{MAGIC, Version, VERSION_1, VERSION_2};
use byteorder::{BE, ReadBytesExt};
use std::io::{BufReader, Read, Seek};
use std::marker::PhantomData;
use crate::reader::byte_stream::{ByteStream, IntEncoding};

mod v1;
mod byte_stream;
mod metadata;
mod constant_pool;
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
    body_start_position: u64,
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
}

#[derive(Debug)]
pub struct Chunk {
    // constant_pool: ConstantPool
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
        Self { stream: ByteStream::new(inner), chunk_start_position: 0 }
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

        // update to next chunk start
        self.chunk_start_position += header.chunk_size as u64;
        self.stream.set_int_encoding(header.int_encoding());

        Err(Error::InvalidFormat)
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
            body_start_position: self.chunk_start_position + ChunkHeader::HEADER_SIZE,
        })
    }
}

pub struct EventIterator<'a, T> {
    chunk: Chunk,
    stream: &'a mut ByteStream<T>,
}

impl<'a, T> Iterator for EventIterator<'a, T> where T: Read + Seek {
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
            for event in reader.events(chunk.unwrap()) {
            }
        }

    }
}

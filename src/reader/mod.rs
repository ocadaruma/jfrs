use crate::{Version, MAGIC};
use byteorder::{ReadBytesExt, BE};
use std::io::{Read, Seek};

mod v1;

#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    InvalidString,
    UnsupportedVersion(Version),
    IoError(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub enum ChunkVersion {
    V1(v1::Chunk),
}

pub struct JfrReader<R> {
    inner: R,
}

impl<R> JfrReader<R>
where
    R: Read + Seek,
{
    pub fn read_chunk(&mut self) -> Result<v1::Chunk> {
        let mut magic = [0u8; 4];
        self.inner.read_exact(&mut magic).map_err(Error::IoError)?;

        if magic != MAGIC {
            return Err(Error::InvalidFormat);
        }

        let version = Version {
            major: self.inner.read_i16::<BE>().map_err(Error::IoError)?,
            minor: self.inner.read_i16::<BE>().map_err(Error::IoError)?,
        };

        match version {
            crate::VERSION_1 | crate::VERSION_2 => v1::ChunkReader::wrap(&mut self.inner).read(),
            _ => Err(Error::UnsupportedVersion(version)),
        }
    }

    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn test_read_chunk() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/recording.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());

        assert!(reader.read_chunk().is_err());
    }
}

//! This crate provides Rust interfaces to manipulate JFR (Java Flight Recorder) files.

pub mod reader;

const MAGIC: [u8; 4] = [b'F', b'L', b'R', b'\0'];
const VERSION_1: Version = Version { major: 1, minor: 0 };
const VERSION_2: Version = Version { major: 2, minor: 0 };

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Version {
    major: i16,
    minor: i16,
}

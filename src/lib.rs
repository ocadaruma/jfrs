//! This crate provides Rust interfaces to manipulate JFR (Java Flight Recorder) files.

use std::fmt;
use std::fmt::Formatter;

pub mod reader;

const MAGIC: [u8; 4] = [b'F', b'L', b'R', b'\0'];

const EVENT_TYPE_METADATA: i64 = 0;
const EVENT_TYPE_CONSTANT_POOL: i64 = 1;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Version {
    major: i16,
    minor: i16,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

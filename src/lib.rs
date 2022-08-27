pub mod reader;

const MAGIC: [u8; 4] = [b'F', b'L', b'R', b'\0'];
const VERSION_1: Version = Version { major: 1, minor: 0 };
const VERSION_2: Version = Version { major: 2, minor: 0 };

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Version {
    pub major: i16,
    pub minor: i16,
}

use crate::reader::v1::byte_reader::{ByteReader, StringType};
use crate::reader::{Error, Result};
use std::io::Read;

const EVENT_TYPE_METADATA: i64 = 0;

struct RootElement {
    metadata: Option<MetadataElement>,
    region: Option<RegionElement>,
}

enum ChildElement {
    Metadata(MetadataElement),
    Region(RegionElement),
    Class(ClassElement),
    Field(FieldElement),
    Annotation(AnnotationElement),
}

struct MetadataElement {
    classes: Vec<ClassElement>,
}

struct RegionElement {}

struct ClassElement {
    annotations: Vec<AnnotationElement>,
}

struct FieldElement {
    annotations: Vec<AnnotationElement>,
}

struct AnnotationElement {}

#[derive(Debug)]
pub struct Metadata {
    string_table: Vec<Option<String>>,
}

pub struct MetadataReader<'a, R>(&'a mut R);

impl<'a, R> MetadataReader<'a, R>
where
    R: Read,
{
    pub fn wrap(inner: &'a mut R) -> Self {
        Self(inner)
    }

    pub fn read_metadata(&mut self, reader: &ByteReader) -> Result<Metadata> {
        // size
        reader.read_i32(self.0)?;
        if reader.read_i64(self.0)? != EVENT_TYPE_METADATA {
            return Err(Error::InvalidFormat);
        }

        // start time
        reader.read_i64(self.0)?;
        // duration
        reader.read_i64(self.0)?;
        // metadata id
        reader.read_i64(self.0)?;

        let string_count = reader.read_i32(self.0)?;
        let mut string_table = Vec::with_capacity(string_count as usize);

        for _ in 0..string_count {
            match reader.read_string(self.0)? {
                StringType::Null => string_table.push(None),
                StringType::Empty => string_table.push(Some("".to_string())),
                StringType::Raw(s) => string_table.push(Some(s)),
                _ => return Err(Error::InvalidString),
            }
        }

        Ok(Metadata { string_table })
    }
}

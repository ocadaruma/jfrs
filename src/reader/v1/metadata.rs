use crate::reader::v1::byte_reader::{ByteReader, StringType};
use crate::reader::{Error, Result};
use std::collections::HashMap;
use std::io::Read;

const EVENT_TYPE_METADATA: i64 = 0;

#[derive(Debug)]
enum ElementType<'st> {
    Root(RootElement<'st>),
    Metadata(MetadataElement<'st>),
    Region(RegionElement),
    Class(ClassElement<'st>),
    Field(FieldElement<'st>),
    Annotation(AnnotationElement<'st>),
    Setting(SettingElement<'st>),
}

impl<'st> ElementType<'st> {
    fn try_new(name: &String) -> Result<Self> {
        match name.as_str() {
            "metadata" => Ok(ElementType::Metadata(MetadataElement::default())),
            "region" => Ok(ElementType::Region(RegionElement::default())),
            "class" => Ok(ElementType::Class(ClassElement::default())),
            "field" => Ok(ElementType::Field(FieldElement::default())),
            "setting" => Ok(ElementType::Setting(SettingElement::default())),
            "annotation" => Ok(ElementType::Annotation(AnnotationElement::default())),
            _ => Err(Error::InvalidFormat),
        }
    }

    fn append_child(&mut self, child: ElementType<'st>) {
        match self {
            ElementType::Root(e) => match child {
                ElementType::Metadata(m) => e.metadata = Some(m),
                ElementType::Region(r) => e.region = Some(r),
                _ => {}
            },
            ElementType::Metadata(e) => match child {
                ElementType::Class(c) => e.classes.push(c),
                _ => {}
            },
            ElementType::Class(e) => match child {
                ElementType::Field(f) => e.fields.push(f),
                ElementType::Annotation(a) => e.annotations.push(a),
                ElementType::Setting(s) => e.setting = Some(s),
                _ => {}
            },
            ElementType::Field(e) => match child {
                ElementType::Annotation(a) => e.annotations.push(a),
                _ => {}
            },
            ElementType::Setting(e) => match child {
                ElementType::Annotation(a) => e.annotations.push(a),
                _ => {}
            },
            _ => {}
        }
    }

    fn set_attribute(&mut self, key: &'st String, value: &'st String) -> Result<()> {
        match self {
            ElementType::Class(c) => match key.as_str() {
                "id" => c.class_id = value.parse().map_err(|_| Error::InvalidFormat)?,
                "name" => c.type_identifier = Some(value),
                "superType" => c.super_type = Some(value),
                "simpleType" => {
                    c.simple_type = Some(value.parse().map_err(|_| Error::InvalidFormat)?)
                }
                _ => {}
            },
            ElementType::Field(f) => match key.as_str() {
                "name" => f.field_identifier = Some(value),
                "class" => f.class_id = value.parse().map_err(|_| Error::InvalidFormat)?,
                "constantPool" => {
                    f.constant_pool = Some(value.parse().map_err(|_| Error::InvalidFormat)?)
                }
                "dimension" => f.dimension = Some(value.parse().map_err(|_| Error::InvalidFormat)?),
                _ => {}
            },
            ElementType::Annotation(a) => match key.as_str() {
                "class" => a.class_id = value.parse().map_err(|_| Error::InvalidFormat)?,
                _ => {
                    a.attributes.insert(key, value);
                }
            },
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RootElement<'st> {
    metadata: Option<MetadataElement<'st>>,
    region: Option<RegionElement>,
}

#[derive(Debug, Default)]
struct MetadataElement<'st> {
    classes: Vec<ClassElement<'st>>,
}

#[derive(Debug, Default)]
struct RegionElement {}

#[derive(Debug, Default)]
struct ClassElement<'st> {
    annotations: Vec<AnnotationElement<'st>>,
    fields: Vec<FieldElement<'st>>,
    setting: Option<SettingElement<'st>>,
    class_id: i64,
    type_identifier: Option<&'st str>,
    super_type: Option<&'st str>,
    simple_type: Option<bool>,
}

#[derive(Debug, Default)]
struct FieldElement<'st> {
    annotations: Vec<AnnotationElement<'st>>,
    field_identifier: Option<&'st String>,
    class_id: i64,
    constant_pool: Option<bool>,
    dimension: Option<i32>,
}

#[derive(Debug, Default)]
struct AnnotationElement<'st> {
    class_id: i64,
    attributes: HashMap<&'st String, &'st String>,
}

#[derive(Debug, Default)]
struct SettingElement<'st> {
    annotations: Vec<AnnotationElement<'st>>,
}

#[derive(Debug)]
pub struct Metadata {
    string_table: StringTable,
}

#[derive(Debug)]
pub struct StringTable(Vec<Option<String>>);

impl StringTable {
    pub fn get(&self, idx: i32) -> Result<&String> {
        self.0
            .get(idx as usize)
            .map(|s| s.as_ref())
            .flatten()
            .ok_or(Error::InvalidFormat)
    }
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
        let mut strings = Vec::with_capacity(string_count as usize);

        for _ in 0..string_count {
            match reader.read_string(self.0)? {
                StringType::Null => strings.push(None),
                StringType::Empty => strings.push(Some("".to_string())),
                StringType::Raw(s) => strings.push(Some(s)),
                _ => return Err(Error::InvalidString),
            }
        }

        let string_table = StringTable(strings);

        // we don't care root element name
        reader.read_i32(self.0)?;
        let root_element = self.read_element(
            reader,
            &string_table,
            ElementType::Root(RootElement {
                metadata: None,
                region: None,
            }),
        )?;

        println!("root: {:?}", root_element);
        Ok(Metadata { string_table })
    }

    fn read_element<'st>(
        &mut self,
        reader: &ByteReader,
        string_table: &'st StringTable,
        mut current_element: ElementType<'st>,
    ) -> Result<ElementType<'st>> {
        let attribute_count = reader.read_i32(self.0)?;
        for _ in 0..attribute_count {
            let key = string_table.get(reader.read_i32(self.0)?)?;
            let value = string_table.get(reader.read_i32(self.0)?)?;
            current_element.set_attribute(key, value)?;
        }

        let children_count = reader.read_i32(self.0)?;
        for _ in 0..children_count {
            let name = string_table.get(reader.read_i32(self.0)?)?;
            let element = ElementType::try_new(name)?;
            current_element.append_child(self.read_element(reader, string_table, element)?);
        }

        Ok(current_element)
    }
}

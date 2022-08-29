use crate::reader::v1::byte_reader::{ByteReader, StringType};
use crate::reader::v1::type_descriptor::{
    FieldDescriptor, TickUnit, TypeDescriptor, TypePool, Unit,
};
use crate::reader::{Error, Result};
use std::collections::HashMap;
use std::io::Read;
use std::process::id;

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
    fn try_new(name: &str) -> Result<Self> {
        match name {
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

    fn set_attribute(&mut self, key: &'st str, value: &'st str) -> Result<()> {
        match self {
            ElementType::Class(c) => match key {
                "id" => c.class_id = value.parse().map_err(|_| Error::InvalidFormat)?,
                "name" => c.type_identifier = Some(value),
                "superType" => c.super_type = Some(value),
                "simpleType" => {
                    c.simple_type = Some(value.parse().map_err(|_| Error::InvalidFormat)?)
                }
                _ => {}
            },
            ElementType::Field(f) => match key {
                "name" => f.field_identifier = Some(value),
                "class" => f.class_id = value.parse().map_err(|_| Error::InvalidFormat)?,
                "constantPool" => {
                    f.constant_pool = Some(value.parse().map_err(|_| Error::InvalidFormat)?)
                }
                "dimension" => f.dimension = Some(value.parse().map_err(|_| Error::InvalidFormat)?),
                _ => {}
            },
            ElementType::Annotation(a) => match key {
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
    field_identifier: Option<&'st str>,
    class_id: i64,
    constant_pool: Option<bool>,
    dimension: Option<i32>,
}

#[derive(Debug, Default)]
struct AnnotationElement<'st> {
    class_id: i64,
    attributes: HashMap<&'st str, &'st str>,
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
    pub fn get(&self, idx: i32) -> Result<&str> {
        self.0
            .get(idx as usize)
            .and_then(|s| s.as_ref())
            .ok_or(Error::InvalidFormat)
            .map(|s| s.as_str())
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

        let mut class_name_map = HashMap::new();
        // we don't care root element name
        reader.read_i32(self.0)?;
        let root_element = self.read_element(
            reader,
            &string_table,
            &mut class_name_map,
            ElementType::Root(RootElement::default()),
        )?;

        // println!("root: {:?}", root_element);
        let type_pool = if let ElementType::Root(root) = root_element {
            self.declare_types(root, class_name_map)?
        } else {
            return Err(Error::InvalidFormat);
        };
        println!("type_pool: {:#?}", type_pool);
        Ok(Metadata { string_table })
    }

    fn read_element<'st>(
        &mut self,
        reader: &ByteReader,
        string_table: &'st StringTable,
        class_name_map: &mut HashMap<i64, &'st str>,
        mut current_element: ElementType<'st>,
    ) -> Result<ElementType<'st>> {
        let attribute_count = reader.read_i32(self.0)?;
        for _ in 0..attribute_count {
            let key = string_table.get(reader.read_i32(self.0)?)?;
            let value = string_table.get(reader.read_i32(self.0)?)?;
            current_element.set_attribute(key, value)?;
        }

        // at this point, class names should be resolved from attributes
        if let ElementType::Class(ref c) = current_element {
            if let Some(name) = c.type_identifier {
                class_name_map.insert(c.class_id, name);
            }
        }

        let children_count = reader.read_i32(self.0)?;
        for _ in 0..children_count {
            let name = string_table.get(reader.read_i32(self.0)?)?;
            let element = ElementType::try_new(name)?;
            current_element.append_child(self.read_element(
                reader,
                string_table,
                class_name_map,
                element,
            )?);
        }

        Ok(current_element)
    }

    fn declare_types<'st>(
        &self,
        root_element: RootElement<'st>,
        class_name_map: HashMap<i64, &'st str>,
    ) -> Result<TypePool<'st>> {
        let mut pool = TypePool::default();
        if let Some(classes) = root_element.metadata.map(|m| m.classes) {
            for class_element in classes {
                let mut desc = TypeDescriptor {
                    class_id: class_element.class_id,
                    name: class_element.type_identifier.ok_or(Error::InvalidFormat)?,
                    super_type: class_element.super_type,
                    simple_type: class_element.simple_type.unwrap_or(false),
                    fields: vec![],
                    label: None,
                    description: None,
                    experimental: false,
                    category: vec![],
                };

                for annot in class_element.annotations {
                    if let Some(&name) = class_name_map.get(&annot.class_id) {
                        match name {
                            "jdk.jfr.Label" => desc.label = annot.attributes.get("value").copied(),
                            "jdk.jfr.Description" => {
                                desc.description = annot.attributes.get("value").copied()
                            }
                            "jdk.jfr.Experimental" => desc.experimental = true,
                            "jdk.jfr.Category" => {
                                let mut idx = 0;
                                loop {
                                    if let Some(&v) =
                                        annot.attributes.get(format!("value-{}", id()).as_str())
                                    {
                                        desc.category.push(v);
                                    } else {
                                        break;
                                    }
                                    idx += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                for field in class_element.fields {
                    let mut field_desc = FieldDescriptor {
                        class_id: field.class_id,
                        name: field.field_identifier.ok_or(Error::InvalidFormat)?,
                        label: None,
                        description: None,
                        experimental: false,
                        constant_pool: field.constant_pool.unwrap_or(false),
                        array_type: field.dimension.unwrap_or(0) > 0,
                        unsigned: false,
                        unit: None,
                        tick_unit: None,
                    };

                    for annot in field.annotations {
                        if let Some(&name) = class_name_map.get(&annot.class_id) {
                            match name {
                                "jdk.jfr.Label" => {
                                    field_desc.label = annot.attributes.get("value").copied()
                                }
                                "jdk.jfr.Description" => {
                                    field_desc.description = annot.attributes.get("value").copied()
                                }
                                "jdk.jfr.Experimental" => field_desc.experimental = true,
                                "jdk.jfr.Unsigned" => field_desc.unsigned = true,
                                "jdk.jfr.MemoryAmount" | "jdk.jfr.DataAmount" => {
                                    field_desc.unit = Some(Unit::Byte)
                                }
                                "jdk.jfr.Percentage" => field_desc.unit = Some(Unit::PercentUnity),
                                "jdk.jfr.MemoryAddress" => {
                                    field_desc.unit = Some(Unit::AddressUnity)
                                }
                                "jdk.jfr.Timespan" => {
                                    if let Some(&v) = annot.attributes.get("value") {
                                        match v {
                                            "TICKS" => {
                                                field_desc.tick_unit = Some(TickUnit::Timespan)
                                            }
                                            "NANOSECONDS" => {
                                                field_desc.unit = Some(Unit::Nanosecond)
                                            }
                                            "MILLISECONDS" => {
                                                field_desc.unit = Some(Unit::Millisecond)
                                            }
                                            "SECONDS" => field_desc.unit = Some(Unit::Second),
                                            _ => {}
                                        }
                                    }
                                }
                                "jdk.jfr.Frequency" => field_desc.unit = Some(Unit::Hz),
                                "jdk.jfr.Timestamp" => {
                                    if let Some(&v) = annot.attributes.get("value") {
                                        match v {
                                            "TICKS" => {
                                                field_desc.tick_unit = Some(TickUnit::Timestamp)
                                            }
                                            "NANOSECONDS_SINCE_EPOCH" => {
                                                field_desc.unit = Some(Unit::EpochNano)
                                            }
                                            "MILLISECONDS_SINCE_EPOCH" => {
                                                field_desc.unit = Some(Unit::EpochMilli)
                                            }
                                            "SECONDS_SINCE_EPOCH" => {
                                                field_desc.unit = Some(Unit::EpochSecond)
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    desc.fields.push(field_desc);
                }

                pool.register(class_element.class_id, desc);
            }
        }

        Ok(pool)
    }
}

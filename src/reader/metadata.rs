//! Read JFR Metadata event.
//! Metadata event contains the type definitions to parse further constant pools and recorded events.
//!
//! Related JMC code: [ChunkMetadata.java](https://github.com/openjdk/jmc/blob/8.2.0-ga/core/org.openjdk.jmc.flightrecorder/src/main/java/org/openjdk/jmc/flightrecorder/internal/parser/v1/ChunkMetadata.java)

use crate::reader::byte_stream::ByteStream;
use crate::reader::type_descriptor::{
    FieldDescriptor, StringTable, TickUnit, TypeDescriptor, TypePool, Unit,
};
use crate::reader::{ChunkHeader, Error, Result};
use crate::EVENT_TYPE_METADATA;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::rc::Rc;

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

    fn append_child(&mut self, child: Self) {
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

    fn set_attribute(&mut self, key: &'st str, value: &'st Rc<str>) -> Result<()> {
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
                    a.attributes.insert(key, value.clone());
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
    type_identifier: Option<&'st Rc<str>>,
    super_type: Option<&'st Rc<str>>,
    simple_type: Option<bool>,
}

#[derive(Debug, Default)]
struct FieldElement<'st> {
    annotations: Vec<AnnotationElement<'st>>,
    field_identifier: Option<&'st Rc<str>>,
    class_id: i64,
    constant_pool: Option<bool>,
    dimension: Option<i32>,
}

#[derive(Debug, Default)]
struct AnnotationElement<'st> {
    class_id: i64,
    attributes: HashMap<&'st str, Rc<str>>,
}

#[derive(Debug, Default)]
struct SettingElement<'st> {
    annotations: Vec<AnnotationElement<'st>>,
}

#[derive(Debug)]
pub struct Metadata {
    pub type_pool: TypePool,
}

impl Metadata {
    pub fn try_new<T: Read + Seek>(
        stream: &mut ByteStream<T>,
        header: &ChunkHeader,
    ) -> Result<Self> {
        stream.seek(header.metadata_offset as u64)?;

        // size
        stream.read_i32()?;
        if stream.read_i64()? != EVENT_TYPE_METADATA {
            return Err(Error::InvalidFormat);
        }
        // start time
        stream.read_i64()?;
        // duration
        stream.read_i64()?;
        // metadata id
        stream.read_i64()?;

        let string_table = StringTable::try_new(stream)?;
        let type_pool = Self::read_types(stream, &string_table)?;

        Ok(Self { type_pool })
    }

    fn read_types<T: Read>(
        stream: &mut ByteStream<T>,
        string_table: &StringTable,
    ) -> Result<TypePool> {
        let mut class_name_map = HashMap::new();

        // we don't care root element name. just consume
        stream.read_i32()?;

        let root_element = Self::read_element(
            stream,
            string_table,
            &mut class_name_map,
            ElementType::Root(RootElement::default()),
        )?;

        let type_pool = if let ElementType::Root(root) = root_element {
            Self::declare_types(root, class_name_map)?
        } else {
            return Err(Error::InvalidFormat);
        };

        Ok(type_pool)
    }

    fn read_element<'st, T: Read>(
        stream: &mut ByteStream<T>,
        string_table: &'st StringTable,
        class_name_map: &mut HashMap<i64, &'st str>,
        mut current_element: ElementType<'st>,
    ) -> Result<ElementType<'st>> {
        let attribute_count = stream.read_i32()?;
        for _ in 0..attribute_count {
            let key = string_table.get(stream.read_i32()?)?;
            let value = string_table.get(stream.read_i32()?)?;
            current_element.set_attribute(key, value)?;
        }

        // at this point, class name is already resolved from attributes
        if let ElementType::Class(c) = &current_element {
            if let Some(name) = c.type_identifier {
                class_name_map.insert(c.class_id, name.as_ref());
            }
        }

        let children_count = stream.read_i32()?;
        for _ in 0..children_count {
            let name = string_table.get(stream.read_i32()?)?;
            let element = ElementType::try_new(name.as_ref())?;
            current_element.append_child(Self::read_element(
                stream,
                string_table,
                class_name_map,
                element,
            )?);
        }

        Ok(current_element)
    }

    fn declare_types(
        root_element: RootElement,
        class_name_map: HashMap<i64, &str>,
    ) -> Result<TypePool> {
        let mut pool = TypePool::default();
        let classes = match root_element.metadata {
            Some(m) => m.classes,
            None => return Ok(pool),
        };

        for class_element in classes {
            let mut desc = TypeDescriptor {
                class_id: class_element.class_id,
                name: class_element
                    .type_identifier
                    .cloned()
                    .ok_or(Error::InvalidFormat)?,
                super_type: class_element.super_type.cloned(),
                simple_type: class_element.simple_type.unwrap_or(false),
                fields: Vec::with_capacity(class_element.fields.len()),
                label: None,
                description: None,
                experimental: false,
                category: vec![],
            };

            for annot in class_element.annotations {
                Self::resolve_class_annotation(&mut desc, &annot, &class_name_map)?;
            }

            for field in class_element.fields {
                let mut field_desc = FieldDescriptor {
                    class_id: field.class_id,
                    name: field
                        .field_identifier
                        .cloned()
                        .ok_or(Error::InvalidFormat)?,
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
                    Self::resolve_field_annotation(&mut field_desc, &annot, &class_name_map)?;
                }
                desc.fields.push(field_desc);
            }

            pool.register(class_element.class_id, desc);
        }

        Ok(pool)
    }

    fn resolve_class_annotation(
        desc: &mut TypeDescriptor,
        annot: &AnnotationElement,
        class_name_map: &HashMap<i64, &str>,
    ) -> Result<()> {
        if let Some(&name) = class_name_map.get(&annot.class_id) {
            match name {
                "jdk.jfr.Label" => desc.label = annot.attributes.get("value").cloned(),
                "jdk.jfr.Description" => desc.description = annot.attributes.get("value").cloned(),
                "jdk.jfr.Experimental" => desc.experimental = true,
                "jdk.jfr.Category" => {
                    let mut idx = 0;
                    loop {
                        if let Some(v) = annot
                            .attributes
                            .get(format!("value-{}", idx).as_str())
                            .cloned()
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
        Ok(())
    }

    fn resolve_field_annotation(
        desc: &mut FieldDescriptor,
        annot: &AnnotationElement,
        class_name_map: &HashMap<i64, &str>,
    ) -> Result<()> {
        if let Some(&name) = class_name_map.get(&annot.class_id) {
            match name {
                "jdk.jfr.Label" => desc.label = annot.attributes.get("value").cloned(),
                "jdk.jfr.Description" => desc.description = annot.attributes.get("value").cloned(),
                "jdk.jfr.Experimental" => desc.experimental = true,
                "jdk.jfr.Unsigned" => desc.unsigned = true,
                "jdk.jfr.MemoryAmount" | "jdk.jfr.DataAmount" => desc.unit = Some(Unit::Byte),
                "jdk.jfr.Percentage" => desc.unit = Some(Unit::PercentUnity),
                "jdk.jfr.MemoryAddress" => desc.unit = Some(Unit::AddressUnity),
                "jdk.jfr.Timespan" => {
                    if let Some(v) = annot.attributes.get("value") {
                        match v.as_ref() {
                            "TICKS" => desc.tick_unit = Some(TickUnit::Timespan),
                            "NANOSECONDS" => desc.unit = Some(Unit::Nanosecond),
                            "MILLISECONDS" => desc.unit = Some(Unit::Millisecond),
                            "SECONDS" => desc.unit = Some(Unit::Second),
                            _ => {}
                        }
                    }
                }
                "jdk.jfr.Frequency" => desc.unit = Some(Unit::Hz),
                "jdk.jfr.Timestamp" => {
                    if let Some(v) = annot.attributes.get("value") {
                        match v.as_ref() {
                            "TICKS" => desc.tick_unit = Some(TickUnit::Timestamp),
                            "NANOSECONDS_SINCE_EPOCH" => desc.unit = Some(Unit::EpochNano),
                            "MILLISECONDS_SINCE_EPOCH" => desc.unit = Some(Unit::EpochMilli),
                            "SECONDS_SINCE_EPOCH" => desc.unit = Some(Unit::EpochSecond),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_intern() {
        let class1_name = Rc::from("Class1");
        let class2_name = Rc::from("Class2");
        let class3_name = Rc::from("Class3");
        let field_name = Rc::from("fieldWithTypeOfClass1");

        let class1 = class(1, &class1_name, vec![]);
        let class2 = class(2, &class2_name, vec![field(1, &field_name)]);
        let class3 = class(3, &class3_name, vec![field(1, &field_name)]);

        let mut meta = MetadataElement::default();
        meta.classes = vec![class1, class2, class3];

        let mut root = RootElement::default();
        root.metadata = Some(meta);

        let class_name_map = HashMap::from([
            (1i64, class1_name.as_ref()),
            (2, class2_name.as_ref()),
            (3, class3_name.as_ref()),
        ]);

        let type_pool = Metadata::declare_types(root, class_name_map).unwrap();

        let desc2 = type_pool.get(2).unwrap();
        let desc3 = type_pool.get(3).unwrap();

        assert!(Rc::ptr_eq(&desc2.fields[0].name, &desc3.fields[0].name));
    }

    fn class<'a>(
        class_id: i64,
        name: &'a Rc<str>,
        fields: Vec<FieldElement<'a>>,
    ) -> ClassElement<'a> {
        let mut element = ClassElement::default();
        element.class_id = class_id;
        element.type_identifier = Some(name);
        element.fields = fields;
        element
    }

    fn field(class_id: i64, name: &Rc<str>) -> FieldElement {
        let mut element = FieldElement::default();
        element.class_id = class_id;
        element.field_identifier = Some(name);
        element
    }
}

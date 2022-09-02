//! Low-level representation of the decoded JFR values.

use crate::reader::byte_stream::{ByteStream, StringType};
use crate::reader::metadata::Metadata;

use crate::reader::{Error, Result};
use std::io::{Read, Seek};
use crate::reader::type_descriptor::{FieldDescriptor, TypeDescriptor};

#[derive(Debug)]
pub enum ValueDescriptor {
    Primitive(Primitive),
    Object(Object),
    Array(Vec<ValueDescriptor>),
    ConstantPool { class_id: i64, constant_index: i64 },
}

impl ValueDescriptor {
    pub fn try_new<T: Read>(
        stream: &mut ByteStream<T>,
        class_id: i64,
        metadata: &Metadata,
    ) -> Result<ValueDescriptor> {
        let type_desc = metadata
            .type_pool
            .get(class_id)
            .ok_or(Error::ClassNotFound(class_id))?;

        if let Some(value) = Self::try_read_primitive(stream, type_desc)? {
            return Ok(value);
        }

        let mut obj = Object {
            class_id: type_desc.class_id,
            fields: vec![],
        };

        for field_desc in type_desc.fields.iter() {
            let value = if field_desc.array_type {
                let mut elems = vec![];
                let count = stream.read_i32()?;
                for _ in 0..count {
                    elems.push(Self::try_read_field_single(stream, field_desc, metadata)?);
                }
                ValueDescriptor::Array(elems)
            } else {
                Self::try_read_field_single(stream, field_desc, metadata)?
            };
            obj.fields.push(value);
        }

        Ok(ValueDescriptor::Object(obj))
    }

    fn try_read_field_single<T: Read>(
        stream: &mut ByteStream<T>,
        field_desc: &FieldDescriptor,
        metadata: &Metadata
    ) -> Result<ValueDescriptor> {
        if field_desc.constant_pool {
            Ok(ValueDescriptor::ConstantPool {
                class_id: field_desc.class_id,
                constant_index: stream.read_i64()?
            })
        } else {
            Self::try_new(stream, field_desc.class_id, metadata)
        }
    }

    fn try_read_primitive<T: Read>(
        stream: &mut ByteStream<T>,
        type_desc: &TypeDescriptor,
    ) -> Result<Option<ValueDescriptor>> {
        let value = match type_desc.name.as_ref() {
            "int" => Some(ValueDescriptor::Primitive(Primitive::Integer(
                stream.read_i32()?,
            ))),
            "long" => Some(ValueDescriptor::Primitive(Primitive::Long(
                stream.read_i64()?,
            ))),
            "float" => Some(ValueDescriptor::Primitive(Primitive::Float(
                stream.read_f32()?,
            ))),
            "double" => Some(ValueDescriptor::Primitive(Primitive::Double(
                stream.read_f64()?,
            ))),
            "char" => Some(ValueDescriptor::Primitive(Primitive::Character(
                stream.read_char()?,
            ))),
            "boolean" => Some(ValueDescriptor::Primitive(Primitive::Boolean(
                stream.read_i8()? != 0,
            ))),
            "short" => Some(ValueDescriptor::Primitive(Primitive::Short(
                stream.read_i16()?,
            ))),
            "byte" => Some(ValueDescriptor::Primitive(Primitive::Byte(
                stream.read_i8()?,
            ))),
            "java.lang.String" => match stream.read_string()? {
                StringType::Null => Some(ValueDescriptor::Primitive(Primitive::NullString)),
                StringType::Empty => Some(ValueDescriptor::Primitive(Primitive::String(
                    "".to_string(),
                ))),
                StringType::Raw(s) => Some(ValueDescriptor::Primitive(Primitive::String(s))),
                StringType::ConstantPool(idx) => Some(ValueDescriptor::ConstantPool {
                    class_id: type_desc.class_id,
                    constant_index: idx,
                }),
            },
            _ => None,
        };
        Ok(value)
    }
}

#[derive(Debug)]
pub struct Object {
    pub class_id: i64,
    pub fields: Vec<ValueDescriptor>,
}

#[derive(Debug)]
pub enum Primitive {
    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Character(char),
    Boolean(bool),
    Short(i16),
    Byte(i8),
    NullString,
    String(String),
}

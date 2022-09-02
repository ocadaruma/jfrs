//! Low-level representation of the decoded JFR values.

use crate::reader::type_descriptor::TypePool;
use crate::reader::{Error, Result};
use std::io::{Read, Seek};
use crate::reader::byte_stream::{ByteStream, StringType};

#[derive(Debug)]
pub enum ValueDescriptor {
    Primitive(Primitive),
    Object(Object),
    Array(Vec<ValueDescriptor>),
    ConstantPool(i64, i64),
}

impl ValueDescriptor {
    pub fn try_new<T: Read + Seek>(stream: &mut ByteStream<T>, class_id: i64, type_pool: &TypePool<'_>) -> Result<ValueDescriptor> {
        let type_desc = type_pool.get(class_id).ok_or(Error::InvalidFormat)?;

        match type_desc.name {
            "int" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Integer(
                    stream.read_i32()?,
                )))
            }
            "long" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Long(
                    stream.read_i64()?,
                )))
            }
            "float" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Float(
                    stream.read_f32()?,
                )))
            }
            "double" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Double(
                    stream.read_f64()?,
                )))
            }
            "char" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Float(
                    stream.read_f32()?,
                )))
            }
            "boolean" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Boolean(
                    stream.read_i8()? != 0,
                )))
            }
            "short" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Short(
                    stream.read_i16()?,
                )))
            }
            "byte" => {
                return Ok(ValueDescriptor::Primitive(Primitive::Byte(
                    stream.read_i8()?,
                )))
            }
            "java.lang.String" => {
                return match stream.read_string()? {
                    StringType::Null => Ok(ValueDescriptor::Primitive(Primitive::NullString)),
                    StringType::Empty => Ok(ValueDescriptor::Primitive(Primitive::String(
                        "".to_string(),
                    ))),
                    StringType::Raw(s) => Ok(ValueDescriptor::Primitive(Primitive::String(s))),
                    StringType::ConstantPool(idx) => {
                        Ok(ValueDescriptor::ConstantPool(type_desc.class_id, idx))
                    }
                };
            }
            _ => {}
        }

        let mut obj = Object {
            class_id: type_desc.class_id,
            fields: vec![],
        };
        for field_desc in type_desc.fields.iter() {
            if field_desc.array_type {
                let mut elems = vec![];
                let count = stream.read_i32()?;
                for _ in 0..count {
                    if field_desc.constant_pool {
                        elems.push(ValueDescriptor::ConstantPool(
                            field_desc.class_id,
                            stream.read_i64()?,
                        ));
                    } else {
                        elems.push(Self::try_new(stream, field_desc.class_id, type_pool)?);
                    }
                }
                obj.fields.push(ValueDescriptor::Array(elems));
            } else {
                if field_desc.constant_pool {
                    obj.fields.push(ValueDescriptor::ConstantPool(
                        field_desc.class_id,
                        stream.read_i64()?,
                    ));
                } else {
                    obj.fields
                        .push(Self::try_new(stream, field_desc.class_id, type_pool)?);
                }
            }
        }

        Ok(ValueDescriptor::Object(obj))
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

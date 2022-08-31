//! Low-level representation of the decoded values.

use crate::reader::v1::byte_reader::{ByteReader, StringType};
use crate::reader::v1::constant_pool::ConstantPool;
use crate::reader::v1::type_descriptor::TypePool;
use crate::reader::{Error, Result};
use std::io::Read;
use std::process::id;

#[derive(Debug)]
pub enum ValueDescriptor {
    Primitive(Primitive),
    Object(Object),
    Array(Vec<ValueDescriptor>),
    ConstantPool(i64, i64),
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

pub fn read_value<R>(
    r: &mut R,
    reader: &ByteReader,
    class_id: i64,
    type_pool: &TypePool<'_>,
) -> Result<ValueDescriptor>
where
    R: Read,
{
    let type_desc = type_pool.get(class_id).ok_or(Error::InvalidFormat)?;

    match type_desc.name {
        "int" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Integer(
                reader.read_i32(r)?,
            )))
        }
        "long" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Long(
                reader.read_i64(r)?,
            )))
        }
        "float" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Float(
                reader.read_f32(r)?,
            )))
        }
        "double" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Double(
                reader.read_f64(r)?,
            )))
        }
        "char" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Float(
                reader.read_f32(r)?,
            )))
        }
        "boolean" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Boolean(
                reader.read_i8(r)? != 0,
            )))
        }
        "short" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Short(
                reader.read_i16(r)?,
            )))
        }
        "byte" => {
            return Ok(ValueDescriptor::Primitive(Primitive::Byte(
                reader.read_i8(r)?,
            )))
        }
        "java.lang.String" => {
            return match reader.read_string(r)? {
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
            let count = reader.read_i32(r)?;
            for _ in 0..count {
                if field_desc.constant_pool {
                    elems.push(ValueDescriptor::ConstantPool(
                        field_desc.class_id,
                        reader.read_i64(r)?,
                    ));
                } else {
                    elems.push(read_value(r, reader, field_desc.class_id, type_pool)?);
                }
            }
            obj.fields.push(ValueDescriptor::Array(elems));
        } else {
            if field_desc.constant_pool {
                obj.fields.push(ValueDescriptor::ConstantPool(
                    field_desc.class_id,
                    reader.read_i64(r)?,
                ));
            } else {
                obj.fields
                    .push(read_value(r, reader, field_desc.class_id, type_pool)?);
            }
        }
    }

    Ok(ValueDescriptor::Object(obj))
}

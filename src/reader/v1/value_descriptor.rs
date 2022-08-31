//! Low-level representation of the decoded values.

use crate::reader::v1::byte_reader::{ByteReader, StringType};
use crate::reader::v1::constant_pool::ConstantPool;
use crate::reader::v1::type_descriptor::TypePool;
use crate::reader::{Error, Result};
use std::io::Read;
use std::process::id;

#[derive(Debug)]
pub enum ValueDescriptor<'cp> {
    Primitive(Primitive),
    Object(Object<'cp>),
    Array(Vec<ValueDescriptor<'cp>>),
    ConstantPool(&'cp ValueDescriptor<'cp>),
}

#[derive(Debug)]
pub struct Object<'cp> {
    pub class_id: i64,
    pub fields: Vec<ValueDescriptor<'cp>>,
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

pub fn read_value<'cp, R>(
    r: &mut R,
    reader: &ByteReader,
    class_id: i64,
    type_pool: &TypePool<'_>,
    constant_pool: &'cp ConstantPool<'cp>,
) -> Result<ValueDescriptor<'cp>>
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
                    let s = constant_pool
                        .get(type_desc.class_id, idx)
                        .ok_or(Error::InvalidFormat)?;
                    Ok(ValueDescriptor::ConstantPool(s))
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
                    let c = constant_pool
                        .get(field_desc.class_id, reader.read_i64(r)?)
                        .ok_or(Error::InvalidFormat)?;
                    elems.push(ValueDescriptor::ConstantPool(c));
                } else {
                    elems.push(read_value(
                        r,
                        reader,
                        field_desc.class_id,
                        type_pool,
                        constant_pool,
                    )?);
                }
            }
            obj.fields.push(ValueDescriptor::Array(elems));
        } else {
            if field_desc.constant_pool {
                let c = constant_pool
                    .get(field_desc.class_id, reader.read_i64(r)?)
                    .ok_or(Error::InvalidFormat)?;
                obj.fields.push(ValueDescriptor::ConstantPool(c));
            } else {
                obj.fields.push(read_value(
                    r,
                    reader,
                    field_desc.class_id,
                    type_pool,
                    constant_pool,
                )?);
            }
        }
    }

    Ok(ValueDescriptor::Object(obj))
}

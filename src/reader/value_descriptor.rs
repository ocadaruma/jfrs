//! Low-level representation of the decoded JFR values.

use crate::reader::byte_stream::{ByteStream, StringType};
use crate::reader::metadata::Metadata;

use crate::reader::type_descriptor::{FieldDescriptor, TypeDescriptor};
use crate::reader::{Chunk, Error, Result};
use std::io::Read;

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
            fields: Vec::with_capacity(type_desc.fields.len()),
        };

        for field_desc in type_desc.fields.iter() {
            let value = if field_desc.array_type {
                let count = stream.read_i32()? as usize;
                let mut elems = Vec::with_capacity(count);
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

    pub fn get_field<'a>(&'a self, name: &str, chunk: &'a Chunk) -> Option<&'a ValueDescriptor> {
        match self {
            ValueDescriptor::Object(o) => Self::get_object_field(o, name, chunk),
            ValueDescriptor::ConstantPool {
                class_id,
                constant_index,
            } => match chunk.constant_pool.get(class_id, constant_index) {
                Some(ValueDescriptor::Object(o)) => Self::get_object_field(o, name, chunk),
                _ => None,
            },
            _ => None,
        }
    }

    fn get_object_field<'a>(
        obj: &'a Object,
        name: &str,
        chunk: &'a Chunk,
    ) -> Option<&'a ValueDescriptor> {
        let res = chunk
            .metadata
            .type_pool
            .get(obj.class_id)
            .and_then(|c| c.get_field(name))
            .and_then(|(idx, _)| obj.fields.get(idx));

        match res {
            Some(ValueDescriptor::ConstantPool {
                class_id,
                constant_index,
            }) => chunk.constant_pool.get(class_id, constant_index) ,
            _ => res
        }
    }

    fn try_read_field_single<T: Read>(
        stream: &mut ByteStream<T>,
        field_desc: &FieldDescriptor,
        metadata: &Metadata,
    ) -> Result<ValueDescriptor> {
        if field_desc.constant_pool {
            Ok(ValueDescriptor::ConstantPool {
                class_id: field_desc.class_id,
                constant_index: stream.read_i64()?,
            })
        } else {
            Self::try_new(stream, field_desc.class_id, metadata)
        }
    }

    fn try_read_primitive<T: Read>(
        stream: &mut ByteStream<T>,
        type_desc: &TypeDescriptor,
    ) -> Result<Option<ValueDescriptor>> {
        let value = match type_desc.name() {
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

#[macro_use]
mod macros {
    macro_rules! impl_try_from_primitive {
        ($variant:ident, $ty:ty) => {
            impl<'a> TryFrom<&'a ValueDescriptor> for &'a $ty {
                type Error = ();
                fn try_from(value: &'a ValueDescriptor) -> std::result::Result<Self, Self::Error> {
                    if let ValueDescriptor::Primitive(Primitive::$variant(v)) = value {
                        Ok(v)
                    } else {
                        Err(())
                    }
                }
            }

            impl<'a> TryFrom<&'a ValueDescriptor> for $ty {
                type Error = ();
                fn try_from(value: &'a ValueDescriptor) -> std::result::Result<Self, Self::Error> {
                    <&$ty>::try_from(value).map(|v| *v)
                }
            }
        };
    }
}

impl_try_from_primitive!(Integer, i32);
impl_try_from_primitive!(Long, i64);
impl_try_from_primitive!(Float, f32);
impl_try_from_primitive!(Double, f64);
impl_try_from_primitive!(Character, char);
impl_try_from_primitive!(Boolean, bool);
impl_try_from_primitive!(Short, i16);
impl_try_from_primitive!(Byte, i8);

impl<'a> TryFrom<&'a ValueDescriptor> for &'a str {
    type Error = ();

    fn try_from(value: &'a ValueDescriptor) -> std::result::Result<Self, Self::Error> {
        if let ValueDescriptor::Primitive(Primitive::String(s)) = value {
            Ok(s.as_str())
        } else {
            Err(())
        }
    }
}

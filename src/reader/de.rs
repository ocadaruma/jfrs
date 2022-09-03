use crate::reader::value_descriptor::{Object, Primitive, ValueDescriptor};
use crate::reader::{Chunk, Error};
use serde::de::value::{BorrowedStrDeserializer, StrDeserializer};
use serde::de::{DeserializeSeed, IntoDeserializer, Visitor};
use serde::forward_to_deserialize_any;
use std::fmt::Display;

pub struct Deserializer<'de> {
    chunk: &'de Chunk,
    value: &'de ValueDescriptor,
}

impl<'de> Deserializer<'de> {
    pub fn new(chunk: &'de Chunk, value: &'de ValueDescriptor) -> Self {
        Self { chunk, value }
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::DeserializeError(msg.to_string())
    }
}

struct ObjectDeserializer<'de> {
    chunk: &'de Chunk,
    field_idx: usize,
    value: &'de Object,
}

impl<'de> serde::de::MapAccess<'de> for ObjectDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.field_idx >= self.value.fields.len() {
            return Ok(None);
        }
        if let Some(key) = self
            .chunk
            .metadata
            .type_pool
            .get(self.value.class_id)
            .map(|t| t.fields[self.field_idx].name.as_ref())
        {
            let key: StrDeserializer<Self::Error> = key.into_deserializer();
            let key: K::Value = seed.deserialize(key)?;
            Ok(Some(key))
        } else {
            Err(Error::ClassNotFound(self.value.class_id))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        assert!(self.field_idx < self.value.fields.len());
        let value = seed.deserialize(Deserializer::new(
            self.chunk,
            &self.value.fields[self.field_idx],
        ))?;
        self.field_idx += 1;
        Ok(value)
    }
}

struct ArrayDeserializer<'de> {
    chunk: &'de Chunk,
    array_idx: usize,
    value: &'de Vec<ValueDescriptor>,
}

impl<'de> serde::de::SeqAccess<'de> for ArrayDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.array_idx >= self.value.len() {
            return Ok(None);
        }
        let value = seed.deserialize(Deserializer::new(self.chunk, &self.value[self.array_idx]))?;
        self.array_idx += 1;
        Ok(Some(value))
    }
}

impl<'de> serde::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        use ValueDescriptor::Primitive;
        use crate::reader::value_descriptor::Primitive::*;

        match self.value {
            Primitive(Integer(v)) => visitor.visit_i32(*v),
            Primitive(Long(v)) => visitor.visit_i64(*v),
            Primitive(Float(v)) => visitor.visit_f32(*v),
            Primitive(Double(v)) => visitor.visit_f64(*v),
            Primitive(Character(v)) => visitor.visit_char(*v),
            Primitive(Boolean(v)) => visitor.visit_bool(*v),
            Primitive(Short(v)) => visitor.visit_i16(*v),
            Primitive(Byte(v)) => visitor.visit_i8(*v),
            Primitive(String(v)) => visitor.visit_borrowed_str(v.as_str()),
            Primitive(NullString) => Err(Error::DeserializeError("Unexpected null string".to_string())),
            ValueDescriptor::Object(obj) => visitor.visit_map(ObjectDeserializer {
                chunk: self.chunk,
                field_idx: 0,
                value: obj,
            }),
            ValueDescriptor::Array(array) => visitor.visit_seq(ArrayDeserializer {
                chunk: self.chunk,
                array_idx: 0,
                value: array,
            }),
            ValueDescriptor::ConstantPool {
                class_id,
                constant_index,
            } => match self.chunk.constant_pool.get(class_id, constant_index) {
                Some(value) => Self::deserialize_any(Deserializer::new(self.chunk, value), visitor),
                None => Err(Error::DeserializeError(format!("Not found in constant pool: class_id={}, index={}", class_id, constant_index)))
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        match self.value {
            ValueDescriptor::Primitive(Primitive::NullString) => visitor.visit_none(),
            ValueDescriptor::ConstantPool {
                class_id, constant_index
            } => match self.chunk.constant_pool.get(class_id, constant_index) {
                Some(value) => visitor.visit_some(Deserializer::new(self.chunk, value)),
                None => visitor.visit_none(),
            },
            _ => visitor.visit_some(self)
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any struct
    }
}

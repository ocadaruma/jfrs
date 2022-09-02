//! Descriptor of types declared in the JFR chunk.
//! TypeDescriptor defines the "schema" of types.
//! Event and ConstantPool values are parsed based on declared TypeDescriptor.

use crate::reader::{Error, Result};
use std::collections::HashMap;
use std::io::Read;
use crate::reader::byte_stream::{ByteStream, StringType};

#[derive(Debug)]
pub struct StringTable(Vec<Option<String>>);

impl StringTable {
    pub fn try_new<T: Read>(stream: &mut ByteStream<T>) -> Result<StringTable> {
        let string_count = stream.read_i32()?;
        let mut strings = Vec::with_capacity(string_count as usize);

        for _ in 0..string_count {
            match stream.read_string()? {
                StringType::Null => strings.push(None),
                StringType::Empty => strings.push(Some("".to_string())),
                StringType::Raw(s) => strings.push(Some(s)),
                _ => return Err(Error::InvalidString),
            }
        }

        Ok(StringTable(strings))
    }

    pub fn get(&self, idx: i32) -> Result<&str> {
        self.0
            .get(idx as usize)
            .and_then(|s| s.as_ref())
            .ok_or(Error::InvalidStringIndex(idx))
            .map(|s| s.as_str())
    }
}

#[derive(Debug, Default)]
pub struct TypePool<'st> {
    inner: HashMap<i64, TypeDescriptor<'st>>,
}

impl<'st> TypePool<'st> {
    pub fn register(&mut self, class_id: i64, desc: TypeDescriptor<'st>) {
        self.inner.insert(class_id, desc);
    }

    pub fn get(&self, class_id: i64) -> Option<&TypeDescriptor<'st>> {
        self.inner.get(&class_id)
    }
}

#[derive(Debug)]
pub struct TypeDescriptor<'st> {
    pub class_id: i64,
    pub name: &'st str,
    pub super_type: Option<&'st str>,
    pub simple_type: bool,
    pub fields: Vec<FieldDescriptor<'st>>,

    // these fields are filled by annotations
    pub label: Option<&'st str>,
    pub description: Option<&'st str>,
    pub experimental: bool,
    pub category: Vec<&'st str>,
}

#[derive(Debug)]
pub struct FieldDescriptor<'st> {
    pub class_id: i64,
    pub name: &'st str,
    pub label: Option<&'st str>,
    pub description: Option<&'st str>,
    pub experimental: bool,
    pub constant_pool: bool,
    pub array_type: bool,
    pub unsigned: bool,
    pub unit: Option<Unit>,
    pub tick_unit: Option<TickUnit>,
}

#[derive(Debug)]
pub enum Unit {
    Byte,
    PercentUnity,
    AddressUnity,
    Hz,
    Nanosecond,
    Millisecond,
    Second,
    EpochNano,
    EpochMilli,
    EpochSecond,
}

#[derive(Debug)]
pub enum TickUnit {
    Timespan,
    Timestamp,
}

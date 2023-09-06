//! Descriptor of types declared in the JFR chunk.
//! TypeDescriptor defines the "schema" of types.
//! Event and ConstantPool values are parsed based on declared TypeDescriptor.

use crate::reader::byte_stream::{ByteStream, StringType};
use crate::reader::{Error, Result};
use std::io::Read;

use rustc_hash::FxHashMap;
use std::rc::Rc;

/// String intern pool
#[derive(Debug)]
pub struct StringTable(Vec<Option<Rc<str>>>);

impl StringTable {
    pub fn try_new<T: Read>(stream: &mut ByteStream<T>) -> Result<Self> {
        let string_count = stream.read_i32()?;
        let mut strings = Vec::with_capacity(string_count as usize);

        for _ in 0..string_count {
            match stream.read_string()? {
                StringType::Null => strings.push(None),
                StringType::Empty => strings.push(Some(Rc::from(""))),
                StringType::Raw(s) => strings.push(Some(Rc::from(s))),
                _ => return Err(Error::InvalidString),
            }
        }

        Ok(Self(strings))
    }

    pub fn get(&self, idx: i32) -> Result<&Rc<str>> {
        self.0
            .get(idx as usize)
            .and_then(|s| s.as_ref())
            .ok_or(Error::InvalidStringIndex(idx))
    }
}

#[derive(Debug, Default)]
pub struct TypePool {
    pub(crate) inner: FxHashMap<i64, TypeDescriptor>,
}

impl TypePool {
    pub fn register(&mut self, class_id: i64, desc: TypeDescriptor) {
        self.inner.insert(class_id, desc);
    }

    pub fn get(&self, class_id: i64) -> Option<&TypeDescriptor> {
        self.inner.get(&class_id)
    }

    pub fn get_types(&self) -> impl Iterator<Item = &TypeDescriptor> {
        self.inner.values()
    }
}

#[derive(Debug)]
pub struct TypeDescriptor {
    pub class_id: i64,
    pub(crate) name: Rc<str>,
    pub(crate) super_type: Option<Rc<str>>,
    pub simple_type: bool,
    pub fields: Vec<FieldDescriptor>,

    // these fields are filled by annotations
    pub(crate) label: Option<Rc<str>>,
    pub(crate) description: Option<Rc<str>>,
    pub experimental: bool,
    pub(crate) category: Vec<Rc<str>>,
}

impl TypeDescriptor {
    pub fn get_field(&self, name: &str) -> Option<(usize, &FieldDescriptor)> {
        for (idx, field) in self.fields.iter().enumerate() {
            if field.name.as_ref() == name {
                return Some((idx, field));
            }
        }
        None
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn super_type(&self) -> Option<&str> {
        self.super_type.as_ref().map(|s| s.as_ref())
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_ref().map(|s| s.as_ref())
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_ref())
    }

    pub fn category(&self) -> impl Iterator<Item = &str> {
        self.category.iter().map(|s| s.as_ref())
    }
}

#[derive(Debug)]
pub struct FieldDescriptor {
    pub class_id: i64,
    pub(crate) name: Rc<str>,
    pub(crate) label: Option<Rc<str>>,
    pub(crate) description: Option<Rc<str>>,
    pub experimental: bool,
    pub constant_pool: bool,
    pub array_type: bool,
    pub unsigned: bool,
    pub unit: Option<Unit>,
    pub tick_unit: Option<TickUnit>,
}

impl FieldDescriptor {
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_ref().map(|s| s.as_ref())
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|s| s.as_ref())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TickUnit {
    Timespan,
    Timestamp,
}

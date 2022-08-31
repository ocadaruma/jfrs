//! Descriptor of types declared in the JFR chunk.
//! TypeDescriptor defines the "schema" of types.
//! Event and ConstantPool values are parsed based on declared TypeDescriptor.

use std::collections::HashMap;

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

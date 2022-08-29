#[derive(Debug)]
pub enum ValueDescriptor<'st> {
    Primitive(Primitive<'st>),
    Object(Object<'st>),
    Array(Vec<ValueDescriptor<'st>>),
    ConstantPool(&'st ValueDescriptor<'st>),
}

#[derive(Debug)]
pub struct Object<'st> {
    pub class_id: i64,
    pub fields: Vec<ValueDescriptor<'st>>,
}

#[derive(Debug)]
pub enum Primitive<'st> {
    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Character(char),
    Boolean(bool),
    Short(i16),
    Byte(i8),
    String(&'st str),
    ConstantPoolString(i64),
}

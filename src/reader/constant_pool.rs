// use crate::reader::v1::byte_reader::ByteReader;
// use crate::reader::type_descriptor::TypePool;
// use crate::reader::value_descriptor::{read_value, ValueDescriptor};
// use crate::reader::v1::{ChunkHeader, Result};
// use crate::reader::Error;
// use std::collections::HashMap;
// use std::io::{Read, Seek, SeekFrom};
//
// const EVENT_TYPE_CONSTANT_POOL: i64 = 1;
//
// #[derive(Debug, Default)]
// struct PerTypePool {
//     inner: HashMap<i64, ValueDescriptor>,
// }
//
// #[derive(Debug, Default)]
// pub struct ConstantPool {
//     inner: HashMap<i64, PerTypePool>,
// }
//
// impl ConstantPool {
//     pub fn register(&mut self, class_id: i64, constant_index: i64, value: ValueDescriptor) {
//         self.inner
//             .entry(class_id)
//             .or_insert(PerTypePool::default())
//             .inner
//             .insert(constant_index, value);
//     }
//
//     pub fn get(&self, class_id: i64, constant_index: i64) -> Option<&ValueDescriptor> {
//         self.inner
//             .get(&class_id)
//             .and_then(|p| p.inner.get(&constant_index))
//     }
// }
//
// pub struct ConstantPoolReader<'a, R>(&'a mut R);
//
// impl<'a, R> ConstantPoolReader<'a, R>
// where
//     R: Read + Seek,
// {
//     pub fn wrap(inner: &'a mut R) -> Self {
//         Self(inner)
//     }
//
//     pub fn read_constant_pool(
//         &mut self,
//         reader: &ByteReader,
//         header: &ChunkHeader,
//         type_pool: &TypePool<'_>,
//     ) -> Result<ConstantPool> {
//         let mut constant_pool = ConstantPool::default();
//
//         let mut offset = 0;
//         let mut delta = header.constant_pool_offset;
//         while delta != 0 {
//             offset += delta;
//             self.0
//                 .seek(SeekFrom::Start(offset as u64))
//                 .map_err(Error::IoError)?;
//             delta = self.read_constant_pool_event(reader, &mut constant_pool, type_pool)?;
//         }
//
//         Ok(constant_pool)
//     }
//
//     fn read_constant_pool_event(
//         &mut self,
//         reader: &ByteReader,
//         constant_pool: &mut ConstantPool,
//         type_pool: &TypePool<'_>,
//     ) -> Result<i64> {
//         // size
//         reader.read_i32(self.0)?;
//         if reader.read_i64(self.0)? != EVENT_TYPE_CONSTANT_POOL {
//             return Err(Error::InvalidFormat);
//         }
//
//         // start
//         reader.read_i64(self.0)?;
//         // duration
//         reader.read_i64(self.0)?;
//
//         let delta = reader.read_i64(self.0)?;
//         // flush
//         reader.read_i8(self.0)?;
//         let pool_count = reader.read_i32(self.0)?;
//
//         for _ in 0..pool_count {
//             let class_id = reader.read_i64(self.0)?;
//             let constant_count = reader.read_i32(self.0)?;
//
//             for _ in 0..constant_count {
//                 let constant_index = reader.read_i64(self.0)?;
//                 let value = read_value(self.0, reader, class_id, type_pool)?;
//                 constant_pool.register(class_id, constant_index, value);
//             }
//         }
//
//         Ok(delta)
//     }
// }

use crate::reader::byte_stream::ByteStream;
use crate::reader::metadata::Metadata;

use crate::reader::value_descriptor::ValueDescriptor;
use crate::reader::Error;
use crate::reader::{ChunkHeader, Result};
use crate::EVENT_TYPE_CONSTANT_POOL;
use rustc_hash::FxHashMap;
use std::io::{Read, Seek};

#[derive(Debug, Default)]
pub struct PerTypePool {
    pub(crate) inner: FxHashMap<i64, ValueDescriptor>,
}

#[derive(Debug, Default)]
pub struct ConstantPool {
    pub(crate) inner: FxHashMap<i64, PerTypePool>,
}

impl ConstantPool {
    pub fn try_new<T: Read + Seek>(
        stream: &mut ByteStream<T>,
        header: &ChunkHeader,
        metadata: &Metadata,
    ) -> Result<Self> {
        let mut constant_pool = Self::default();
        let mut offset = 0;
        let mut delta = header.constant_pool_offset;
        while delta != 0 {
            offset += delta;
            stream.seek(offset as u64)?;
            delta = Self::read_constant_pool_event(stream, &mut constant_pool, metadata)?;
        }

        Ok(constant_pool)
    }

    pub fn register(&mut self, class_id: i64, constant_index: i64, value: ValueDescriptor) {
        self.inner
            .entry(class_id)
            .or_insert(PerTypePool::default())
            .inner
            .insert(constant_index, value);
    }

    pub fn get(&self, class_id: &i64, constant_index: &i64) -> Option<&ValueDescriptor> {
        self.inner
            .get(&class_id)
            .and_then(|p| p.inner.get(&constant_index))
    }

    fn read_constant_pool_event<T: Read + Seek>(
        stream: &mut ByteStream<T>,
        constant_pool: &mut ConstantPool,
        metadata: &Metadata,
    ) -> Result<i64> {
        // size
        stream.read_i32()?;
        if stream.read_i64()? != EVENT_TYPE_CONSTANT_POOL {
            return Err(Error::InvalidFormat);
        }

        // start
        stream.read_i64()?;
        // duration
        stream.read_i64()?;

        let delta = stream.read_i64()?;
        // flush
        stream.read_i8()?;
        let pool_count = stream.read_i32()?;

        for _ in 0..pool_count {
            let class_id = stream.read_i64()?;
            let constant_count = stream.read_i32()?;

            for _ in 0..constant_count {
                let constant_index = stream.read_i64()?;
                let value = ValueDescriptor::try_new(stream, class_id, metadata)?;
                constant_pool.register(class_id, constant_index, value);
            }
        }

        Ok(delta)
    }
}

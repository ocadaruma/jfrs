use crate::reader::byte_stream::ByteStream;
use crate::reader::type_descriptor::TypeDescriptor;
use crate::reader::value_descriptor::ValueDescriptor;
use crate::reader::{Chunk, Error, HeapByteStream, Result};
use crate::{EVENT_TYPE_CONSTANT_POOL, EVENT_TYPE_METADATA};
use std::io::{Read, Seek};

#[derive(Debug)]
pub struct Event<'a> {
    pub class: &'a TypeDescriptor,
    pub value: ValueDescriptor,
}

pub struct EventIterator<'a> {
    chunk: &'a Chunk,
    stream: HeapByteStream,
    offset: u64,
}

impl<'a> EventIterator<'a> {
    pub fn new(chunk: &'a Chunk, stream: HeapByteStream) -> Self {
        Self {
            chunk,
            stream,
            offset: 0,
        }
    }

    fn internal_next(&mut self) -> Result<Option<Event<'a>>> {
        let end_offset = self.chunk.header.chunk_body_size();

        while self.offset < end_offset {
            self.stream
                .seek(self.chunk.header.body_start_offset() + self.offset)?;

            let size = self.stream.read_i32()?;
            let event_type = self.stream.read_i64()?;
            self.offset += size as u64;

            match event_type {
                EVENT_TYPE_METADATA | EVENT_TYPE_CONSTANT_POOL => {}
                _ => {
                    let type_desc = self
                        .chunk
                        .metadata
                        .type_pool
                        .get(event_type)
                        .ok_or(Error::ClassNotFound(event_type))?;
                    let value = ValueDescriptor::try_new(
                        &mut self.stream,
                        event_type,
                        &self.chunk.metadata,
                    )?;

                    return Ok(Some(Event {
                        class: type_desc,
                        value,
                    }));
                }
            }
        }
        Ok(None)
    }
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = Result<Event<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_next() {
            Ok(Some(e)) => Some(Ok(e)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

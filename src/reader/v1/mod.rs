// use crate::reader::v1::byte_reader::{ByteReader, StringType};
// use crate::reader::constant_pool::ConstantPoolReader;
// use crate::reader::metadata::{Metadata, MetadataReader, StringTable};
// use crate::reader::{Error, Result};
// use std::io::{Read, Seek, SeekFrom};
//
// mod byte_reader;
//
// const FEATURES_COMPRESSED_INTS: i32 = 1;
//
// pub struct ChunkReader<'a, R>(&'a mut R);
//
// #[derive(Debug)]
// pub struct Chunk {
//     string_table: StringTable,
//     // constant_pools: ConstantPools,
//     // events: Events,
// }
//
// #[derive(Debug)]
// struct ChunkHeader {
//     chunk_size: i64,
//     constant_pool_offset: i64,
//     metadata_offset: i64,
//     start_time_nanos: i64,
//     duration_nanos: i64,
//     start_ticks: i64,
//     ticks_per_second: i64,
//     features: i32,
//     body_start_offset: u64,
// }
//
// impl ChunkHeader {
//     fn is_ints_compressed(&self) -> bool {
//         self.features & FEATURES_COMPRESSED_INTS != 0
//     }
// }
//
// impl<'a, R> ChunkReader<'a, R>
// where
//     R: Read + Seek,
// {
//     pub fn wrap(inner: &'a mut R) -> Self {
//         Self(inner)
//     }
//
//     pub fn read(&mut self) -> Result<Chunk> {
//         let header = self.read_header()?;
//         println!("header: {:#?}", header);
//
//         self.0
//             .seek(SeekFrom::Start(header.metadata_offset as u64))
//             .map_err(Error::IoError)?;
//
//         let reader = if header.is_ints_compressed() {
//             ByteReader::CompressedInts
//         } else {
//             ByteReader::Raw
//         };
//
//         let mut meta_reader = MetadataReader::wrap(self.0);
//         let metadata = meta_reader.read_metadata(&reader)?;
//         let type_pool = meta_reader.read_types(&reader, &metadata)?;
//         println!("type_pool: {:#?}", type_pool);
//         let constant_pool =
//             ConstantPoolReader::wrap(self.0).read_constant_pool(&reader, &header, &type_pool)?;
//         println!("constant_pool: {:#?}", constant_pool);
//
//         Err(Error::InvalidFormat)
//     }
//
//     fn read_header(&mut self) -> Result<ChunkHeader> {
//         let reader = ByteReader::Raw;
//
//         let header = ChunkHeader {
//             chunk_size: reader.read_i64(self.0)?,
//             constant_pool_offset: reader.read_i64(self.0)?,
//             metadata_offset: reader.read_i64(self.0)?,
//             start_time_nanos: reader.read_i64(self.0)?,
//             duration_nanos: reader.read_i64(self.0)?,
//             start_ticks: reader.read_i64(self.0)?,
//             ticks_per_second: reader.read_i64(self.0)?,
//             features: reader.read_i32(self.0)?,
//             body_start_offset: self.0.stream_position().map_err(Error::IoError)?,
//         };
//
//         Ok(header)
//     }
// }

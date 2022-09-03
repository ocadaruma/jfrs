use std::env;
use std::fs::File;
use std::io::BufReader;
use jfrs::reader::{from_event, JfrReader};
use jfrs::reader::value_descriptor::ValueDescriptor;
use jfrs::reader::value_descriptor::Primitive;
use jfrs::reader::types::jdk::ExecutionSample;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut reader = JfrReader::new(BufReader::new(File::open(&args[1]).unwrap()));

    let mut event_count = 0;
    let mut os_name_total_length = 0;

    println!("started");
    while let Some(chunk) = reader.next() {
        let chunk = chunk.unwrap();
        // TODO class_name should not be exposed directly
        for event in reader.events(&chunk).flatten().filter(|e| e.class.name.as_ref() == "jdk.ExecutionSample") {
            // let sample: ExecutionSample = from_event(&chunk, &event).unwrap();
            event_count += 1;
            // os_name_total_length += sample.sampled_thread.unwrap().os_name.unwrap().len();
            match event.value.get_field("sampledThread", &chunk)
                .and_then(|t| t.get_field("osName", &chunk)) {
                Some(ValueDescriptor::Primitive(Primitive::String(s))) => {
                    os_name_total_length += s.len();
                }
                _ => {}
            }
        }
    }
    println!("event_count: {}, os_name_total_length: {}", event_count, os_name_total_length);
}

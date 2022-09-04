use jfrs::reader::value_descriptor::Primitive;
use jfrs::reader::value_descriptor::ValueDescriptor;
use jfrs::reader::JfrReader;
use std::env;
use std::fs::File;
use std::io::BufReader;

fn main() {
    let args: Vec<String> = env::args().collect();

    let path = &args[1];
    let iteration: usize = args[2].parse().unwrap();

    for _ in 0..iteration {
        let mut reader = JfrReader::new(BufReader::new(File::open(path).unwrap()));

        let mut event_count = 0;
        let mut os_name_total_length = 0;

        println!("started");
        for (reader, chunk) in reader.chunks().flatten() {
            // TODO class_name should not be exposed directly?
            for event in reader
                .events(&chunk)
                .flatten()
                .filter(|e| e.class.name.as_ref() == "jdk.ExecutionSample")
            {
                // let sample: ExecutionSample = from_event(&chunk, &event).unwrap();
                event_count += 1;
                // os_name_total_length += sample.sampled_thread.unwrap().os_name.unwrap().len();
                match event
                    .value
                    .get_field("sampledThread", &chunk)
                    .and_then(|t| t.get_field("osName", &chunk))
                {
                    Some(ValueDescriptor::Primitive(Primitive::String(s))) => {
                        os_name_total_length += s.len();
                    }
                    _ => {}
                }
            }
        }
        println!(
            "event_count: {}, os_name_total_length: {}",
            event_count, os_name_total_length
        );
    }
}

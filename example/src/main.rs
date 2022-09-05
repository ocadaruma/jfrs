use jfrs::reader::JfrReader;
use std::env;
use std::fs::File;
use std::io::BufReader;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let args: Vec<String> = env::args().collect();

    let path = &args[1];
    let iteration: usize = args[2].parse().unwrap();

    for _ in 0..iteration {
        let mut reader = JfrReader::new(BufReader::new(File::open(path).unwrap()));

        let mut event_count = 0;
        let mut os_name_total_length = 0;

        println!("started");
        for (reader, chunk) in reader.chunks().flatten() {
            for event in reader
                .events(&chunk)
                .flatten()
                .filter(|e| e.class.name() == "jdk.ExecutionSample")
            {
                // let sample: ExecutionSample = from_event(&event).unwrap();
                // os_name_total_length += sample.sampled_thread.unwrap().os_name.unwrap().len();

                let thread_name = event
                    .value()
                    .get_field("sampledThread")
                    .and_then(|v| v.get_field("osName"))
                    .and_then(|v| <&str>::try_from(v.value).ok())
                    .unwrap();
                os_name_total_length += thread_name.len();

                event_count += 1;
            }
        }
        println!(
            "event_count: {}, os_name_total_length: {}",
            event_count, os_name_total_length
        );
    }
}

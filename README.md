# jfrs

[![CI](https://github.com/ocadaruma/jfrs/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/ocadaruma/jfrs/actions/workflows/ci.yml)
[![Crate](https://img.shields.io/crates/v/jfrs.svg)](https://crates.io/crates/jfrs)

Java Flight Recorder reader for Rust

## Features

### Read events (low-level API)

```rust
fn main() {
    let mut reader = JfrReader::new(File::open("/path/to/recording.jfr").unwrap());

    for (reader, chunk) in reader.chunks().flatten() {
        for event in reader.events(&chunk).filter(|e| e.class.name() == "jdk.ExecutionSample") {
            let thread_name = event.value
                .get_field("sampledThread", &chunk)
                .and_then(|f| f.get_field("osName", &chunk))
                .and_then(|v| match v {
                    ValueDescriptor::Primitive(Primitive::String(s)) => Some(s),
                    _ => None,
                })
                .unwrap();
            println!("sampled thread: {}", thread_name);
        }
    }
}
```

### \[Experimental\] Deserialize events as Rust struct

> **NOTE**
> As of now, deserialization performance is very poor. See [tuning_notes.md](./example/tuning_notes.md) for the details.

Though low-level API provides full functionality to interpret the events as you need,
usually we want to map known JFR events to the Rust structure.

`jfrs` also provides `serde-rs` based deserialization feature.

```rust
fn main() {
    let mut reader = JfrReader::new(File::open("/path/to/recording.jfr").unwrap());

    for (reader, chunk) in reader.chunks().flatten() {
        for event in reader.events(&chunk).filter(|e| e.class.name() == "jdk.ExecutionSample") {
            let sample: ExecutionSample = from_event(&chunk, &event).unwrap();
            println!("sampled thread: {}", sample.sampled_thread.and_then(|t| t.os_name).unwrap());
        }
    }
}
```

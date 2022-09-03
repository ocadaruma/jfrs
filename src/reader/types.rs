//! Predefine JFR types for ease of parsing
//!
//! Related JMC code: [TypesImpl.java](https://github.com/openjdk/jmc/blob/8.2.0-ga/core/org.openjdk.jmc.flightrecorder.writer/src/main/java/org/openjdk/jmc/flightrecorder/writer/TypesImpl.java)
//! TODO: should refer TypeManager instead?

pub mod builtin {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct JdkThread<'a> {
        // In JFR, strings are encoded as 5 types: utf8, char-array, constant-pool, empty, null
        // To allow null, string field must be always Option.
        // Also, currently all str are supposed to be borrowed from deserializer so must be &str (not String)
        pub os_name: Option<&'a str>,
        pub os_thread_id: i64,
        #[serde(default)]
        pub java_name: Option<&'a str>,
        #[serde(default)]
        pub java_thread_id: i64,
        #[serde(borrow)]
        pub group: Option<ThreadGroup<'a>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadGroup<'a> {
        #[serde(borrow, default)]
        pub parent: Option<Box<ThreadGroup<'a>>>,
        pub name: Option<&'a str>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StackTrace<'a> {
        #[serde(default)]
        pub truncated: bool,
        #[serde(borrow, default)]
        pub frames: Vec<Option<StackFrame<'a>>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StackFrame<'a> {
        #[serde(borrow)]
        pub method: Option<JdkMethod<'a>>,
        #[serde(default)]
        pub line_number: i32,
        #[serde(default)]
        pub bytecode_index: i32,
        #[serde(rename = "type", borrow)]
        pub frame_type: Option<FrameType<'a>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FrameType<'a> {
        pub description: Option<&'a str>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct JdkMethod<'a> {
        #[serde(rename = "type", borrow)]
        pub class: Option<Class<'a>>,
        #[serde(borrow)]
        pub name: Option<Symbol<'a>>,
        #[serde(borrow)]
        pub descriptor: Option<Symbol<'a>>,
        #[serde(default)]
        pub modifiers: i32,
        #[serde(default)]
        pub hidden: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Symbol<'a> {
        pub string: Option<&'a str>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Class<'a> {
        #[serde(borrow, default)]
        pub class_loader: Option<ClassLoader<'a>>,
        #[serde(borrow)]
        pub name: Option<Symbol<'a>>,
        #[serde(borrow, default)]
        pub package: Option<Package<'a>>,
        #[serde(default)]
        pub modifiers: i32,
        #[serde(default)]
        pub hidden: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Package<'a> {
        #[serde(borrow)]
        pub name: Option<Symbol<'a>>,
        #[serde(borrow)]
        pub module: Option<Module<'a>>,
        #[serde(default)]
        pub exported: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Module<'a> {
        #[serde(borrow)]
        pub name: Option<Symbol<'a>>,
        #[serde(borrow)]
        pub version: Option<Symbol<'a>>,
        #[serde(borrow)]
        pub location: Symbol<'a>,
        #[serde(borrow, default)]
        pub class_loader: Option<ClassLoader<'a>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ClassLoader<'a> {
        #[serde(rename = "type", borrow, default)]
        pub class: Option<Box<Class<'a>>>,
        #[serde(borrow)]
        pub name: Option<Symbol<'a>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ThreadState<'a> {
        pub name: Option<&'a str>,
    }
}

pub mod jdk {
    use super::builtin::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ExecutionSample<'a> {
        #[serde(borrow)]
        pub sampled_thread: Option<JdkThread<'a>>,
        #[serde(borrow)]
        pub stack_trace: Option<StackTrace<'a>>,
        #[serde(borrow)]
        pub state: Option<ThreadState<'a>>,
    }
}

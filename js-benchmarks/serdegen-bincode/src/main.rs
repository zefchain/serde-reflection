fn main() {
    use serde_reflection::{Registry, Tracer, TracerConfig};
    let mut tracer = Tracer::new(TracerConfig::default());

    #[derive(serde::Deserialize)]
    pub struct Test {
        pub string: String,
        pub uint32: u32,
        pub inner: Inner,
        pub float: f32,
    }
    #[derive(serde::Deserialize)]
    pub struct Inner {
        pub int32: i32,
        pub inner_inner: InnerInner,
        pub outer: Outer,
    }
    #[derive(serde::Deserialize)]
    pub struct InnerInner {
        pub long: i64,
        pub enum_value: Enum, // enum is a reserved keyword in Rust, use enum_value
        pub sint32: i32,
    }
    #[derive(serde::Deserialize)]
    pub struct Outer {
        pub bools: Vec<bool>, // repeated field is represented as a Vec
        pub double: f64,
    }
    #[derive(serde::Deserialize)]
    pub enum Enum {
        ONE = 0,
        TWO = 1,
        THREE = 2,
        FOUR = 3,
        FIVE = 4,
    }

    tracer.trace_simple_type::<Test>().unwrap();
    tracer.trace_simple_type::<InnerInner>().unwrap();
    tracer.trace_simple_type::<Enum>().unwrap();
    tracer.trace_simple_type::<Outer>().unwrap();
    tracer.trace_simple_type::<Inner>().unwrap();

    let registry = tracer.registry().unwrap();

    use serde_generate::{
        typescript::{self, CodeGenerator},
        CodeGeneratorConfig, Encoding,
    };
    let mut source = Vec::new();
    let config =
        CodeGeneratorConfig::new("bincode".to_string()).with_encodings(vec![Encoding::Bincode]);
    let generator = typescript::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    std::fs::write("../src/bincode/registry/registry.ts", source).unwrap();
}

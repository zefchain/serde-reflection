use std::{collections::HashMap, error::Error};
use serde::{Deserialize, Serialize};
use bincode::{Encode, Decode};

fn main() -> Result<(), Box<dyn Error>> {
    use serde_reflection::{Registry, Tracer, TracerConfig};
    let mut tracer = Tracer::new(TracerConfig::default());

	#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
	struct SimpleStruct {
		a: u32,
		b: String,
	}
	
	#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
	enum MultiEnum {
		VariantA(i32),
		VariantB(String),
		VariantC { x: u8, y: f64 },
		UnitVariant,
	}
	
	#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
	struct UnitStruct;
	
	#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
	struct NewtypeStruct(i32);
	
	#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
	struct TupleStruct(i32, f64, String);
	
	#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
	struct ComplexStruct {
		inner: SimpleStruct,
		flag: bool,
		items: Vec<MultiEnum>,
		unit: UnitStruct,
		newtype: NewtypeStruct,
		tuple: TupleStruct,
		tupple_inline: (String, i32),
		map: HashMap<i32, i64>
	}
	
    tracer.trace_simple_type::<SimpleStruct>()?;
    tracer.trace_simple_type::<MultiEnum>()?;
    tracer.trace_simple_type::<UnitStruct>()?;
    tracer.trace_simple_type::<NewtypeStruct>()?;
    tracer.trace_simple_type::<TupleStruct>()?;
    tracer.trace_simple_type::<ComplexStruct>()?;

	let simple_instance = SimpleStruct { a: 42, b: "Hello".into() };
    let enum_instance = MultiEnum::VariantC { x: 5, y: 3.14 };
    let unit_variant = MultiEnum::UnitVariant;
    let complex_instance = ComplexStruct {
        inner: simple_instance.clone(),
        flag: true,
        items: vec![MultiEnum::VariantA(10), MultiEnum::VariantB("World".into())],
        unit: UnitStruct,
        newtype: NewtypeStruct(99),
        tuple: TupleStruct(123, 45.67, "Test".into()),
		tupple_inline: ("SomeString".into(), 777),
		map: HashMap::from_iter([(3, 7)])
    };

    println!("simple_instance: {:?}", bincode::encode_to_vec(&simple_instance, bincode::config::standard())?);
    println!("enum_instance: {:?}", bincode::encode_to_vec(&enum_instance, bincode::config::standard())?);
    println!("unit_variant: {:?}", bincode::encode_to_vec(&unit_variant, bincode::config::standard())?);
    println!("complex_instance: {:?}", bincode::encode_to_vec(&complex_instance, bincode::config::standard())?);

    let registry = tracer.registry()?;

    use serde_generate::{typescript, CodeGeneratorConfig, Encoding};
    let mut source = Vec::new();
    let config = CodeGeneratorConfig::new("bincode".into()).with_encodings(vec![Encoding::Bincode]);
    typescript::CodeGenerator::new(&config).output(&mut source, &registry)?;
    std::fs::write(format!("{}/ts/bincode/registry.ts", env!("CARGO_MANIFEST_DIR")), source)?;

	Ok(())
}

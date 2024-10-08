use std::{collections::HashMap, error::Error};
use serde::{Deserialize, Serialize};

fn main() -> Result<(), Box<dyn Error>> {
    use serde_reflection::{Registry, Tracer, TracerConfig};
    let mut tracer = Tracer::new(TracerConfig::default());

	#[derive(Clone, Debug, Serialize, Deserialize)]
	struct SimpleStruct {
		a: u32,
		b: String,
	}
	
	#[derive(Serialize, Deserialize, Debug)]
	enum MultiEnum {
		VariantA(i32),
		VariantB(String),
		VariantC { x: u8, y: f64 },
		UnitVariant,
	}
	
	#[derive(Serialize, Deserialize, Debug)]
	struct UnitStruct;
	
	#[derive(Serialize, Deserialize, Debug)]
	struct NewtypeStruct(i32);
	
	#[derive(Serialize, Deserialize, Debug)]
	struct TupleStruct(i32, f64, String);
	
	#[derive(Serialize, Deserialize, Debug)]
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

    println!("simple_instance: {:?}", bincode::serialize(&simple_instance)?);
    println!("enum_instance: {:?}", bincode::serialize(&enum_instance)?);
    println!("unit_variant: {:?}", bincode::serialize(&unit_variant)?);
    println!("complex_instance: {:?}", bincode::serialize(&complex_instance)?);

    let registry = tracer.registry()?;

    use serde_generate::{typescript, CodeGeneratorConfig, Encoding};
    let mut source = Vec::new();
    let config = CodeGeneratorConfig::new("bincode".into()).with_encodings(vec![Encoding::Bincode]);
    typescript::CodeGenerator::new(&config).output(&mut source, &registry)?;
    std::fs::write(format!("{}/ts/bincode/registry.ts", env!("CARGO_MANIFEST_DIR")), source)?;

	Ok(())
}

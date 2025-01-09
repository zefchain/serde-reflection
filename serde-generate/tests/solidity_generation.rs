// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_generate::{cpp, CodeGeneratorConfig, Encoding};
use std::{collections::BTreeMap, fs::File, io::Write, process::Command};
use tempfile::{tempdir, TempDir};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum SerdeData {
    PrimitiveTypes(PrimitiveTypes),
    OtherTypes(OtherTypes),
    NewTypeVariant(String),
    TupleVariant(u32, u64),
    StructVariant {
        f0: NewTypeStruct,
        f1: TupleStruct,
	f2: Struct,
    },
    TupleArray([u32; 3]),
    SimpleList(SimpleList),
    CStyleEnum(CStyleEnum),
    ComplexMap(BTreeMap<([u32; 2], [u8; 4]), ()>),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PrimitiveTypes {
    f_bool: bool,
    f_u8: u8,
    f_u16: u16,
    f_u32: u32,
    f_u64: u64,
    f_u128: u128,
    f_i8: i8,
    f_i16: i16,
    f_i32: i32,
    f_i64: i64,
    f_i128: i128,
    f_char: Option<char>,
}

pub fn get_solidity_registry() -> serde_reflection::Result<Registry> {
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<SerdeData>(&samples)?;
    tracer.trace_type::<PrimitiveTypes>(&samples)?;
    tracer.registry()
}


fn write_compilation_json(path: &Path, file_name: &str) {
    let mut source = File::create(&path).unwrap();
    writeln!(
        source,
        r#"
{
  "language": "Solidity",
  "sources": {
    "{file_name}": {
      "urls": ["./{file_name}"]
    }
  },
  "settings": {
    "outputSelection": {
      "*": {
        "*": ["evm.bytecode"]
      }
    }
  }
}
"#
    )
    .unwrap();

}


fn test_solidity_compilation(
    config: &CodeGeneratorConfig,
) {
    let registry = get_solidity_registry().unwrap();
    let dir = tempdir().unwrap();
    let test_path = dir.path().join("test.sol");
    let mut test_file = File::create(&test_path).unwrap();

    let generator = cpp::CodeGenerator::new(config);
    generator.output(&mut test_file, &registry).unwrap();

    let config_path = dir.path().join("config.json");
    write_compilation_json(&config_path);
    let config_file = File::open(config_path).unwrap();

    let output_path = dir.path().join("result.json");
    let output_file = File::open(output_path).unwrap();

    let status = Command::new("solc")
        .arg("--standard-json")
        .stdin(Stdio::from(config_file))
        .stdout(Stdio::from(output_file))
        .status()
        .unwrap();
    assert!(status.success());
}

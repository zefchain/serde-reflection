// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils::{NewTypeStruct, OtherTypes, Struct, TupleStruct};
use revm::primitives::Bytes;
use serde::{
    de::DeserializeOwned,
    {Deserialize, Serialize},
};
use serde_generate::{solidity, CodeGeneratorConfig};
use serde_reflection::Samples;
use serde_reflection::{Registry, Tracer, TracerConfig};
use std::path::Path;
use std::{
    collections::BTreeMap,
    fs::File,
    io::Write,
    process::{Command, Stdio},
};
use tempfile::tempdir;

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

pub fn write_compilation_json(path: &Path, file_name: &str) {
    let mut source = File::create(path).unwrap();
    writeln!(
        source,
        r#"
{{
  "language": "Solidity",
  "sources": {{
    "{file_name}": {{
      "urls": ["./{file_name}"]
    }}
  }},
  "settings": {{
    "viaIR": true,
    "outputSelection": {{
      "*": {{
        "*": ["evm.bytecode"]
      }}
    }}
  }}
}}
"#
    )
    .unwrap();
}

pub fn get_bytecode(path: &Path, file_name: &str, contract_name: &str) -> anyhow::Result<Bytes> {
    let config_path = path.join("config.json");
    write_compilation_json(&config_path, file_name);
    let config_file = File::open(config_path)?;

    let output_path = path.join("result.json");
    let output_file = File::create(output_path.clone())?;

    let status = Command::new("solc")
        .current_dir(path)
        .arg("--standard-json")
        .stdin(Stdio::from(config_file))
        .stdout(Stdio::from(output_file))
        .status()?;
    assert!(status.success());

    let contents = std::fs::read_to_string(output_path)?;
    let json_data: serde_json::Value = serde_json::from_str(&contents)?;
    println!("json_data={}", json_data);
    let contracts = json_data
        .get("contracts")
        .ok_or(anyhow::anyhow!("failed to get contract"))?;
    let file_name_contract = contracts
        .get(file_name)
        .ok_or(anyhow::anyhow!("failed to get {file_name}"))?;
    let test_data = file_name_contract
        .get(contract_name)
        .ok_or(anyhow::anyhow!("failed to get test"))?;
    let evm_data = test_data
        .get("evm")
        .ok_or(anyhow::anyhow!("failed to get evm"))?;
    let bytecode = evm_data
        .get("bytecode")
        .ok_or(anyhow::anyhow!("failed to get bytecode"))?;
    let object = bytecode
        .get("object")
        .ok_or(anyhow::anyhow!("failed to get object"))?;
    let object = object.to_string();
    let object = object.trim_matches(|c| c == '"').to_string();
    let object = hex::decode(&object)?;
    Ok(Bytes::copy_from_slice(&object))
}

pub fn get_registry_from_type<T: Serialize + DeserializeOwned>() -> Registry {
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<T>(&samples).unwrap();
    tracer.registry().unwrap()
}

#[test]
fn test_solidity_compilation() {
    let name = "test".to_string();
    let config = CodeGeneratorConfig::new(name);
    let registry = get_solidity_registry().unwrap();
    let dir = tempdir().unwrap();
    let path = dir.path();
    let test_path = path.join("test.sol");
    {
        let mut test_file = File::create(&test_path).unwrap();
        let generator = solidity::CodeGenerator::new(&config);
        generator.output(&mut test_file, &registry).unwrap();
    }

    get_bytecode(path, "test.sol", "test").unwrap();
}

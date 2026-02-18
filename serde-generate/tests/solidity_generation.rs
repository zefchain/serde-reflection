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
    println!("json_data={json_data}");
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

fn generate_solidity(config: &CodeGeneratorConfig, registry: &Registry) -> String {
    let mut output = Vec::new();
    let generator = solidity::CodeGenerator::new(config);
    generator.output(&mut output, registry).unwrap();
    String::from_utf8(output).unwrap()
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

#[test]
fn test_external_definitions_struct() {
    use serde_reflection::{ContainerFormat, Format, Named};

    let mut registry = Registry::new();
    registry.insert(
        "Address".into(),
        ContainerFormat::Struct(vec![Named {
            name: "owner".into(),
            value: Format::TupleArray {
                content: Box::new(Format::U8),
                size: 32,
            },
        }]),
    );
    registry.insert(
        "Payment".into(),
        ContainerFormat::Struct(vec![
            Named {
                name: "recipient".into(),
                value: Format::TypeName("Address".into()),
            },
            Named {
                name: "amount".into(),
                value: Format::U64,
            },
        ]),
    );

    let config = CodeGeneratorConfig::new("ExtTypes".into()).with_external_definitions(
        BTreeMap::from([("BaseTypes".into(), vec!["Address".into()])]),
    );
    let output = generate_solidity(&config, &registry);

    // External type definition should NOT be generated
    assert!(
        !output.contains("struct Address"),
        "should not contain struct Address definition"
    );
    assert!(
        !output.contains("function bcs_serialize_Address"),
        "should not contain Address serializer"
    );
    assert!(
        !output.contains("function bcs_deserialize_offset_Address"),
        "should not contain Address deserializer"
    );

    // Local type should reference external type with module prefix
    assert!(
        output.contains("BaseTypes.Address recipient;"),
        "Payment struct fields should qualify Address with module: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_serialize_Address("),
        "serialization should call qualified function: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_deserialize_offset_Address("),
        "deserialization should call qualified function: {output}"
    );

    // Import statement
    assert!(
        output.contains("import \"BaseTypes.sol\";"),
        "should contain import statement: {output}"
    );

    // Local type definition should still be generated
    assert!(
        output.contains("struct Payment"),
        "should contain local struct Payment: {output}"
    );
}

#[test]
fn test_external_definitions_in_array() {
    use serde_reflection::{ContainerFormat, Format, Named};

    let mut registry = Registry::new();
    registry.insert(
        "Address".into(),
        ContainerFormat::Struct(vec![Named {
            name: "owner".into(),
            value: Format::TupleArray {
                content: Box::new(Format::U8),
                size: 32,
            },
        }]),
    );
    registry.insert(
        "Batch".into(),
        ContainerFormat::Struct(vec![Named {
            name: "recipients".into(),
            value: Format::Seq(Box::new(Format::TypeName("Address".into()))),
        }]),
    );

    let config = CodeGeneratorConfig::new("ExtTypes".into()).with_external_definitions(
        BTreeMap::from([("BaseTypes".into(), vec!["Address".into()])]),
    );
    let output = generate_solidity(&config, &registry);

    // Array field should use qualified type name
    assert!(
        output.contains("BaseTypes.Address[] recipients;"),
        "array field should use qualified inner type: {output}"
    );
    // Seq ser/deser should use qualified function calls
    assert!(
        output.contains("BaseTypes.bcs_serialize_Address("),
        "seq serialization should call qualified function: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_deserialize_offset_Address("),
        "seq deserialization should call qualified function: {output}"
    );
}

#[test]
fn test_external_definitions_in_enum() {
    use serde_reflection::{ContainerFormat, Format, Named, VariantFormat};

    let mut registry = Registry::new();
    registry.insert(
        "Address".into(),
        ContainerFormat::Struct(vec![Named {
            name: "owner".into(),
            value: Format::TupleArray {
                content: Box::new(Format::U8),
                size: 32,
            },
        }]),
    );
    registry.insert(
        "Action".into(),
        ContainerFormat::Enum(BTreeMap::from([
            (
                0,
                Named {
                    name: "Deliver".into(),
                    value: VariantFormat::NewType(Box::new(Format::TypeName("Address".into()))),
                },
            ),
            (
                1,
                Named {
                    name: "Noop".into(),
                    value: VariantFormat::Unit,
                },
            ),
        ])),
    );

    let config = CodeGeneratorConfig::new("ExtTypes".into()).with_external_definitions(
        BTreeMap::from([("BaseTypes".into(), vec!["Address".into()])]),
    );
    let output = generate_solidity(&config, &registry);

    // Enum variant should use qualified type name
    assert!(
        output.contains("BaseTypes.Address deliver;"),
        "enum variant field should use qualified type: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_serialize_Address("),
        "enum serialization should call qualified function: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_deserialize_offset_Address("),
        "enum deserialization should call qualified function: {output}"
    );
}

#[test]
fn test_external_definitions_no_import_when_empty() {
    let registry = get_solidity_registry().unwrap();
    let config = CodeGeneratorConfig::new("test".into());
    let output = generate_solidity(&config, &registry);

    assert!(
        !output.contains("import "),
        "should not have any imports without external definitions: {output}"
    );
}

#[test]
fn test_external_definitions_in_option() {
    use serde_reflection::{ContainerFormat, Format, Named};

    let mut registry = Registry::new();
    registry.insert(
        "Address".into(),
        ContainerFormat::Struct(vec![Named {
            name: "owner".into(),
            value: Format::TupleArray {
                content: Box::new(Format::U8),
                size: 32,
            },
        }]),
    );
    registry.insert(
        "MaybePayee".into(),
        ContainerFormat::Struct(vec![Named {
            name: "recipient".into(),
            value: Format::Option(Box::new(Format::TypeName("Address".into()))),
        }]),
    );

    let config = CodeGeneratorConfig::new("ExtTypes".into()).with_external_definitions(
        BTreeMap::from([("BaseTypes".into(), vec!["Address".into()])]),
    );
    let output = generate_solidity(&config, &registry);

    // External type definition should NOT be generated
    assert!(
        !output.contains("struct Address"),
        "should not contain struct Address definition"
    );

    // Option wrapper should exist locally, but its payload type must be qualified
    assert!(
        output.contains("struct opt_Address"),
        "should contain local opt_Address wrapper: {output}"
    );
    assert!(
        output.contains("BaseTypes.Address value;"),
        "opt_Address.value should be qualified: {output}"
    );

    // Option ser/de should call qualified functions for the external payload type
    assert!(
        output.contains("BaseTypes.bcs_serialize_Address("),
        "option serialization should call qualified function: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_deserialize_offset_Address("),
        "option deserialization should call qualified function: {output}"
    );

    // Import statement
    assert!(
        output.contains("import \"BaseTypes.sol\";"),
        "should contain import statement: {output}"
    );
}

#[test]
fn test_external_definitions_in_tuplearray() {
    use serde_reflection::{ContainerFormat, Format, Named};

    let mut registry = Registry::new();
    registry.insert(
        "Address".into(),
        ContainerFormat::Struct(vec![Named {
            name: "owner".into(),
            value: Format::TupleArray {
                content: Box::new(Format::U8),
                size: 32,
            },
        }]),
    );
    registry.insert(
        "Recipients4".into(),
        ContainerFormat::Struct(vec![Named {
            name: "recipients".into(),
            value: Format::TupleArray {
                content: Box::new(Format::TypeName("Address".into())),
                size: 4,
            },
        }]),
    );

    let config = CodeGeneratorConfig::new("ExtTypes".into()).with_external_definitions(
        BTreeMap::from([("BaseTypes".into(), vec!["Address".into()])]),
    );
    let output = generate_solidity(&config, &registry);

    // The tuplearray helper struct name is tuplearray{size}_{inner_key}
    assert!(
        output.contains("struct tuplearray4_Address"),
        "should contain tuplearray4_Address helper struct: {output}"
    );

    // The tuplearray helper payload array type must be qualified
    assert!(
        output.contains("BaseTypes.Address[] values;"),
        "tuplearray values type should be qualified: {output}"
    );

    // Serialization/deserialization loops should call qualified functions
    assert!(
        output.contains("BaseTypes.bcs_serialize_Address("),
        "tuplearray serialization should call qualified function: {output}"
    );
    assert!(
        output.contains("BaseTypes.bcs_deserialize_offset_Address("),
        "tuplearray deserialization should call qualified function: {output}"
    );
}

/// Types reachable *only* through external types should not be emitted locally.
/// This validates the core purpose of `locally_needed_types()`.
#[test]
fn test_external_definitions_excludes_transitive_only_through_external() {
    use serde_reflection::{ContainerFormat, Format, Named};

    let mut registry = Registry::new();

    // AddressInner exists in the registry and is a dependency of Address.
    registry.insert(
        "AddressInner".into(),
        ContainerFormat::Struct(vec![Named {
            name: "bytes".into(),
            value: Format::TupleArray {
                content: Box::new(Format::U8),
                size: 32,
            },
        }]),
    );

    // Address depends on AddressInner.
    registry.insert(
        "Address".into(),
        ContainerFormat::Struct(vec![Named {
            name: "inner".into(),
            value: Format::TypeName("AddressInner".into()),
        }]),
    );

    // Local type uses Address.
    registry.insert(
        "Payment".into(),
        ContainerFormat::Struct(vec![Named {
            name: "recipient".into(),
            value: Format::TypeName("Address".into()),
        }]),
    );

    // Mark Address as external, but NOT AddressInner.
    // AddressInner should not be emitted locally because it is only
    // reachable via the external Address type.
    let config = CodeGeneratorConfig::new("ExtTypes".into()).with_external_definitions(
        BTreeMap::from([("BaseTypes".into(), vec!["Address".into()])]),
    );
    let output = generate_solidity(&config, &registry);

    // External Address should not be generated.
    assert!(
        !output.contains("struct Address"),
        "should not contain struct Address definition"
    );

    // AddressInner should ALSO not be generated (reachable only through external Address).
    assert!(
        !output.contains("struct AddressInner"),
        "should not contain struct AddressInner definition: {output}"
    );
    assert!(
        !output.contains("function bcs_serialize_AddressInner"),
        "should not contain AddressInner serializer: {output}"
    );
    assert!(
        !output.contains("function bcs_deserialize_offset_AddressInner"),
        "should not contain AddressInner deserializer: {output}"
    );

    // Local Payment should still be generated and reference qualified external Address.
    assert!(
        output.contains("struct Payment"),
        "should contain local struct Payment: {output}"
    );
    assert!(
        output.contains("BaseTypes.Address recipient;"),
        "Payment should qualify Address with module: {output}"
    );

    // Import statement should exist.
    assert!(
        output.contains("import \"BaseTypes.sol\";"),
        "should contain import statement: {output}"
    );
}

#[test]
#[should_panic(expected = "not a valid Solidity identifier")]
fn test_external_definitions_rejects_invalid_module_name() {
    use serde_reflection::{ContainerFormat, Format, Named};

    let mut registry = Registry::new();
    registry.insert(
        "Foo".into(),
        ContainerFormat::Struct(vec![Named {
            name: "x".into(),
            value: Format::U64,
        }]),
    );

    // "path/to/module" is not a valid Solidity identifier
    let config = CodeGeneratorConfig::new("Test".into()).with_external_definitions(BTreeMap::from(
        [("path/to/module".into(), vec!["Foo".into()])],
    ));
    let _ = generate_solidity(&config, &registry);
}

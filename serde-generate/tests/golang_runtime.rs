// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use heck::CamelCase;
use serde_generate::{golang, CodeGeneratorConfig};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

#[test]
fn test_golang_runtime_autotests() {
    let runtime_mod_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("../../../serde-generate/runtime/golang");

    let status = Command::new("go")
        .current_dir(runtime_mod_path.to_str().unwrap())
        .arg("test")
        .arg("./...")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_golang_bcs_runtime_on_simple_data() {
    test_golang_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_golang_bincode_runtime_on_simple_data() {
    test_golang_runtime_on_simple_data(Runtime::Bincode);
}

fn test_golang_runtime_on_simple_data(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.go");
    let mut source = File::create(&source_path).unwrap();

    let config = CodeGeneratorConfig::new("main".to_string())
        .with_encodings(vec![runtime.into()])
        .with_external_definitions(
            vec![("github.com/google/go-cmp/cmp".to_string(), vec![])]
                .into_iter()
                .collect(),
        );
    let generator = golang::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    writeln!(
        source,
        r#"
func main() {{
	input := []byte{{{0}}}
	value, err := {1}DeserializeTest(input)
	if err != nil {{ panic("failed to deserialize") }}

        value2 := Test {{
		A: []uint32{{ 4, 6 }},
		B: struct {{ Field0 int64; Field1 uint64 }} {{ -3, 5 }},
		C: &Choice__C {{ X: 7 }},
	}}
	if !cmp.Equal(value, value2) {{ panic("value != value2") }}

	output, err := value2.{1}Serialize()
	if err != nil {{ panic("failed to serialize") }}
	if !cmp.Equal(input, output) {{ panic("input != output") }}

	input2 := []byte{{{0}, 1}}
	value2, err2 := {1}DeserializeTest(input2)
	if err2 == nil {{ panic("was expecting an error") }}
}}
"#,
        reference
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", "),
        runtime.name().to_camel_case(),
    )
    .unwrap();

    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("mod")
        .arg("init")
        .arg("testing")
        .status()
        .unwrap();
    assert!(status.success());

    let runtime_mod_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("../../../serde-generate/runtime/golang");
    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("mod")
        .arg("edit")
        .arg("-replace")
        .arg(format!(
            "github.com/novifinancial/serde-reflection/serde-generate/runtime/golang={}",
            runtime_mod_path.to_str().unwrap()
        ))
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("run")
        .arg("-mod=mod")
        .arg(&source_path)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_golang_bcs_runtime_on_supported_types() {
    test_golang_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_golang_bincode_runtime_on_supported_types() {
    test_golang_runtime_on_supported_types(Runtime::Bincode);
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "{{{}}}",
        bytes
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn test_golang_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.go");
    let mut source = File::create(&source_path).unwrap();

    let config = CodeGeneratorConfig::new("main".to_string())
        .with_encodings(vec![runtime.into()])
        .with_external_definitions(
            vec![("github.com/google/go-cmp/cmp".to_string(), vec![])]
                .into_iter()
                .collect(),
        );
    let generator = golang::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let positive_encodings = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join(", ");

    let negative_encodings = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(
        source,
        r#"
func main() {{
	positive_inputs := [][]byte{{{0}}}
	negative_inputs := [][]byte{{{1}}}

	for _, input := range(positive_inputs) {{
		value, err := {2}DeserializeSerdeData(input)
		if err != nil {{ panic(fmt.Sprintf("failed to deserialize input: %v", err)) }}
		output, err := value.{2}Serialize()
		if err != nil {{ panic(fmt.Sprintf("failed to serialize: %v", err)) }}
		if !cmp.Equal(input, output) {{ panic(fmt.Sprintf("input != output:\n  %v\n  %v", input, output)) }}

		// Test self-equality for the Serde value.
		{{
			value2, err := {2}DeserializeSerdeData(input)
			if err != nil {{ panic(fmt.Sprintf("failed to deserialize input: %v", err)) }}
			if !cmp.Equal(value, value2) {{ panic("Value should test equal to itself.") }}
		}}

		// Test simple mutations of the input.
		for i := 0; i < len(input); i++ {{
			input2 := make([]byte, len(input))
			copy(input2, input)
			input2[i] ^= 0x80
			value2, err := {2}DeserializeSerdeData(input2)
			if err != nil {{ continue }}
			if cmp.Equal(value, value2) {{ panic("Modified input should give a different value.") }}
		}}
	}}

	for _, input := range(negative_inputs) {{
		_, err := {2}DeserializeSerdeData(input)
		if err == nil {{ panic(fmt.Sprintf("Input should fail to deserialize: %v", input)) }}
	}}
}}
"#,
        positive_encodings,
        negative_encodings,
        runtime.name().to_camel_case(),
    )
    .unwrap();

    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("mod")
        .arg("init")
        .arg("testing")
        .status()
        .unwrap();
    assert!(status.success());

    let runtime_mod_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("../../../serde-generate/runtime/golang");
    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("mod")
        .arg("edit")
        .arg("-replace")
        .arg(format!(
            "github.com/novifinancial/serde-reflection/serde-generate/runtime/golang={}",
            runtime_mod_path.to_str().unwrap()
        ))
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("run")
        .arg("-mod=mod")
        .arg(source_path)
        .status()
        .unwrap();
    assert!(status.success());
}

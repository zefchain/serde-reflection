// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde::{Deserialize, Serialize};
use serde_generate::{golang, CodeGeneratorConfig, Encoding};
use serde_reflection::{Registry, Result, Samples, Tracer, TracerConfig};
use std::{collections::BTreeMap, fs::File, io::Write, process::Command};
use tempfile::{tempdir, TempDir};

#[derive(Serialize, Deserialize)]
struct Test {
    a: Vec<u32>,
}

fn get_small_registry() -> Result<Registry> {
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<Test>(&samples)?;
    tracer.registry()
}

fn get_empty_registry() -> Result<Registry> {
    let tracer = Tracer::new(TracerConfig::default());
    tracer.registry()
}

fn test_that_golang_code_compiles_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    test_that_golang_code_compiles_with_config_and_registry(config, &get_empty_registry().unwrap());
    test_that_golang_code_compiles_with_config_and_registry(config, &get_small_registry().unwrap());
    test_that_golang_code_compiles_with_config_and_registry(
        config,
        &test_utils::get_registry().unwrap(),
    )
}

fn test_that_golang_code_compiles_with_config_and_registry(
    config: &CodeGeneratorConfig,
    registry: &Registry,
) -> (TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.go");
    let mut source = File::create(&source_path).unwrap();

    let generator = golang::CodeGenerator::new(config);
    generator.output(&mut source, registry).unwrap();

    writeln!(&mut source, "func main() {{}}").unwrap();

    let status = Command::new("go")
        .current_dir(dir.path())
        .arg("mod")
        .arg("init")
        .arg("example.com/test")
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
        .arg("build")
        .arg("-mod=mod")
        .arg(&source_path)
        .status()
        .unwrap();
    assert!(status.success());

    (dir, source_path)
}

#[test]
fn test_that_golang_code_compiles() {
    let config = CodeGeneratorConfig::new("main".to_string());
    test_that_golang_code_compiles_with_config(&config);
}

#[test]
fn test_that_golang_code_compiles_without_serialization() {
    let config = CodeGeneratorConfig::new("main".to_string()).with_serialization(false);
    test_that_golang_code_compiles_with_config(&config);
}

#[test]
fn test_that_golang_code_compiles_with_bcs() {
    let config = CodeGeneratorConfig::new("main".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_golang_code_compiles_with_config(&config);
}

#[test]
fn test_that_golang_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("main".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_golang_code_compiles_with_config(&config);
}

#[test]
fn test_that_golang_code_compiles_with_comments() {
    let comments = vec![
        (
            vec!["main".to_string(), "SerdeData".to_string()],
            "Some\ncomments".to_string(),
        ),
        (
            vec!["main".to_string(), "List".to_string(), "Node".to_string()],
            "Some other comments".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("main".to_string()).with_comments(comments);

    let (_dir, source_path) = test_that_golang_code_compiles_with_config(&config);
    // Comments were correctly generated.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains(
        r#"
// Some
// comments
"#
    ));
    assert!(content.contains(
        r#"
// Some other comments
"#
    ));
}

#[test]
fn test_golang_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.go");
    let mut source = File::create(&source_path).unwrap();

    // Pretend that "Tree" is external.
    let mut definitions = BTreeMap::new();
    definitions.insert("foo".to_string(), vec!["Tree".to_string()]);
    let config =
        CodeGeneratorConfig::new("main".to_string()).with_external_definitions(definitions);
    let generator = golang::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    // References were updated.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("foo.Tree"));
}

#[test]
fn test_that_golang_code_compiles_with_custom_code() {
    let custom_code = vec![
        (
            vec!["main".to_string(), "SerdeData".to_string()],
            "// custom1".to_string(),
        ),
        (
            vec!["main".to_string(), "List".to_string(), "Node".to_string()],
            "// custom2".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("main".to_string()).with_custom_code(custom_code);

    let (_dir, source_path) = test_that_golang_code_compiles_with_config(&config);
    // Comments were correctly generated.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("// custom1"));
    assert!(content.contains("// custom2"));
}

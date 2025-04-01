// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde::{Deserialize, Serialize};
use serde_generate::{swift, CodeGeneratorConfig, Encoding};
use serde_reflection::{Registry, Result, Samples, Tracer, TracerConfig};
use std::{collections::BTreeMap, fs::File, io::Write, process::Command, sync::Mutex};
use tempfile::{tempdir, TempDir};

lazy_static::lazy_static! {
    // Avoid interleaving compiler calls because the output gets very messy.
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

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

fn test_that_swift_code_compiles_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    test_that_swift_code_compiles_with_config_and_registry(config, &get_small_registry().unwrap());
    test_that_swift_code_compiles_with_config_and_registry(
        config,
        &test_utils::get_registry().unwrap(),
    )
}

fn test_that_swift_code_compiles_with_config_and_registry(
    config: &CodeGeneratorConfig,
    registry: &Registry,
) -> (TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Sources/Testing")).unwrap_or(());
    let serde_package_path = std::env::current_dir()
        .unwrap()
        .join("../serde-generate/runtime/swift");
    let mut file = File::create(dir.path().join("Package.swift")).unwrap();
    write!(
        file,
        r#"// swift-tools-version:5.3

import PackageDescription

let package = Package(
    name: "Testing",
    products: [
        .library(
            name: "Testing",
            targets: ["Testing"]),
    ],
    dependencies: [
        .package(name: "Serde", path: "{}"),
    ],
    targets: [
        .target(
            name: "Testing",
            dependencies: ["Serde"]),
    ]
)
"#,
        serde_package_path.to_str().unwrap()
    )
    .unwrap();

    let source_path = dir.path().join("Sources/Testing/Testing.swift");
    let mut source = File::create(&source_path).unwrap();

    let generator = swift::CodeGenerator::new(config);
    generator.output(&mut source, registry).unwrap();

    {
        let _lock = MUTEX.lock();
        let status = Command::new("swift")
            .current_dir(dir.path())
            .arg("build")
            .status()
            .unwrap();
        assert!(status.success());
    }

    (dir, source_path)
}

#[test]
fn test_that_swift_code_compiles() {
    let config = CodeGeneratorConfig::new("Testing".to_string());
    test_that_swift_code_compiles_with_config(&config);
}

#[test]
fn test_that_swift_code_compiles_without_serialization() {
    let config = CodeGeneratorConfig::new("Testing".to_string()).with_serialization(false);
    test_that_swift_code_compiles_with_config(&config);
}

#[test]
fn test_that_swift_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("Testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_swift_code_compiles_with_config(&config);
}

#[test]
fn test_that_swift_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("Testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_swift_code_compiles_with_config(&config);
}

#[test]
fn test_that_swift_code_compiles_with_comments() {
    let comments = vec![
        (
            vec!["Testing".to_string(), "SerdeData".to_string()],
            "Some\ncomments".to_string(),
        ),
        (
            vec![
                "Testing".to_string(),
                "List".to_string(),
                "Node".to_string(),
            ],
            "Some other comments".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("Testing".to_string()).with_comments(comments);

    let (_dir, source_path) = test_that_swift_code_compiles_with_config(&config);
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
fn test_swift_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("Testing.swift");
    let mut source = File::create(&source_path).unwrap();

    // Pretend that "Tree" is external.
    let mut definitions = BTreeMap::new();
    definitions.insert("foo".to_string(), vec!["Tree".to_string()]);
    let config =
        CodeGeneratorConfig::new("Testing".to_string()).with_external_definitions(definitions);
    let generator = swift::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    // References were updated.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("foo.Tree"));
}

#[test]
fn test_that_swift_code_compiles_with_custom_code() {
    let custom_code = vec![(
        vec!["Testing".to_string(), "SerdeData".to_string()],
        "// custom1".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("Testing".to_string()).with_custom_code(custom_code);

    let (_dir, source_path) = test_that_swift_code_compiles_with_config(&config);
    // Comments were correctly generated.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("// custom1"));
}

#[test]
fn test_that_swift_code_follow_case_convention() {
    let config = CodeGeneratorConfig::new("Testing".to_string());

    let (_dir, source_path) = test_that_swift_code_compiles_with_config(&config);
    // Case convention were correctly followed.
    let content = std::fs::read_to_string(source_path).unwrap();

    // Enum variants are `lowerCamelCase`.
    assert!(content.contains(r#"case primitiveTypes"#));
    assert!(!content.contains(r#"case PrimitiveTypes"#));
    assert!(!content.contains(r#"case primitive_ypes"#));
    // Field names are `lowerCamelCase`.
    assert!(content.contains(r#"@Indirect public var fBool: Bool"#));
    assert!(!content.contains(r#"@Indirect public var f_bool: Bool"#));
}

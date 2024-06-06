// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{rust, CodeGeneratorConfig};
use std::{fs::File, io::Write, process::Command};
use tempfile::{tempdir, TempDir};

// Quick test using rustc directly.
fn test_that_rust_code_compiles_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.rs");
    let mut source = File::create(&source_path).unwrap();

    let generator = rust::CodeGenerator::new(config);
    generator.output(&mut source, &registry).unwrap();

    let status = Command::new("rustc")
        .current_dir(dir.path())
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2018")
        .arg(&source_path)
        .status()
        .unwrap();
    assert!(status.success());

    (dir, source_path)
}

#[test]
fn test_that_rust_code_compiles() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    test_that_rust_code_compiles_with_config(&config);
}

#[test]
fn test_that_rust_code_compiles_with_comments() {
    let comments = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "Some\ncomments".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_serialization(false)
        .with_comments(comments);
    let (_dir, source_path) = test_that_rust_code_compiles_with_config(&config);
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("/// Some\n/// comments\n"));
}

#[test]
fn test_that_rust_code_compiles_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.rs");
    let mut source = File::create(&source_path).unwrap();

    let definitions = vec![
        ("foo".to_string(), vec!["Map".to_string()]),
        (String::new(), vec!["Bytes".into()]),
    ]
    .into_iter()
    .collect();

    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_external_definitions(definitions)
        .with_serialization(false);
    let generator = rust::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let output = Command::new("rustc")
        .current_dir(dir.path())
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2018")
        .arg(&source_path)
        .output()
        .unwrap();
    assert!(!output.status.success()); // Must fail.

    // Externally defined names "Map" and "Bytes" have caused the usual imports to be
    // replaced by `use foo::Map` (and nothing, respectively), so we must add the definitions.
    writeln!(
        &mut source,
        r#"
type Bytes = Vec<u8>;

mod foo {{
    pub type Map<K, V> = std::collections::BTreeMap<K, V>;
}}
"#
    )
    .unwrap();

    let status = Command::new("rustc")
        .current_dir(dir.path())
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2018")
        .arg(&source_path)
        .status()
        .unwrap();
    assert!(status.success());
}

// Full test using cargo. This may take a while.
#[test]
fn test_that_rust_code_compiles_with_serialization() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "testing"
version = "0.1.0"
edition = "2018"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_bytes = "0.11"

[workspace]
"#,
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();

    let config = CodeGeneratorConfig::new("testing".to_string());
    let generator = rust::CodeGenerator::new(&config);

    let source_path = dir.path().join("src/lib.rs");
    let mut source = File::create(source_path).unwrap();
    generator.output(&mut source, &registry).unwrap();

    // Use a stable `target` dir to avoid downloading and recompiling crates everytime.
    let target_dir = std::env::current_dir().unwrap().join("../target");
    let status = Command::new("cargo")
        .current_dir(dir.path())
        .arg("build")
        .arg("--target-dir")
        .arg(target_dir)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_that_rust_code_compiles_with_custom_code() {
    let custom_code = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "// custom code\n".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_serialization(false)
        .with_custom_code(custom_code);
    let (_dir, source_path) = test_that_rust_code_compiles_with_config(&config);
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("// custom code\n"));
}

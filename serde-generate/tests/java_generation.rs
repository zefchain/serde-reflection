// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{java, CodeGeneratorConfig, Encoding};
use std::{collections::BTreeMap, process::Command};
use tempfile::{tempdir, TempDir};

fn test_that_java_code_compiles_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let generator = java::CodeGenerator::new(config);
    generator
        .write_source_files(dir.path().to_path_buf(), &registry)
        .unwrap();

    let paths = std::iter::empty()
        .chain(std::fs::read_dir("runtime/java/com/novi/serde").unwrap())
        .chain(std::fs::read_dir("runtime/java/com/novi/bincode").unwrap())
        .chain(std::fs::read_dir("runtime/java/com/novi/bcs").unwrap())
        .chain(std::fs::read_dir(dir.path().join("testing")).unwrap())
        .map(|e| e.unwrap().path());
    let status = Command::new("javac")
        .arg("-Xlint")
        .arg("-d")
        .arg(dir.path())
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success());

    let path = dir.path().join("testing");
    (dir, path)
}

#[test]
fn test_that_java_code_compiles() {
    let config = CodeGeneratorConfig::new("testing".to_string());
    test_that_java_code_compiles_with_config(&config);
}

#[test]
fn test_that_java_code_compiles_without_serialization() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    test_that_java_code_compiles_with_config(&config);
}

#[test]
fn test_that_java_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_java_code_compiles_with_config(&config);
}

#[test]
fn test_that_java_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_java_code_compiles_with_config(&config);
}

#[test]
fn test_that_java_code_compiles_with_comments() {
    let comments = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "Some\ncomments".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string()).with_comments(comments);

    let (_dir, path) = test_that_java_code_compiles_with_config(&config);

    // Comment was correctly generated.
    let content = std::fs::read_to_string(path.join("SerdeData.java")).unwrap();
    assert!(content.contains(
        r#"
/**
 * Some
 * comments
 */
"#
    ));
}

#[test]
fn test_java_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    // (wrongly) Declare TraitHelpers as external.
    let mut definitions = BTreeMap::new();
    definitions.insert("foo".to_string(), vec!["TraitHelpers".to_string()]);
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_external_definitions(definitions);
    let generator = java::CodeGenerator::new(&config);

    generator
        .write_source_files(dir.path().to_path_buf(), &registry)
        .unwrap();

    // References were updated.
    let content = std::fs::read_to_string(dir.path().join("testing/SerdeData.java")).unwrap();
    assert!(content.contains("foo.TraitHelpers."));
}

#[test]
fn test_that_java_code_compiles_with_custom_code() {
    let comments = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "SerdeData me() {{ return this; }}".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string()).with_comments(comments);

    let (_dir, path) = test_that_java_code_compiles_with_config(&config);

    // Comment was correctly generated.
    let content = std::fs::read_to_string(path.join("SerdeData.java")).unwrap();
    assert!(content.contains("me()"));
}

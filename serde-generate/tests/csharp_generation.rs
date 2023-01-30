// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{csharp, CodeGeneratorConfig, Encoding};
use std::{collections::BTreeMap, process::Command, sync::Mutex};
use tempfile::{tempdir, TempDir};

lazy_static::lazy_static! {
    // `dotnet build` spuriously fails on linux if run concurrently
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

fn test_that_csharp_code_compiles_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    use serde_generate::SourceInstaller;

    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_path_buf();

    let installer = csharp::Installer::new(dir_path.clone());
    installer.install_module(config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();

    let proj_path = dir_path.join(config.module_name().replace('.', "/"));
    {
        let _lock = MUTEX.lock();
        let status = Command::new("dotnet")
            .arg("build")
            .current_dir(&proj_path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    (dir, proj_path)
}

#[test]
fn test_that_csharp_code_compiles() {
    let config = CodeGeneratorConfig::new("Generated".to_string());
    test_that_csharp_code_compiles_with_config(&config);
}

#[test]
fn test_that_csharp_code_compiles_without_serialization() {
    let config = CodeGeneratorConfig::new("Generated".to_string()).with_serialization(false);
    test_that_csharp_code_compiles_with_config(&config);
}

#[test]
fn test_that_csharp_code_compiles_with_c_style_enums() {
    let config = CodeGeneratorConfig::new("Generated".to_string()).with_c_style_enums(true);
    test_that_csharp_code_compiles_with_config(&config);
}

#[test]
fn test_that_csharp_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("Generated".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_csharp_code_compiles_with_config(&config);
}

#[test]
fn test_that_csharp_code_compiles_with_bcs_and_extra_nesting() {
    let config = CodeGeneratorConfig::new("My.Generated.Project".to_string())
        .with_encodings(vec![Encoding::Bcs]);
    test_that_csharp_code_compiles_with_config(&config);
}

#[test]
fn test_that_csharp_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("Generated".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_csharp_code_compiles_with_config(&config);
}

#[test]
fn test_that_csharp_code_compiles_with_comments() {
    let comments = vec![(
        vec!["Generated".to_string(), "SerdeData".to_string()],
        "Some\ncomments".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("Generated".to_string()).with_comments(comments);

    let (_dir, path) = test_that_csharp_code_compiles_with_config(&config);
    let content = std::fs::read_to_string(path.join("SerdeData.cs")).unwrap();
    assert!(content.contains("/// Some\n    /// comments"));
}

#[test]
fn test_csharp_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    // (wrongly) Declare TraitHelpers as external.
    let mut definitions = BTreeMap::new();
    definitions.insert("foo".to_string(), vec!["TraitHelpers".to_string()]);
    let config =
        CodeGeneratorConfig::new("Generated".to_string()).with_external_definitions(definitions);
    let generator = csharp::CodeGenerator::new(&config);

    generator
        .write_source_files(dir.path().to_path_buf(), &registry)
        .unwrap();

    // References were updated.
    let content = std::fs::read_to_string(dir.path().join("Generated/SerdeData.cs")).unwrap();
    assert!(content.contains("foo.TraitHelpers."));
}

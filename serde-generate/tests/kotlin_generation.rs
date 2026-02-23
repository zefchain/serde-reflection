// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{kotlin, CodeGeneratorConfig, Encoding, SourceInstaller};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::{tempdir, TempDir};
use which::which;

fn test_that_kotlin_code_compiles_with_config(config: &CodeGeneratorConfig) -> (TempDir, PathBuf) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let installer = kotlin::Installer::new(dir.path().to_path_buf());
    installer.install_module(config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();

    maybe_compile_kotlin(dir.path());

    let path = module_path(dir.path(), config.module_name());
    (dir, path)
}

#[test]
fn test_that_kotlin_code_compiles() {
    let config = CodeGeneratorConfig::new("testing".to_string());
    test_that_kotlin_code_compiles_with_config(&config);
}

#[test]
fn test_that_kotlin_code_compiles_without_serialization() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    test_that_kotlin_code_compiles_with_config(&config);
}

#[test]
fn test_that_kotlin_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_kotlin_code_compiles_with_config(&config);
}

#[test]
fn test_that_kotlin_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_kotlin_code_compiles_with_config(&config);
}

#[test]
fn test_that_kotlin_code_compiles_with_comments() {
    let comments = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "Some\ncomments".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string()).with_comments(comments);

    let (_dir, path) = test_that_kotlin_code_compiles_with_config(&config);

    let content = std::fs::read_to_string(path.join("SerdeData.kt")).unwrap();
    assert!(content.contains(
        r#"
// Some
// comments
"#
    ));
}

#[test]
fn test_kotlin_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let mut definitions = BTreeMap::new();
    definitions.insert("foo".to_string(), vec!["Tree".to_string()]);
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_external_definitions(definitions);

    let installer = kotlin::Installer::new(dir.path().to_path_buf());
    installer.install_module(&config, &registry).unwrap();

    let path = module_path(dir.path(), config.module_name());
    let content = std::fs::read_to_string(path.join("SerdeData.kt")).unwrap();
    assert!(content.contains("foo.Tree"));
}

#[test]
fn test_that_kotlin_code_compiles_with_custom_code() {
    let custom_code = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "fun me(): SerdeData { return this }".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string()).with_custom_code(custom_code);

    let (_dir, path) = test_that_kotlin_code_compiles_with_config(&config);

    let content = std::fs::read_to_string(path.join("SerdeData.kt")).unwrap();
    assert!(content.contains("fun me(): SerdeData"));
}

fn find_kotlin_compiler() -> Option<PathBuf> {
    which("kotlinc-native").ok()
}

fn collect_kotlin_sources(root: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_kotlin_sources(&path, output)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("kt") {
            output.push(path);
        }
    }
    Ok(())
}

fn maybe_compile_kotlin(dir: &Path) {
    let compiler = match find_kotlin_compiler() {
        Some(path) => {
            println!("Kotlin/Native compiler found: {}", path.display());
            path
        }
        None => {
            eprintln!("Skipping Kotlin/Native compilation test: compiler not found");
            return;
        }
    };

    let mut sources = Vec::new();
    collect_kotlin_sources(dir, &mut sources).unwrap();

    let output_path = dir.join("kotlin_generation_test");
    let mut args = vec![
        "-produce".to_string(),
        "library".to_string(),
        "-o".to_string(),
        output_path.to_str().unwrap().to_string(),
    ];
    for source in &sources {
        args.push(source.to_str().unwrap().to_string());
    }

    let output = Command::new(compiler).args(&args).output().unwrap();
    if !output.status.success() {
        eprintln!(
            "Kotlin compile stdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        eprintln!(
            "Kotlin compile stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(output.status.success());
}

fn module_path(base: &Path, module_name: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in module_name.split('.') {
        path = path.join(part);
    }
    path
}

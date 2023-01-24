// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_generate::{dart, test_utils, CodeGeneratorConfig, Encoding, SourceInstaller};
use std::fs::read_to_string;
use std::{
    io::Result,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir;

fn install_test_dependencies(path: &Path) -> Result<()> {
    Command::new("dart")
        .current_dir(path)
        .env("PUB_CACHE", "../.pub-cache")
        .args(["pub", "add", "-d", "test"])
        .status()?;

    Ok(())
}

fn generate_with_config(source_path: PathBuf, config: &CodeGeneratorConfig) -> PathBuf {
    let registry = test_utils::get_registry().unwrap();

    let installer = dart::Installer::new(source_path.clone());
    installer.install_module(config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();

    install_test_dependencies(&source_path).unwrap();

    let dart_analyze = Command::new("dart")
        .current_dir(&source_path)
        .env("PUB_CACHE", "../.pub-cache")
        .args(["analyze"])
        .status()
        .unwrap();

    assert!(
        dart_analyze.success(),
        "Generated Dart source code did not pass `dart analyze`"
    );

    source_path
}

#[test]
fn test_dart_code_compiles() {
    let source_path = tempdir().unwrap().path().join("dart_basic_project");

    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode])
        .with_c_style_enums(true);

    generate_with_config(source_path, &config);
}

#[test]
fn test_dart_code_compiles_with_comments() {
    let source_path = tempdir().unwrap().path().join("dart_comment_project");

    let comments = vec![(
        vec!["example".to_string(), "SerdeData".to_string()],
        "Some\ncomments".to_string(),
    )]
    .into_iter()
    .collect();

    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![Encoding::Bincode])
        .with_c_style_enums(true)
        .with_comments(comments);

    let path = generate_with_config(source_path, &config);

    // Comment was correctly generated.
    let content = std::fs::read_to_string(
        path.join("lib")
            .join("src")
            .join(config.module_name())
            .join("serde_data.dart"),
    )
    .unwrap();

    assert!(content.contains(
        r#"
/// Some
/// comments
"#
    ));
}

#[test]
fn test_dart_code_compiles_with_class_enums() {
    let source_path = tempdir().unwrap().path().join("dart_enum_project");

    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode])
        .with_c_style_enums(false);

    generate_with_config(source_path, &config);
}

#[test]
fn test_dart_code_compiles_class_enums_for_complex_enums() {
    let source_path = tempdir().unwrap().path().join("dart_class_enum_project");

    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode])
        // we enable native Dart enums to test that complex Rust enums will still produce Dart classes
        .with_c_style_enums(true);

    generate_with_config(source_path.clone(), &config);

    let generated_c_style =
        read_to_string(&source_path.join("lib/src/example/c_style_enum.dart")).unwrap();
    let generated_class_style =
        read_to_string(&source_path.join("lib/src/example/list.dart")).unwrap();

    assert!(generated_c_style.contains("enum CStyleEnum {"));
    assert!(generated_class_style.contains("abstract class List_ {"));
}

#[test]
fn test_dart_code_includes_getters_for_shared_properties_of_complex_enums() {
    let source_path = tempdir()
        .unwrap()
        .path()
        .join("dart_class_enum_shared_properties_project");

    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode])
        // we enable native Dart enums to test that complex Rust enums will still produce Dart classes
        .with_c_style_enums(true);

    generate_with_config(source_path.clone(), &config);

    let generated_class_style =
        read_to_string(&source_path.join("lib/src/example/complex_enum.dart")).unwrap();

    assert!(generated_class_style.contains("String get id;\n"));
    assert!(!generated_class_style.contains("String get value;\n"));
    assert!(!generated_class_style.contains("int get value;\n"));
    assert!(!generated_class_style.contains("bool get value;\n"));
    assert!(!generated_class_style.contains("String get a;\n"));
    assert!(!generated_class_style.contains("String get b;\n"));
    assert!(!generated_class_style.contains("String get c;\n"));
}

// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{python3, CodeGeneratorConfig, Encoding, SourceInstaller};
use std::{collections::BTreeMap, fs::File, process::Command};
use tempfile::{tempdir, TempDir};

fn test_that_python_code_parses_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let source_path = dir.path().join("test.py");
    let mut source = File::create(&source_path).unwrap();

    let generator = python3::CodeGenerator::new(config);
    generator.output(&mut source, &registry).unwrap();

    let python_path = format!(
        "{}:runtime/python",
        std::env::var("PYTHONPATH").unwrap_or_default()
    );
    let status = Command::new("python3")
        .arg(&source_path)
        .env("PYTHONPATH", python_path)
        .status()
        .unwrap();
    assert!(status.success());

    (dir, source_path)
}

#[test]
fn test_that_python_code_parses() {
    let config = CodeGeneratorConfig::new("testing".to_string());
    test_that_python_code_parses_with_config(&config);
}

#[test]
fn test_that_python_code_parses_without_serialization() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    test_that_python_code_parses_with_config(&config);
}

#[test]
fn test_that_python_code_parses_with_bcs() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_python_code_parses_with_config(&config);
}

#[test]
fn test_that_python_code_parses_with_bincode() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_python_code_parses_with_config(&config);
}

#[test]
fn test_that_python_code_parses_with_comments() {
    let comments = vec![
        (
            vec!["testing".to_string(), "SerdeData".to_string()],
            "Some\ncomments".to_string(),
        ),
        (
            vec![
                "testing".to_string(),
                "List".to_string(),
                "Node".to_string(),
            ],
            "Some other comments".to_string(),
        ),
    ]
    .into_iter()
    .collect();

    let config = CodeGeneratorConfig::new("testing".to_string()).with_comments(comments);
    let (_dir, source_path) = test_that_python_code_parses_with_config(&config);

    // Check that comments were correctly generated.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains(
        r#"
    """Some
    comments
    """
"#
    ));
    assert!(content.contains(
        r#"
    """Some other comments
    """
"#
    ));
}

#[test]
fn test_python_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let source_path = dir.path().join("test.py");
    let mut source = File::create(&source_path).unwrap();

    // Pretend that "Tree" is external.
    let mut definitions = BTreeMap::new();
    definitions.insert("pkg.foo".to_string(), vec!["Tree".to_string()]);
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_external_definitions(definitions);
    let generator = python3::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let content = std::fs::read_to_string(&source_path).unwrap();
    assert!(content.contains("from pkg import foo"));
    assert!(content.contains("value: foo.Tree"));
    assert!(!content.contains("value: Tree"));
}

#[test]
fn test_that_python_code_parses_with_custom_code() {
    let custom_code = vec![
        (
            vec!["testing".to_string(), "SerdeData".to_string()],
            "def nothing1(self):\n    pass".to_string(),
        ),
        (
            vec![
                "testing".to_string(),
                "List".to_string(),
                "Node".to_string(),
            ],
            "def nothing2(self):\n    pass".to_string(),
        ),
    ]
    .into_iter()
    .collect();

    let config = CodeGeneratorConfig::new("testing".to_string()).with_custom_code(custom_code);
    let (_dir, source_path) = test_that_python_code_parses_with_config(&config);

    // Check that custom_code was added.
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("nothing1"));
    assert!(content.contains("nothing2"));
}

#[test]
fn test_that_installed_python_code_passes_pyre_check() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    let installer = python3::Installer::new(dir.path().join("src"), /* serde package */ None);
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();

    // Copy test files manually to type-check them as well.
    // This should go away when python runtimes are properly packaged.
    let status = Command::new("cp")
        .arg("-r")
        .arg("runtime/python/bcs/test_bcs.py")
        .arg(dir.path().join("src/bcs"))
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("cp")
        .arg("-r")
        .arg("runtime/python/bincode/test_bincode.py")
        .arg(dir.path().join("src/bincode"))
        .status()
        .unwrap();
    assert!(status.success());

    let site_packages = Command::new("python3")
        .arg("-c")
        .arg("import os; import numpy; print(os.path.dirname(numpy.__path__[0]), end='')")
        .output()
        .unwrap()
        .stdout;

    let local_bin_path = which::which("pyre")
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let status = Command::new("pyre")
        .current_dir(dir.path())
        .arg("--source-directory")
        .arg("src")
        .arg("--noninteractive")
        .arg("--binary")
        .arg(local_bin_path.join("pyre.bin"))
        .arg("--typeshed")
        .arg(local_bin_path.join("../lib/pyre_check/typeshed"))
        .arg("--search-path")
        .arg(String::from_utf8_lossy(&site_packages).as_ref())
        .arg("check")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_python_autotest() {
    let status = Command::new("python3")
        .arg("-m")
        .arg("unittest")
        .arg("discover")
        .arg("-s")
        .arg("runtime/python")
        .status()
        .unwrap();
    assert!(status.success());
}

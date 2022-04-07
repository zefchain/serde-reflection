// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde_generate::test_utils;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_that_installed_python_code_parses() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate")
        .arg("--")
        .arg("--language")
        .arg("python3")
        .arg("--target-source-dir")
        .arg(dir.path())
        .arg("--module-name")
        .arg("test_types")
        .arg("--with-runtimes")
        .arg("serde")
        .arg("bincode")
        .arg("bcs")
        .arg("--")
        .arg(yaml_path)
        .status()
        .unwrap();
    assert!(status.success());

    let python_path = format!(
        "{}:{}",
        std::env::var("PYTHONPATH").unwrap_or_default(),
        dir.path().to_string_lossy(),
    );
    let status = Command::new("python3")
        .arg("-c")
        .arg("import serde_types; import bincode; import bcs; import test_types")
        .env("PYTHONPATH", python_path)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_that_installed_python_code_with_package_parses() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate")
        .arg("--")
        .arg("--language")
        .arg("python3")
        .arg("--target-source-dir")
        .arg(dir.path().join("my_package"))
        .arg("--module-name")
        .arg("test_types")
        .arg("--serde-package-name")
        .arg("my_package")
        .arg("--with-runtimes")
        .arg("serde")
        .arg("bincode")
        .arg("bcs")
        .arg("--")
        .arg(yaml_path)
        .status()
        .unwrap();
    assert!(status.success());

    std::fs::write(
        dir.path().join("my_package").join("__init__.py"),
        r#"
__all__ = ["bcs", "serde_types", "serde_binary", "bincode", "test_types"]
"#,
    )
    .unwrap();

    let python_path = format!(
        "{}:{}",
        std::env::var("PYTHONPATH").unwrap_or_default(),
        dir.path().to_string_lossy(),
    );
    let status = Command::new("python3")
        .arg("-c")
        .arg("from my_package import serde_types; from my_package import bincode; from my_package import bcs; from my_package import test_types")
        .env("PYTHONPATH", python_path)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_that_installed_rust_code_compiles() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate")
        .arg("--")
        .arg("--language")
        .arg("rust")
        .arg("--module-name")
        .arg("testing:0.2.0")
        .arg("--target-source-dir")
        .arg(dir.path())
        .arg(yaml_path)
        .status()
        .unwrap();
    assert!(status.success());

    // Use a stable `target` dir to avoid downloading and recompiling crates everytime.
    let target_dir = std::env::current_dir().unwrap().join("../target");
    let status = Command::new("cargo")
        .current_dir(dir.path().join("testing"))
        .arg("build")
        .arg("--target-dir")
        .arg(target_dir)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_that_installed_cpp_code_compiles() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate")
        .arg("--")
        .arg("--language")
        .arg("cpp")
        .arg("--target-source-dir")
        .arg(dir.path())
        .arg(yaml_path)
        .arg("--with-runtimes")
        .arg("serde")
        .arg("bincode")
        .arg("bcs")
        .arg("--")
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("clang++")
        .arg("--std=c++17")
        .arg("-c")
        .arg("-o")
        .arg(dir.path().join("test.o"))
        .arg("-I")
        .arg(dir.path())
        .arg(dir.path().join("test.hpp"))
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_that_installed_java_code_compiles() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate")
        .arg("--")
        .arg("--language")
        .arg("java")
        .arg("--target-source-dir")
        .arg(dir.path())
        .arg("--module-name")
        .arg("test.types")
        .arg("--with-runtimes")
        .arg("serde")
        .arg("--")
        .arg(yaml_path)
        .status()
        .unwrap();
    assert!(status.success());

    let paths = std::fs::read_dir(dir.path().join("com/novi/serde"))
        .unwrap()
        .map(|e| e.unwrap().path());
    let status = Command::new("javac")
        .arg("-cp")
        .arg(dir.path())
        .arg("-d")
        .arg(dir.path())
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success());

    let paths = std::fs::read_dir(dir.path().join("test/types"))
        .unwrap()
        .map(|e| e.unwrap().path());
    let status = Command::new("javac")
        .arg("-cp")
        .arg(dir.path())
        .arg("-d")
        .arg(dir.path())
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_that_installed_ocaml_code_compiles() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate")
        .arg("--")
        .arg("--language")
        .arg("ocaml")
        .arg("--target-source-dir")
        .arg(dir.path())
        .arg("--with-runtimes")
        .arg("serde")
        .arg("--")
        .arg(yaml_path)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("dune")
        .arg("build")
        .arg("--root")
        .arg(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
}

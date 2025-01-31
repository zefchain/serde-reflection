// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::process::Command;
use tempfile::tempdir;

mod test_utils {
    use serde::{Deserialize, Serialize};
    use serde_bytes::ByteBuf;
    use serde_reflection::{Registry, Samples, Tracer, TracerConfig};
    use std::collections::BTreeMap;

    // More complex data format used to test re-serialization and basic fuzzing.
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub enum SerdeData {
        PrimitiveTypes(PrimitiveTypes),
        OtherTypes(OtherTypes),
        UnitVariant,
        NewTypeVariant(String),
        TupleVariant(u32, u64),
        StructVariant {
            f0: UnitStruct,
            f1: NewTypeStruct,
            f2: TupleStruct,
            f3: Struct,
        },
        ListWithMutualRecursion(List<Box<SerdeData>>),
        TreeWithMutualRecursion(Tree<Box<SerdeData>>),
        TupleArray([u32; 3]),
        UnitVector(Vec<()>),
        SimpleList(SimpleList),
        CStyleEnum(CStyleEnum),
        ComplexMap(BTreeMap<([u32; 2], [u8; 4]), ()>),
        EmptyTupleVariant(),
        EmptyStructVariant {},
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct PrimitiveTypes {
        f_bool: bool,
        f_u8: u8,
        f_u16: u16,
        f_u32: u32,
        f_u64: u64,
        f_u128: u128,
        f_i8: i8,
        f_i16: i16,
        f_i32: i32,
        f_i64: i64,
        f_i128: i128,
        // The following types are not supported by our bincode and BCS runtimes, therefore
        // we don't populate them for testing.
        f_f32: Option<f32>,
        f_f64: Option<f64>,
        f_char: Option<char>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct OtherTypes {
        f_string: String,
        f_bytes: ByteBuf,
        f_option: Option<Struct>,
        f_unit: (),
        f_seq: Vec<Struct>,
        f_tuple: (u8, u16),
        f_stringmap: BTreeMap<String, u32>,
        f_intset: BTreeMap<u64, ()>, // Avoiding BTreeSet because Serde treats them as sequences.
        f_nested_seq: Vec<Vec<Struct>>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct UnitStruct;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct NewTypeStruct(u64);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct TupleStruct(u32, u64);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Struct {
        x: u32,
        y: u64,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub enum List<T> {
        Empty,
        Node(T, Box<List<T>>),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct Tree<T> {
        value: T,
        children: Vec<Tree<T>>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct SimpleList(Option<Box<SimpleList>>);

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub enum CStyleEnum {
        A,
        B,
        C,
        D,
        E = 10,
    }

    /// The registry corresponding to the test data structures above.
    pub fn get_registry() -> serde_reflection::Result<Registry> {
        let mut tracer = Tracer::new(TracerConfig::default());
        let samples = Samples::new();
        tracer.trace_type::<SerdeData>(&samples)?;
        tracer.trace_type::<List<SerdeData>>(&samples)?;
        tracer.trace_type::<CStyleEnum>(&samples)?;
        tracer.registry()
    }
}

#[test]
fn test_that_installed_python_code_parses() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path.clone(), serde_yaml::to_string(&registry).unwrap()).unwrap();

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("serde-generate-bin")
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
        .arg("serde-generate-bin")
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
        .arg("serde-generate-bin")
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
fn create_test_yaml() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let yaml_path = dir.path().join("test.yaml");
    std::fs::write(yaml_path, serde_yaml::to_string(&registry).unwrap()).unwrap();
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
        .arg("serde-generate-bin")
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
        .arg("serde-generate-bin")
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
        .arg("serde-generate-bin")
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

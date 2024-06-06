// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::Runtime;
use serde_generate::{rust, CodeGeneratorConfig};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

#[test]
fn test_rust_bcs_runtime() {
    test_rust_runtime(Runtime::Bcs);
}

#[test]
fn test_rust_bincode_runtime() {
    test_rust_runtime(Runtime::Bincode);
}

// Full test using cargo. This may take a while.
fn test_rust_runtime(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let mut file = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
    write!(
        &mut file,
        r#"[package]
name = "testing2"
version = "0.1.0"
edition = "2018"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_bytes = "0.11"
{}

[workspace]
"#,
        runtime.rust_package()
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();

    let config = CodeGeneratorConfig::new("testing".to_string());
    let generator = rust::CodeGenerator::new(&config);

    let source_path = dir.path().join("src/main.rs");
    let mut source = File::create(source_path).unwrap();
    generator.output(&mut source, &registry).unwrap();

    let encodings: Vec<_> = runtime
        .get_positive_samples()
        .iter()
        .map(|bytes| format!("vec!{:?}", bytes))
        .collect();

    writeln!(
        source,
        r#"
fn main() {{
    for encoding in vec![{}] {{
        let value = {}::<SerdeData>(&encoding).unwrap();
        let s = {}(&value).unwrap();
        assert_eq!(s, encoding);
    }}
}}
"#,
        encodings.join(", "),
        runtime.quote_deserialize(),
        runtime.quote_serialize(),
    )
    .unwrap();

    // Use a stable `target` dir to avoid downloading and recompiling crates everytime.
    let target_dir = std::env::current_dir().unwrap().join("../target");
    let status = Command::new("cargo")
        .current_dir(dir.path())
        .arg("run")
        .arg("--target-dir")
        .arg(target_dir)
        .status()
        .unwrap();
    assert!(status.success());
}

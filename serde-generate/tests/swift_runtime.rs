// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{swift, CodeGeneratorConfig, SourceInstaller};
use std::{fs::File, io::Write, process::Command, sync::Mutex};

lazy_static::lazy_static! {
    // Avoid interleaving compiler calls because the output gets very messy.
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn test_swift_runtime_autotests() {
    let runtime_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("../../../serde-generate/runtime/swift");

    let status = Command::new("swift")
        .current_dir(runtime_path.to_str().unwrap())
        .arg("test")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_swift_bcs_runtime_on_simple_data() {
    test_swift_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_swift_bincode_runtime_on_simple_data() {
    test_swift_runtime_on_simple_data(Runtime::Bincode);
}

fn test_swift_runtime_on_simple_data(runtime: Runtime) {
    // To see the source, uncomment this and replace `dir.path()` by `my_path` below.
    // let my_path = std::path::Path::new("../test");
    // std::fs::remove_dir_all(my_path).unwrap_or(());
    // std::fs::create_dir_all(my_path).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config =
        CodeGeneratorConfig::new("Testing".to_string()).with_encodings(vec![runtime.into()]);
    let registry = test_utils::get_simple_registry().unwrap();
    let installer = swift::Installer::new(dir.path().to_path_buf());
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap(); // also installs bcs and bincode

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    std::fs::create_dir_all(dir.path().join("Sources/main")).unwrap();
    let main_path = dir.path().join("Sources/main/main.swift");
    let mut main = File::create(main_path).unwrap();
    writeln!(
        main,
        r#"
import Serde
import Testing

var input : [UInt8] = [{0}]
let value = try Test.{1}Deserialize(input: input)

let value2 = Test.init(
    a: [4, 6],
    b: Tuple2.init(-3, 5),
    c: Choice.c(x: 7)
)
assert(value == value2, "value != value2")

let output = try value2.{1}Serialize()
assert(input == output, "input != output")

input += [0]
do {{
    let _ = try Test.{1}Deserialize(input: input)
    assertionFailure("Was expecting an error")
}}
catch {{}}

do {{
    let input2 : [UInt8] = [0, 1]
    let _ = try Test.{1}Deserialize(input: input2)
    assertionFailure("Was expecting an error")
}}
catch {{}}
"#,
        reference
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", "),
        runtime.name(),
    )
    .unwrap();

    let mut file = File::create(dir.path().join("Package.swift")).unwrap();
    write!(
        file,
        r#"// swift-tools-version:5.3

import PackageDescription

let package = Package(
    name: "Testing",
    targets: [
        .target(
            name: "Serde",
            dependencies: []),
        .target(
            name: "Testing",
            dependencies: ["Serde"]),
        .target(
            name: "main",
            dependencies: ["Serde", "Testing"]
        ),
    ]
)
"#
    )
    .unwrap();

    {
        let _lock = MUTEX.lock().unwrap();
        let status = Command::new("swift")
            .current_dir(dir.path())
            .arg("run")
            .status()
            .unwrap();
        assert!(status.success());
    }
}

#[test]
fn test_swift_bcs_runtime_on_supported_types() {
    test_swift_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_swift_bincode_runtime_on_supported_types() {
    test_swift_runtime_on_supported_types(Runtime::Bincode);
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "[{}]",
        bytes
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn test_swift_runtime_on_supported_types(runtime: Runtime) {
    // To see the source, uncomment this and replace `dir.path()` by `my_path` below.
    // let my_path = std::path::Path::new("../test");
    // std::fs::remove_dir_all(my_path).unwrap_or(());
    // std::fs::create_dir_all(my_path).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let config =
        CodeGeneratorConfig::new("Testing".to_string()).with_encodings(vec![runtime.into()]);
    let registry = test_utils::get_registry().unwrap();
    let installer = swift::Installer::new(dir.path().to_path_buf());
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap(); // also installs bcs and bincode

    std::fs::create_dir_all(dir.path().join("Sources/main")).unwrap();
    let main_path = dir.path().join("Sources/main/main.swift");
    let mut main = File::create(main_path).unwrap();

    let positive_encodings = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join(", ");

    let negative_encodings = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(
        main,
        r#"
import Serde
import Testing

let positive_inputs : [[UInt8]] = [{0}]
let negative_inputs : [[UInt8]] = [{1}]

for input in positive_inputs {{
    let value = try SerdeData.{2}Deserialize(input: input)
    let output = try value.{2}Serialize()
    assert(input == output, "input != output:\n  \(input)\n  \(output)")

    // Test self-equality for the Serde value.
    let value2 = try SerdeData.{2}Deserialize(input: input)
    assert(value == value2, "Deserialized value should test equal to itself: \(input)")

    // Test simple mutations of the input.
    for i in 0..<min(40, input.count) {{
        var input3 = input
        input3[i] ^= 0x80
        let value3 = try? SerdeData.{2}Deserialize(input: input3)
        if let value3 = value3 {{
            assert(value3 != value, "Modified input should give a different value:\n  \(input)\n  \(input3)")
        }}
    }}

}}

for input in negative_inputs {{
    if let _ = try? SerdeData.{2}Deserialize(input: input) {{
        assertionFailure("Input should fail to deserialize: \(input)")
    }}
}}
"#,
        positive_encodings,
        negative_encodings,
        runtime.name(),
    )
    .unwrap();

    let mut file = File::create(dir.path().join("Package.swift")).unwrap();
    write!(
        file,
        r#"// swift-tools-version:5.3

import PackageDescription

let package = Package(
    name: "Testing",
    targets: [
        .target(
            name: "Serde",
            dependencies: []),
        .target(
            name: "Testing",
            dependencies: ["Serde"]),
        .target(
            name: "main",
            dependencies: ["Serde", "Testing"]
        ),
    ]
)
"#
    )
    .unwrap();

    {
        let _lock = MUTEX.lock().unwrap();
        let status = Command::new("swift")
            .current_dir(dir.path())
            .arg("run")
            .status()
            .unwrap();
        assert!(status.success());
    }
}

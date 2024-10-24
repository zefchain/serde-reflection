// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{dart, CodeGeneratorConfig, SourceInstaller};
use std::{
    fs::{create_dir_all, File},
    io::{Result, Write},
    path::Path,
    process::Command,
};
use tempfile::tempdir;

#[cfg(target_family = "windows")]
const DART_EXECUTABLE: &str = "dart.bat";
#[cfg(not(target_family = "windows"))]
const DART_EXECUTABLE: &str = "dart";

fn install_test_dependency(path: &Path) -> Result<()> {
    Command::new(DART_EXECUTABLE)
        .current_dir(path)
        .env("PUB_CACHE", "../.pub-cache")
        .args(["pub", "add", "-d", "test"])
        .status()?;

    Ok(())
}

#[test]
fn test_dart_runtime_autotest() {
    // Not setting PUB_CACHE here because this is the only test run
    // with the default config anyway.
    let output = Command::new(DART_EXECUTABLE)
        .current_dir("runtime/dart")
        .args(["test", "-r", "expanded"])
        .output()
        .unwrap();
    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stdout);
        eprintln!("{}", error_output);
    }
    assert!(output.status.success());
}

#[test]
fn test_dart_bcs_runtime_on_simple_data() {
    test_dart_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_dart_bincode_runtime_on_simple_data() {
    test_dart_runtime_on_simple_data(Runtime::Bincode);
}

fn test_dart_runtime_on_simple_data(runtime: Runtime) {
    let tempdir = tempdir().unwrap();
    let source_path = tempdir
        .path()
        .join(format!("dart_project_{}", runtime.name().to_lowercase()));
    let registry = test_utils::get_simple_registry().unwrap();
    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![runtime.into()])
        .with_c_style_enums(false);

    let installer = dart::Installer::new(source_path.clone());
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();
    install_test_dependency(&source_path).unwrap();

    create_dir_all(source_path.join("test")).unwrap();

    let source = source_path.join("test/runtime_test.dart");
    let mut source_file = File::create(source).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    writeln!(
        source_file,
        r#"
import 'dart:typed_data';
import 'package:example/example.dart';
import 'package:test/test.dart';
import 'package:tuple/tuple.dart';
import '../lib/src/bcs/bcs.dart';
import '../lib/src/bincode/bincode.dart';

void main() {{
    test('{0} simple data', () {{
        final expectedBytes = {1};
        Test deserializedInstance = Test.{0}Deserialize(expectedBytes);

        Test expectedInstance = Test(
            a: [4, 6],
            b: Tuple2(-3, Uint64.parse('5')),
            c: ChoiceC(x: 7),
        );

        expect(deserializedInstance, equals(expectedInstance));
        final serializedBytes = expectedInstance.{0}Serialize();
        expect(serializedBytes, equals(expectedBytes));
    }});
}}"#,
        runtime.name().to_lowercase(),
        quote_bytes(&reference),
    )
    .unwrap();

    let output = Command::new(DART_EXECUTABLE)
        .current_dir(&source_path)
        .env("PUB_CACHE", "../.pub-cache")
        .args(["test", "test/runtime_test.dart"])
        .output()
        .unwrap();
    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stdout);
        eprintln!("{}", error_output);
    }
    assert!(output.status.success());
}

#[test]
fn test_dart_bcs_runtime_on_supported_types() {
    test_dart_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_dart_bincode_runtime_on_supported_types() {
    test_dart_runtime_on_supported_types(Runtime::Bincode);
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "Uint8List.fromList([{}])",
        bytes
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn test_dart_runtime_on_supported_types(runtime: Runtime) {
    let tempdir = tempdir().unwrap();
    let source_path = tempdir
        .path()
        .join(format!("dart_project_{}", runtime.name().to_lowercase()));
    let registry = test_utils::get_registry().unwrap();
    let config = CodeGeneratorConfig::new("example".to_string())
        .with_encodings(vec![runtime.into()])
        .with_c_style_enums(false);

    let installer = dart::Installer::new(source_path.clone());
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();
    install_test_dependency(&source_path).unwrap();

    create_dir_all(source_path.join("test")).unwrap();

    let source = source_path.join("test/runtime_test.dart");
    let mut source_file = File::create(source).unwrap();

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
        source_file,
        r#"
import 'dart:typed_data';
import 'package:example/example.dart';
import 'package:test/test.dart';
import 'package:tuple/tuple.dart';
import '../lib/src/bcs/bcs.dart';
import '../lib/src/bincode/bincode.dart';

void main() {{
    test('{0} supported types', () {{
        List<Uint8List> positiveInputs = [{1}];
        List<Uint8List> negativeInputs = [{2}];

        for (var input in positiveInputs) {{
            // Deserialize the input.
            SerdeData value = SerdeData.{0}Deserialize(input);

            // Exclude `SerdeData::ComplexMap` from tests
            // because the `matcher` package used by the `test` package
            // doesn't support lists within tuples.
            if (value is SerdeDataComplexMap) {{
                continue;
            }}

            // Serialize the deserialized value.
            final output = value.{0}Serialize();
            expect(output, equals(input));

            // Test self-equality for the deserialized value.
            SerdeData value2 = SerdeData.{0}Deserialize(input);
            expect(value, equals(value2));

            // Test simple mutations of the input.
            for (var i = 0; i < input.length; i++) {{
                var input2 = Uint8List.fromList(input);
                input2[i] ^= 0x80; // Mutate a byte
                SerdeData value2;
                try {{
                    value2 = SerdeData.{0}Deserialize(input2);
                }} catch (e) {{
                    continue;
                }}
                expect(value, isNot(equals(value2)));
            }}
        }}

        // Test negative inputs for deserialization failure.
        for (var input in negativeInputs) {{
            try {{
                SerdeData.{0}Deserialize(input);
            }} catch (e) {{
                continue;
            }}
            throw Exception('Input should fail to deserialize');
        }}
    }});
}}
"#,
        runtime.name().to_lowercase(),
        positive_encodings,
        negative_encodings,
    )
    .unwrap();

    let output = Command::new(DART_EXECUTABLE)
        .current_dir(&source_path)
        .env("PUB_CACHE", "../.pub-cache")
        .args(["test", "test/runtime_test.dart"])
        .output()
        .unwrap();
    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stdout);
        eprintln!("{}", error_output);
    }
    assert!(output.status.success());
}

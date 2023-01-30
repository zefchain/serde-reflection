// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{python3, CodeGeneratorConfig};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

#[test]
fn test_python_bcs_runtime_on_simple_data() {
    test_python_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_python_bincode_runtime_on_simple_data() {
    test_python_runtime_on_simple_data(Runtime::Bincode);
}

fn test_python_runtime_on_simple_data(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.py");
    let mut source = File::create(&source_path).unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let generator = python3::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (3, 5),
        c: Choice::C { x: 7 },
    });
    writeln!(
        source,
        r#"
input = bytes({1:?})
value = Test([4, 6], (3, 5), Choice__C(7))

s = value.{0}_serialize()
assert s == input

v = Test.{0}_deserialize(s)
assert v == value
assert v.c.x == 7

v = Test([4, 6], (3, 0), Choice__C(7))
t = v.{0}_serialize()
assert len(t) == len(s)
assert t != s

seen_error = False
try:
    Test.{0}_deserialize(input + bytes([0]))
except st.DeserializationError:
    seen_error = True
assert seen_error
"#,
        runtime.name(),
        reference,
    )
    .unwrap();

    let python_path = std::env::var("PYTHONPATH").unwrap_or_default() + ":runtime/python";
    let status = Command::new("python3")
        .arg(source_path)
        .env("PYTHONPATH", python_path)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_python_bcs_runtime_on_supported_types() {
    test_python_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_python_bincode_runtime_on_supported_types() {
    test_python_runtime_on_supported_types(Runtime::Bincode);
}

fn test_python_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.py");
    let mut source = File::create(&source_path).unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let generator = python3::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let positive_encodings: Vec<_> = runtime.get_positive_samples_quick();
    let negative_encodings: Vec<_> = runtime.get_negative_samples();

    writeln!(
        source,
        r#"
from copy import copy
import serde_types as st
import sys
import bcs

# Required to avoid RecursionError's in python.
sys.setrecursionlimit(bcs.MAX_CONTAINER_DEPTH * 5)

positive_encodings = [bytes(a) for a in {1:?}]
negative_encodings = [bytes(a) for a in {2:?}]

for encoding in positive_encodings:
    v = SerdeData.{0}_deserialize(encoding)
    s = v.{0}_serialize()
    assert s == encoding

    # Test self-equality for the Serde value.
    assert v == SerdeData.{0}_deserialize(encoding)

    # Test simple mutations of the input.
    for i in range(min(len(encoding), 20)):
        encoding2 = bytearray(encoding)
        encoding2[i] ^= 0x81
        try:
            v2 = SerdeData.{0}_deserialize(encoding2)
            assert v2 != v
        except st.DeserializationError:
            pass

for encoding in negative_encodings:
    try:
        SerdeData.{0}_deserialize(encoding)
        print('Input bitstring was wrongfully accepted:\n', encoding)
        sys.exit(1)
    except st.DeserializationError:
        pass
"#,
        runtime.name(),
        positive_encodings,
        negative_encodings,
    )
    .unwrap();

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
}

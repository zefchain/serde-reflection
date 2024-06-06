// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{cpp, CodeGeneratorConfig};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

#[test]
fn test_cpp_bcs_runtime_on_simple_date() {
    test_cpp_runtime_on_simple_date(Runtime::Bcs);
}

#[test]
fn test_cpp_bincode_runtime_on_simple_date() {
    test_cpp_runtime_on_simple_date(Runtime::Bincode);
}

fn test_cpp_runtime_on_simple_date(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();
    let header_path = dir.path().join("test.hpp");
    let mut header = File::create(header_path).unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let generator = cpp::CodeGenerator::new(&config);
    generator.output(&mut header, &registry).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    let source_path = dir.path().join("test.cpp");
    let mut source = File::create(&source_path).unwrap();
    writeln!(
        source,
        r#"
#include <cassert>
#include "test.hpp"

using namespace testing;

int main() {{
    std::vector<uint8_t> input = {{{0}}};
    auto value = Test::{1}Deserialize(input);

    auto a = std::vector<uint32_t> {{4, 6}};
    auto b = std::tuple<int64_t, uint64_t> {{-3, 5}};
    auto c = Choice {{ Choice::C {{ 7 }} }};
    auto value2 = Test {{a, b, c}};

    assert(value == value2);

    auto output = value2.{1}Serialize();

    assert(input == output);

    input.push_back(1);
    try {{
        Test::{1}Deserialize(input);
    }} catch (...) {{
        return 0;
    }}
    return 1;
}}
"#,
        reference
            .iter()
            .map(|x| format!("0x{:02x}", x))
            .collect::<Vec<_>>()
            .join(", "),
        runtime.name(),
    )
    .unwrap();

    let status = Command::new("clang++")
        .arg("--std=c++17")
        .arg("-o")
        .arg(dir.path().join("test"))
        .arg("-I")
        .arg("runtime/cpp")
        .arg(source_path)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(dir.path().join("test")).status().unwrap();
    assert!(status.success());
}

#[test]
fn test_cpp_bcs_runtime_on_supported_types() {
    test_cpp_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_cpp_bincode_runtime_on_supported_types() {
    test_cpp_runtime_on_supported_types(Runtime::Bincode);
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "std::vector<uint8_t>{{{}}}",
        bytes
            .iter()
            .map(|x| format!("0x{:02x}", x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn test_cpp_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let header_path = dir.path().join("test.hpp");
    let mut header = File::create(header_path).unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let generator = cpp::CodeGenerator::new(&config);
    generator.output(&mut header, &registry).unwrap();

    let positive_encodings: Vec<_> = runtime
        .get_positive_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect();

    let negative_encodings: Vec<_> = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect();

    let source_path = dir.path().join("test.cpp");
    let mut source = File::create(&source_path).unwrap();
    writeln!(
        source,
        r#"
#include <algorithm>
#include <exception>
#include <iostream>
#include <cassert>
#include "test.hpp"

using namespace testing;

int main() {{
    std::vector<std::vector<uint8_t>> positive_inputs = {{{0}}};
    std::vector<std::vector<uint8_t>> negative_inputs = {{{1}}};
    try {{
        for (auto input: positive_inputs) {{
            auto value = SerdeData::{2}Deserialize(input);
            auto output = value.{2}Serialize();
            assert(input == output);

            // Test self-equality for the Serde value.
            {{
                auto value2 = SerdeData::{2}Deserialize(input);
                assert(value == value2);
            }}

            // Test simple mutations of the input.
            for (int i = 0; i < std::min(input.size(), 20ul); i++) {{
                auto input2 = input;
                input2[i] ^= 0x81;
                try {{
                    auto value2 = SerdeData::{2}Deserialize(input2);
                    assert(!(value2 == value));
                }} catch (serde::deserialization_error e) {{
                    // All good
                }} catch (std::bad_alloc const &e) {{
                    // All good
                }} catch (std::length_error const &e) {{
                    // All good
                }}
            }}
        }}

        for (auto input: negative_inputs) {{
            try {{
                SerdeData::{2}Deserialize(input);
                printf("Input should fail to deserialize:");
                for (auto x : input) {{
                    printf(" %d", x);
                }}
                printf("\n");
                assert(false);
            }} catch (serde::deserialization_error e) {{
                // All good
            }}
        }}
        return 0;
    }} catch (std::exception& e) {{
        std::cout << "Error: " << e.what() << '\n';
        return 1;
    }}
}}
"#,
        positive_encodings.join(", "),
        negative_encodings.join(", "),
        runtime.name(),
    )
    .unwrap();

    let status = Command::new("clang++")
        .arg("--std=c++17")
        .arg("-g")
        .arg("-O3") // remove for debugging
        .arg("-o")
        .arg(dir.path().join("test"))
        .arg("-I")
        .arg("runtime/cpp")
        .arg(source_path)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new(dir.path().join("test")).status().unwrap();
    assert!(status.success());
}

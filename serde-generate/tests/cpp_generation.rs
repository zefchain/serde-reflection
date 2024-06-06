// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{cpp, CodeGeneratorConfig, Encoding};
use std::{collections::BTreeMap, fs::File, io::Write, process::Command};
use tempfile::{tempdir, TempDir};

fn test_that_cpp_code_compiles_with_config(
    config: &CodeGeneratorConfig,
) -> (TempDir, std::path::PathBuf) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let header_path = dir.path().join("test.hpp");
    let mut header = File::create(&header_path).unwrap();

    let generator = cpp::CodeGenerator::new(config);
    generator.output(&mut header, &registry).unwrap();

    let source_path = dir.path().join("test.cpp");
    let mut source = File::create(&source_path).unwrap();
    writeln!(
        source,
        r#"
#include "bincode.hpp"
#include "test.hpp"
"#
    )
    .unwrap();

    let status = Command::new("clang++")
        .arg("--std=c++17")
        .arg("-c")
        .arg("-o")
        .arg(dir.path().join("test.o"))
        .arg("-I")
        .arg("runtime/cpp")
        .arg(&source_path)
        .status()
        .unwrap();
    assert!(status.success());

    (dir, header_path)
}

#[test]
fn test_that_cpp_code_compiles() {
    let config = CodeGeneratorConfig::new("testing".to_string());
    test_that_cpp_code_compiles_with_config(&config);
}

#[test]
fn test_that_cpp_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_cpp_code_compiles_with_config(&config);
}

#[test]
fn test_that_cpp_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_cpp_code_compiles_with_config(&config);
}

#[test]
fn test_that_cpp_code_compiles_without_serialization() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    test_that_cpp_code_compiles_with_config(&config);
}

#[test]
fn test_that_cpp_code_compiles_with_comments() {
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

    let (_dir, header_path) = test_that_cpp_code_compiles_with_config(&config);

    // Comments were correctly generated.
    let content = std::fs::read_to_string(header_path).unwrap();
    assert!(content.contains(
        r#"
    /// Some
    /// comments
"#
    ));
    assert!(content.contains(
        r#"
        /// Some other comments
"#
    ));
}

#[test]
fn test_cpp_code_with_external_definitions() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let header_path = dir.path().join("test.hpp");
    let mut header = File::create(&header_path).unwrap();

    // Pretend that "Tree" is external.
    let mut definitions = BTreeMap::new();
    definitions.insert("pkg::foo".to_string(), vec!["Tree".to_string()]);
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_external_definitions(definitions);
    let generator = cpp::CodeGenerator::new(&config);
    generator.output(&mut header, &registry).unwrap();

    let content = std::fs::read_to_string(&header_path).unwrap();
    assert!(content.contains("pkg::foo::Tree"));
    assert!(!content.contains("testing::Tree"));
}

#[test]
fn test_that_cpp_code_compiles_with_custom_code() {
    let custom_code = vec![
        (
            vec!["testing".to_string(), "SerdeData".to_string()],
            "virtual ~SerdeData() = default;".to_string(),
        ),
        (
            vec![
                "testing".to_string(),
                "List".to_string(),
                "Node".to_string(),
            ],
            "virtual ~Node() = default;".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string()).with_custom_code(custom_code);

    let (_dir, header_path) = test_that_cpp_code_compiles_with_config(&config);

    // Comments were correctly generated.
    let content = std::fs::read_to_string(header_path).unwrap();
    assert!(content.contains("~SerdeData"));
    assert!(content.contains("~Node"));
}

#[test]
fn test_that_cpp_code_links() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let header_path = dir.path().join("test.hpp");
    let mut header = File::create(header_path).unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    let generator = cpp::CodeGenerator::new(&config);
    generator.output(&mut header, &registry).unwrap();

    let source_path = dir.path().join("lib.cpp");
    let mut source = File::create(source_path).unwrap();
    writeln!(
        source,
        r#"
#include "bcs.hpp"
#include "test.hpp"

using namespace serde;
using namespace testing;

std::vector<uint8_t> serialize_data(SerdeData data) {{
    auto serializer = BcsSerializer();
    Serializable<SerdeData>::serialize(data, serializer);
    return std::move(serializer).bytes();
}}

SerdeData deserialize_data(const std::vector<uint8_t> &input) {{
    auto deserializer = BcsDeserializer(input);
    return Deserializable<SerdeData>::deserialize(deserializer);
}}
"#
    )
    .unwrap();

    let source_path = dir.path().join("main.cpp");
    let mut source = File::create(source_path).unwrap();
    writeln!(
        source,
        r#"
#include "test.hpp"

using namespace serde;
using namespace testing;

extern std::vector<uint8_t> serialize_data(SerdeData data);

extern SerdeData deserialize_data(const std::vector<uint8_t> &bytes);

bool test(const std::vector<uint8_t>& input) {{
    auto output = serialize_data(deserialize_data(input));
    return input == output;
}}

int main() {{
    // dummy
    return test({{}});
}}
"#
    )
    .unwrap();

    let status = Command::new("clang++")
        .arg("--std=c++17")
        .arg("-I")
        .arg("runtime/cpp")
        .arg("-o")
        .arg(dir.path().join("main"))
        .arg(dir.path().join("lib.cpp"))
        .arg(dir.path().join("main.cpp"))
        .status()
        .unwrap();
    assert!(status.success());
}

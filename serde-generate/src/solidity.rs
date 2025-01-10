// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig,
};
use heck::SnakeCase;
use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
use std::{
    collections::{HashMap, HashSet},
    io::{Result, Write},
    path::PathBuf,
};

/// Main configuration object for code-generation in C++.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
}

/// Shared state for the code generation of a C++ source file.
struct SolEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
}

fn get_data_location(need_memory: bool) -> String {
    match need_memory {
        true => " memory".to_string(),
        false => "".to_string(),
    }
}


fn output_generic_bcs_deserialize<T: std::io::Write>(out: &mut IndentedWriter<T>, key_name: &str, code_name: &str, need_memory: bool) -> Result<()> {
    let data_location = get_data_location(need_memory);
    writeln!(out, "function bcs_deserialize_{key_name}(bytes memory input) internal pure returns ({code_name}{data_location}) {{")?;
    writeln!(out, "  uint64 new_pos;")?;
    writeln!(out, "  {code_name}{data_location} value;")?;
    writeln!(out, "  (new_pos, value) = bcs_deserialize_offset_{key_name}(0, input);")?;
    writeln!(out, "  require(new_pos == input.length, \"incomplete deserialization\");")?;
    writeln!(out, "  return value;")?;
    writeln!(out, "}}")?;
    writeln!(out)?;
    Ok(())
}


fn get_keywords() -> HashSet<String> {
    let v = vec![
        "abstract", "after", "alias", "anonymous", "as", "assembly", "break",
        "catch", "constant", "continue", "constructor", "contract", "delete",
        "do", "else", "emit", "enum", "error", "event", "external", "fallback",
        "for", "function", "if", "immutable", "import", "indexed", "interface",
        "internal", "is", "library", "mapping", "memory", "modifier", "new",
        "override", "payable", "pragma", "private", "public", "pure", "receive",
        "return", "returns", "revert", "storage", "struct", "throw", "try",
        "type", "unchecked", "using", "virtual", "view", "while", "addmod",
        "blockhash", "ecrecover", "keccak256", "mulmod", "sha256", "ripemd160",
        "block", "msg", "tx", "balance", "transfer", "send", "call", "delegatecall",
        "staticcall", "this", "super", "gwei", "finney", "szabo", "ether", "seconds",
        "minutes", "hours", "days", "weeks", "years", "wei", "hex", "address",
        "bool", "bytes", "string", "mapping", "int"];
    let mut v = v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>();
    for length in [8, 16, 32, 64, 128, 256] {
        v.push(format!("int{}", length));
        v.push(format!("uint{}", length));
    }
    for length in 1..=32 {
        v.push(format!("int{}", length));
    }
    v.into_iter().collect::<HashSet<_>>()
}

fn safe_variable(s: &str) -> String {
    let keywords = get_keywords();
    if keywords.contains(s) {
        s.to_owned() + "_"
    } else {
        s.to_string()
    }
}


#[derive(Clone, Debug)]
enum Primitive {
    Unit,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    Char,
    Str,
    Bytes,
}

impl Primitive {
    pub fn name(&self) -> String {
        use Primitive::*;
        match self {
            Unit => "empty_struct".into(),
            Bool => "bool".into(),
            I8 => "int8".into(),
            I16 => "int16".into(),
            I32 => "int32".into(),
            I64 => "int64".into(),
            I128 => "int128".into(),
            U8 => "uint8".into(),
            U16 => "uint16".into(),
            U32 => "uint32".into(),
            U64 => "uint64".into(),
            U128 => "uint128".into(),
            Char => "bytes1".into(),
            Str => "string".into(),
            Bytes => "bytes".into(),
        }
    }

    pub fn output<T: std::io::Write>(&self, out: &mut IndentedWriter<T>) -> Result<()> {
        use Primitive::*;
        match self {
            Unit => {
                writeln!(out, "struct empty_struct {{")?;
                writeln!(out, "  int8 val;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_serialize_empty_struct(empty_struct memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bytes memory result;")?;
                writeln!(out, "  return result;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_empty_struct(uint64 pos, bytes memory input) internal pure returns (uint64, empty_struct memory) {{")?;
                writeln!(out, "  int8 val = 0;")?;
                writeln!(out, "  return (pos, empty_struct(val));")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            Bool => {
                writeln!(out, "function bcs_serialize_bool(bool input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_bool(uint64 pos, bytes memory input) internal pure returns (uint64, bool) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 1);")?;
                writeln!(out, "  bool value = abi.decode(input_red, (bool));")?;
                writeln!(out, "  return (pos + 1, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            I8 => {
                writeln!(out, "function bcs_serialize_int8(int8 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_int8(uint64 pos, bytes memory input) internal pure returns (uint64, int8) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 1);")?;
                writeln!(out, "  int8 value = abi.decode(input_red, (int8));")?;
                writeln!(out, "  return (pos + 1, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            I16 => {
                writeln!(out, "function bcs_serialize_int16(int16 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_int16(uint64 pos, bytes memory input) internal pure returns (uint64, int16) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 2);")?;
                writeln!(out, "  int16 value = abi.decode(input_red, (int16));")?;
                writeln!(out, "  return (pos + 2, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            I32 => {
                writeln!(out, "function bcs_serialize_int32(int32 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_int32(uint64 pos, bytes memory input) internal pure returns (uint64, int32) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 4);")?;
                writeln!(out, "  int32 value = abi.decode(input_red, (int32));")?;
                writeln!(out, "  return (pos + 4, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            I64 => {
                writeln!(out, "function bcs_serialize_int64(int64 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_int64(uint64 pos, bytes memory input) internal pure returns (uint64, int64) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 8);")?;
                writeln!(out, "  int64 value = abi.decode(input_red, (int64));")?;
                writeln!(out, "  return (pos + 8, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            I128 => {
                writeln!(out, "function bcs_serialize_int128(int128 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_int128(uint64 pos, bytes memory input) internal pure returns (uint64, int128) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 16);")?;
                writeln!(out, "  int128 value = abi.decode(input_red, (int128));")?;
                writeln!(out, "  return (pos+8, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            U8 => {
                writeln!(out, "function bcs_serialize_uint8(uint8 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_uint8(uint64 pos, bytes memory input) internal pure returns (uint64, uint8) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 1);")?;
                writeln!(out, "  uint8 value = abi.decode(input_red, (uint8));")?;
                writeln!(out, "  return (pos + 1, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            U16 => {
                writeln!(out, "function bcs_serialize_uint16(uint16 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_uint16(uint64 pos, bytes memory input) internal pure returns (uint64, uint16) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 2);")?;
                writeln!(out, "  uint16 value = abi.decode(input_red, (uint16));")?;
                writeln!(out, "  return (pos + 2, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            U32 => {
                writeln!(out, "function bcs_serialize_uint32(uint32 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_uint32(uint64 pos, bytes memory input) internal pure returns (uint64, uint32) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 4);")?;
                writeln!(out, "  uint32 value = abi.decode(input_red, (uint32));")?;
                writeln!(out, "  return (pos + 4, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            U64 => {
                writeln!(out, "function bcs_serialize_uint64(uint64 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_uint64(uint64 pos, bytes memory input) internal pure returns (uint64, uint64) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 8);")?;
                writeln!(out, "  uint64 value = abi.decode(input_red, (uint64));")?;
                writeln!(out, "  return (pos + 8, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            U128 => {
                writeln!(out, "function bcs_serialize_uint128(uint128 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_uint128(uint64 pos, bytes memory input) internal pure returns (uint64, uint128) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 16);")?;
                writeln!(out, "  uint128 value = abi.decode(input_red, (uint128));")?;
                writeln!(out, "  return (pos + 16, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            Char => {
                writeln!(out, "function bcs_serialize_bytes1(bytes1 input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bytes memory result = abi.encodePacked(input);")?;
                writeln!(out, "  return result;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_bytes1(uint64 pos, bytes memory input) internal pure returns (uint64, bytes1) {{")?;
                writeln!(out, "  bytes1 result = bytes1(input[pos]);")?;
                writeln!(out, "  return (pos + 1, result);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            Str => {
                writeln!(out, "function bcs_serialize_string(string memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_string(uint64 pos, bytes memory input) internal pure returns (uint64, string memory) {{")?;
                writeln!(out, "  string memory value = abi.decode(input, (string));")?;
                writeln!(out, "  uint256 len = bytes(value).length;")?;
                writeln!(out, "  require(len <= type(uint64).max, \"length exceeds uint64 range\");")?;
                writeln!(out, "  uint64 new_pos = pos + 8 + uint64(len);")?;
                writeln!(out, "  return (new_pos, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
            Bytes => {
                writeln!(out, "function bcs_serialize_bytes(bytes memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bytes memory block1 = abi.encodePacked(input.length);")?;
                writeln!(out, "  return abi.encodePacked(block1, input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_bytes(uint64 pos, bytes memory input) internal pure returns (uint64, bytes memory) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 8);")?;
                writeln!(out, "  uint64 len = abi.decode(input_red, (uint64));")?;
                writeln!(out, "  bytes memory value = slice_bytes(input, pos+8, len);")?;
                writeln!(out, "  return (pos + 8 + len, value);")?;
                writeln!(out, "}}")?;
                writeln!(out)?;
            },
        }
        Ok(())
    }
}


#[derive(Clone, Debug)]
enum SolFormat {
    /// One of the primitive types defined elsewhere
    Primitive(Primitive),
    /// A type defined here or elsewhere.
    TypeName(String),
    /// A sequence of objects.
    Seq(Box<SolFormat>),
    /// A simple solidity enum
    SimpleEnum { name: String, names: Vec<String> },
    /// A solidity struct. Used also to encapsulates Map and Tuple
    Struct { name: String, formats: Vec<Named<SolFormat>> },
    /// An option encapsulated as a solidity struct.
    Option(Box<SolFormat>),
    /// A Tuplearray encapsulated as a solidity struct.
    TupleArray { format: Box<SolFormat>, size: usize },
    /// A complex enum encapsulated as a solidity struct.
    Enum { name: String, formats: Vec<Named<Option<SolFormat>>> },
}

impl SolFormat
{
    pub fn code_name(&self) -> String {
        use SolFormat::*;
        if let Seq(format) = self {
            return format!("{}[]", format.code_name());
        }
        self.key_name()
    }

    pub fn key_name(&self) -> String {
        use SolFormat::*;
        match self {
            Primitive(primitive) => primitive.name(),
            TypeName(name) => name.to_string(),
            Option(format) => format!("opt_{}", format.key_name()),
            Seq(format) => format!("seq_{}", format.key_name()),
            TupleArray { format, size } => format!("tuplearray{}_{}", size, format.key_name()),
            Struct { name, formats: _ } => {
                name.to_string()
            },
            SimpleEnum { name, names: _ } => {
                name.to_string()
            },
            Enum { name, formats: _ } => {
                name.to_string()
            }
        }
    }

    pub fn output<T: std::io::Write>(&self, out: &mut IndentedWriter<T>, sol_registry: &SolRegistry) -> Result<()> {
        use SolFormat::*;
        match self {
            Primitive(primitive) => primitive.output(out)?,
            TypeName(_) => {
                // by definition for TypeName the code already exists
            },
            Option(format) => {
                let key_name = format.key_name();
                let code_name = format.code_name();
                let full_name = format!("opt_{}", key_name);
                let data_location = data_location(&format, sol_registry);
                writeln!(out, "struct {full_name} {{")?;
                writeln!(out, "  bool has_value;")?;
                writeln!(out, "  {code_name} value;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_serialize_{full_name}({full_name} memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bool has_value = input.has_value;")?;
                writeln!(out, "  bytes memory block1 = bcs_serialize_bool(has_value);")?;
                writeln!(out, "  if (has_value) {{")?;
                writeln!(out, "    bytes memory block2 = bcs_serialize_{key_name}(input.value);")?;
                writeln!(out, "    return abi.encodePacked(block1, block2);")?;
                writeln!(out, "  }} else {{")?;
                writeln!(out, "    return block1;")?;
                writeln!(out, "  }}")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_{full_name}(uint64 pos, bytes memory input) internal pure returns (uint64, {full_name} memory) {{")?;
                writeln!(out, "  uint64 new_pos;")?;
                writeln!(out, "  bool has_value;")?;
                writeln!(out, "  (new_pos, has_value) = bcs_deserialize_offset_bool(pos, input);")?;
                writeln!(out, "  {code_name}{data_location} value;")?;
                writeln!(out, "  if (has_value) {{")?;
                writeln!(out, "    (new_pos, value) = bcs_deserialize_offset_{key_name}(new_pos, input);")?;
                writeln!(out, "  }}")?;
                writeln!(out, "  return (new_pos, {full_name}(true, value));")?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, &full_name, &full_name, true)?;
            },
            Seq(format) => {
                let inner_key_name = format.key_name();
                let inner_code_name = format.code_name();
                let code_name = format!("{}[]", format.code_name());
                let key_name = format!("seq_{}", format.key_name());
                let data_location = data_location(format, sol_registry);
                writeln!(out, "function bcs_serialize_{key_name}({code_name} memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  uint256 len = input.length;")?;
                writeln!(out, "  bytes memory result = bcs_serialize_len(len);")?;
                writeln!(out, "  for (uint256 i=0; i<len; i++) {{")?;
                writeln!(out, "    result = abi.encodePacked(result, bcs_serialize_{inner_key_name}(input[i]));")?;
                writeln!(out, "  }}")?;
                writeln!(out, "  return result;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_{key_name}(uint64 pos, bytes memory input) internal pure returns (uint64, {code_name} memory) {{")?;
                writeln!(out, "  uint64 new_pos;")?;
                writeln!(out, "  uint256 len;")?;
                writeln!(out, "  {inner_code_name}[] memory result;")?;
                writeln!(out, "  result = new {inner_code_name}[](len);")?;
                writeln!(out, "  {inner_code_name}{data_location} value;")?;
                writeln!(out, "  (new_pos, len) = bcs_deserialize_offset_len(pos, input);")?;
                writeln!(out, "  for (uint256 i=0; i<len; i++) {{")?;
                writeln!(out, "    (new_pos, value) = bcs_deserialize_offset_{inner_key_name}(new_pos, input);")?;
                writeln!(out, "    result[i] = value;")?;
                writeln!(out, "  }}")?;
                writeln!(out, "  return (new_pos, result);")?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, &key_name, &code_name, true)?;
            }
            TupleArray { format, size } => {
                let inner_key_name = format.key_name();
                let inner_code_name = format.code_name();
                let struct_name = format!("tuplearray{}_{}", size, inner_key_name);
                writeln!(out, "struct {struct_name} {{")?;
                writeln!(out, "  {inner_code_name}[] values;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_serialize_{struct_name}({struct_name} memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bytes memory result;")?;
                writeln!(out, "  for (uint i=0; i<{size}; i++) {{")?;
                writeln!(out, "    result = abi.encodePacked(result, bcs_serialize_{inner_key_name}(input.values[i]));")?;
                writeln!(out, "  }}")?;
                writeln!(out, "  return result;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_{struct_name}(uint64 pos, bytes memory input) internal pure returns (uint64, {struct_name} memory) {{")?;
                writeln!(out, "  uint64 new_pos = pos;")?;
                writeln!(out, "  {inner_code_name} value;")?;
                writeln!(out, "  {inner_code_name}[] memory values;")?;
                writeln!(out, "  values = new {inner_code_name}[]({size});")?;
                writeln!(out, "  for (uint i=0; i<{size}; i++) {{")?;
                writeln!(out, "    (new_pos, value) = bcs_deserialize_offset_{inner_key_name}(new_pos, input);")?;
                writeln!(out, "    values[i] = value;")?;
                writeln!(out, "  }}")?;
                writeln!(out, "  return (new_pos, {struct_name}(values));")?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, &struct_name, &struct_name, true)?;
            }
            Struct { name, formats } => {
                writeln!(out, "struct {name} {{")?;
                for named_format in formats {
                    writeln!(out, "  {} {};", named_format.value.code_name(), safe_variable(&named_format.name))?;
                }
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_serialize_{name}({name} memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bytes memory result = bcs_serialize_{}(input.{});", &formats[0].value.key_name(), safe_variable(&formats[0].name))?;
                for named_format in &formats[1..] {
                    writeln!(out, "  result = abi.encodePacked(result, bcs_serialize_{}(input.{}));", named_format.value.key_name(), safe_variable(&named_format.name))?;
                }
                writeln!(out, "  return result;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_{name}(uint64 pos, bytes memory input) internal pure returns (uint64, {name} memory) {{")?;
                writeln!(out, "  uint64 new_pos = pos;")?;
                for named_format in formats {
                    let data_location = data_location(&named_format.value, sol_registry);
                    writeln!(out, "  {}{} {};", named_format.value.code_name(), data_location, safe_variable(&named_format.name))?;
                    writeln!(out, "  (new_pos, {}) = bcs_deserialize_offset_{}(new_pos, input);", safe_variable(&named_format.name), named_format.value.key_name())?;
                }
                writeln!(out, "  return (new_pos, {name}({}));", formats.into_iter().map(|named_format| safe_variable(&named_format.name)).collect::<Vec<_>>().join(", "))?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, &name, &name, true)?;
            },
            SimpleEnum { name, names } => {
                writeln!(out, "enum {name} {{ {} }}", names.join(", "))?;
                writeln!(out, "function bcs_serialize_{name}({name} input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  return abi.encodePacked(input);")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_{name}(uint64 pos, bytes memory input) internal pure returns (uint64, {name}) {{")?;
                writeln!(out, "  bytes memory input_red = slice_bytes(input, pos, 1);")?;
                writeln!(out, "  {name} value = abi.decode(input_red, ({name}));")?;
                writeln!(out, "  return (pos + 1, value);")?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, &name, &name, false)?;
            },
            Enum { name, formats } => {
                writeln!(out, "struct {name} {{")?;
                writeln!(out, "  uint64 choice;")?;
                for named_format in formats {
                    if let Some(format) = &named_format.value {
                        writeln!(out, "  {} {};", format.code_name(), named_format.name.to_snake_case())?;
                    }
                }
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_serialize_{name}({name} memory input) internal pure returns (bytes memory) {{")?;
                writeln!(out, "  bytes memory result = abi.encodePacked(input.choice);")?;
                for (idx, named_format) in formats.iter().enumerate() {
                    if let Some(format) = &named_format.value {
                        writeln!(out, "  if (input.choice == {idx}) {{")?;
                        writeln!(out, "    return abi.encodePacked(result, bcs_serialize_{}(input.{}));", format.key_name(), named_format.name.to_snake_case())?;
                        writeln!(out, "  }}")?;
                    }
                }
                writeln!(out, "  return result;")?;
                writeln!(out, "}}")?;
                writeln!(out, "function bcs_deserialize_offset_{name}(uint64 pos, bytes memory input) internal pure returns (uint64, {name} memory) {{")?;
                writeln!(out, "  uint64 new_pos;")?;
                writeln!(out, "  uint64 choice;")?;
                writeln!(out, "  (new_pos, choice) = bcs_deserialize_offset_uint64(pos, input);")?;
                let mut entries = Vec::new();
                for (idx, named_format) in formats.iter().enumerate() {
                    if let Some(format) = &named_format.value {
                        let data_location = data_location(format, sol_registry);
                        writeln!(out, "  {}{} {};", format.code_name(), data_location, named_format.name.to_snake_case())?;
                        writeln!(out, "  if (choice == {idx}) {{")?;
                        writeln!(out, "    (new_pos, {}) = bcs_deserialize_offset_{}(new_pos, input);", named_format.name.to_snake_case(), format.key_name())?;
                        writeln!(out, "  }}")?;
                        entries.push(named_format.name.to_snake_case());
                    }
                }
                writeln!(out, "  return (new_pos, {name}(choice, {}));", entries.join(", "))?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, &name, &name, true)?;
            },
        }
        Ok(())
    }

    fn get_dependency(&self) -> Vec<String> {
        use SolFormat::*;
        match self {
            Primitive(_) => vec![],
            TypeName(name) => vec![name.to_string()],
            Seq(format) => vec![format.key_name()],
            SimpleEnum { name: _, names: _ } => vec![],
            Struct { name: _, formats } => {
                formats.iter().map(|format| format.value.key_name()).collect()
            },
            Option(format) => vec![format.key_name()],
            TupleArray { format, size: _ } => vec![format.key_name()],
            Enum { name: _, formats } => {
                formats.iter().map(|format| match &format.value {
                    None => vec![],
                    Some(format) => vec![format.key_name()]
                }).flatten().collect()
            },
        }
    }
}

#[derive(Default)]
struct SolRegistry {
    names: HashMap<String, SolFormat>,
}

impl SolRegistry {
    fn insert(&mut self, sol_format: SolFormat) {
        let key_name = sol_format.key_name();
        if !matches!(sol_format, SolFormat::TypeName(_)) {
            self.names.insert(key_name, sol_format);
        }
    }

    fn has_circular_dependency(&self) -> bool {
        for start_key in self.names.keys() {
            let mut level = HashSet::<String>::new();
            level.insert(start_key.to_string());
            let mut total_dependency = level.clone();
            loop {
                let mut new_level = HashSet::new();
                for key in level {
                    for depend in self.names.get(&key).unwrap().get_dependency() {
                        if depend == *start_key {
                            return true;
                        }
                        if !total_dependency.contains(&depend) {
                            total_dependency.insert(depend.clone());
                            new_level.insert(depend);
                        }
                    }
                }
                if new_level.is_empty() {
                    break;
                }
                level = new_level;
            }
        }
        false
    }
}


fn need_memory(sol_format: &SolFormat, sol_registry: &SolRegistry) -> bool {
    use SolFormat::*;
    match sol_format {
        Primitive(primitive) => {
            use crate::solidity::Primitive;
            match primitive {
                Primitive::Unit => true,
                Primitive::Bytes => true,
                Primitive::Str => true,
                _ => false,
            }
        },
        TypeName(name) => {
            let mesg = format!("to find a matching entry for name={name}");
            let sol_format = sol_registry.names.get(name).expect(&mesg);
            need_memory(sol_format, sol_registry)
        },
        Option(_) => true,
        Seq(_) => true,
        TupleArray { format: _, size: _ } => true,
        Struct { name: _, formats: _ } => true,
        SimpleEnum { name: _, names: _ } => false,
        Enum { name: _, formats: _ } => true,
    }
}

fn data_location(sol_format: &SolFormat, sol_registry: &SolRegistry) -> String {
    get_data_location(need_memory(sol_format, sol_registry))
}



fn parse_format(registry: &mut SolRegistry, format: Format) -> SolFormat {
    use Format::*;
    let sol_format = match format {
        Variable(_) => panic!("variable is not supported in solidity"),
        TypeName(name) => SolFormat::TypeName(name),
        Unit => SolFormat::Primitive(Primitive::Unit),
        Bool => SolFormat::Primitive(Primitive::Bool),
        I8 => SolFormat::Primitive(Primitive::I8),
        I16 => SolFormat::Primitive(Primitive::I16),
        I32 => SolFormat::Primitive(Primitive::I32),
        I64 => SolFormat::Primitive(Primitive::I64),
        I128 => SolFormat::Primitive(Primitive::I128),
        U8 => SolFormat::Primitive(Primitive::U8),
        U16 => SolFormat::Primitive(Primitive::U16),
        U32 => SolFormat::Primitive(Primitive::U32),
        U64 => SolFormat::Primitive(Primitive::U64),
        U128 => SolFormat::Primitive(Primitive::U128),
        F32 => panic!("floating point is not supported in solidity"),
        F64 => panic!("floating point is not supported in solidity"),
        Char => SolFormat::Primitive(Primitive::Char),
        Str => SolFormat::Primitive(Primitive::Str),
        Bytes => SolFormat::Primitive(Primitive::Bytes),
        Option(format) => {
            let format = parse_format(registry, *format);
            SolFormat::Option(Box::new(format))
        },
        Seq(format) => {
            let sol_primitive = SolFormat::Primitive(Primitive::U64);
            registry.insert(sol_primitive);
            let format = parse_format(registry, *format);
            SolFormat::Seq(Box::new(format))
        },
        Map { key, value } => {
            let key = parse_format(registry, *key);
            let value = parse_format(registry, *value);
            let name = format!("key_values_{}_{}", key.key_name(), value.key_name());
            let formats = vec![Named { name: "key".into(), value: key }, Named { name: "value".into(), value }];
            let sol_format = SolFormat::Struct { name, formats };
            registry.insert(sol_format.clone());
            SolFormat::Seq(Box::new(sol_format))
        }
        Tuple(formats) => {
            let formats = formats.into_iter()
                .map(|format| parse_format(registry, format))
                .collect::<Vec<_>>();
            let name = format!("tuple_{}", formats.iter()
                               .map(|format| format.key_name()).collect::<Vec<_>>().join("_"));
            let formats = formats.into_iter().enumerate()
                .map(|(idx, format)| Named { name: format!("entry{idx}"), value: format })
                .collect();
            SolFormat::Struct { name, formats }
        },
        TupleArray { content, size } => {
            SolFormat::TupleArray { format: Box::new(parse_format(registry, *content)), size }
        },
    };
    registry.insert(sol_format.clone());
    sol_format
}


fn parse_struct_format(registry: &mut SolRegistry, name: String, formats: Vec<Named<Format>>) -> SolFormat {
    let formats = formats.into_iter()
        .map(|named_format| Named { name: named_format.name, value: parse_format(registry, named_format.value) })
        .collect();
    let sol_format = SolFormat::Struct { name, formats };
    registry.insert(sol_format.clone());
    sol_format
}

fn parse_container_format(registry: &mut SolRegistry, container_format: Named<ContainerFormat>) {
    use ContainerFormat::*;
    let name = container_format.name;
    let sol_format = match container_format.value {
        UnitStruct => panic!("UnitStruct is not supported in solidity"),
        NewTypeStruct(format) => {
            let format = Named { name: "value".to_string(), value: *format };
            let formats = vec![format];
            parse_struct_format(registry, name, formats)
        },
        TupleStruct(formats) => {
            assert!(!formats.is_empty(), "The TupleStruct should be non-trivial in solidity");
            let formats = formats.into_iter().enumerate()
                .map(|(idx, value)| Named { name: format!("entry{idx}"), value })
                .collect();
            parse_struct_format(registry, name, formats)
        },
        Struct(formats) => {
            assert!(!formats.is_empty(), "The struct should be non-trivial in solidity");
            parse_struct_format(registry, name, formats)
        },
        Enum(map) => {
            assert!(!map.is_empty(), "The enum should be non-trivial in solidity");
            let is_trivial = map.iter().all(|(_,v)| matches!(v.value, VariantFormat::Unit));
            if is_trivial {
                let names = map.into_iter().map(|(_,named_format)| named_format.name).collect();
                SolFormat::SimpleEnum { name, names }
            } else {
                let mut formats = Vec::new();
                for (_key, value) in map {
                    use VariantFormat::*;
                    let name_red = value.name;
                    let concat_name = format!("{}_{}", name, name_red);
                    let entry = match value.value {
                        VariantFormat::Unit => None,
                        NewType(format) => Some(parse_format(registry, *format)),
                        Tuple(formats) => {
                            let formats = formats.into_iter().enumerate()
                                .map(|(idx, value)| Named { name: format!("entry{idx}"), value })
                                .collect::<Vec<_>>();
                            Some(parse_struct_format(registry, concat_name, formats))
                        }
                        Struct(formats) => {
                            Some(parse_struct_format(registry, concat_name, formats))
                        }
                        Variable(_) => panic!("Variable is not supported for solidity")
                    };
                    let format = Named { name: name_red, value: entry };
                    formats.push(format);
                }
                SolFormat::Enum { name, formats }
            }
        },
    };
    registry.insert(sol_format);
}

impl<'a> CodeGenerator<'a> {
    /// Create a C++ code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.c_style_enums {
            panic!("C++ does not support generating c-style enums");
        }
        Self {
            config,
        }
    }

    pub fn output(
        &self,
        out: &mut dyn Write,
        registry: &Registry,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut emitter = SolEmitter {
            out: IndentedWriter::new(out, IndentConfig::Space(4)),
            generator: self,
        };

        emitter.output_preamble()?;
        emitter.output_open_contract()?;

        let mut sol_registry = SolRegistry::default();
        for (key, container_format) in registry {
            let container_format = Named { name: key.to_string(), value: container_format.clone() };
            parse_container_format(&mut sol_registry, container_format);
        }
        if sol_registry.has_circular_dependency() {
            panic!("solidity does not allow for circular dependencies");
        }
        for sol_format in sol_registry.names.values() {
            sol_format.output(&mut emitter.out, &sol_registry)?;
        }

        emitter.output_close_contract()?;
        writeln!(emitter.out)?;
        Ok(())
    }
}

impl<'a, T> SolEmitter<'a, T>
where
    T: std::io::Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        writeln!(self.out, "/// SPDX-License-Identifier: UNLICENSED")?;
        writeln!(
            self.out,
            r#"pragma solidity ^0.8.0;"#
        )?;
        writeln!(self.out, "function slice_bytes(bytes memory input, uint64 pos, uint64 len) pure returns (bytes memory) {{")?;
        writeln!(self.out, "  bytes memory result = new bytes(len);")?;
        writeln!(self.out, "  for (uint64 u=0; u<len; u++) {{")?;
        writeln!(self.out, "    result[u] = input[pos + u];")?;
        writeln!(self.out, "  }}")?;
        writeln!(self.out, "  return result;")?;
        writeln!(self.out, "}}")?;
        writeln!(self.out)?;
        writeln!(self.out, "function bcs_serialize_len(uint256 pos) pure returns (bytes memory) {{")?;
        writeln!(self.out, "  return abi.encodePacked(pos);")?;
        writeln!(self.out, "}}")?;
        writeln!(self.out, "function bcs_deserialize_offset_len(uint64 pos, bytes memory input) pure returns (uint64, uint256) {{")?;
        writeln!(self.out, "  bytes memory input_red = slice_bytes(input, pos, 32);")?;
        writeln!(self.out, "  uint256 value = abi.decode(input_red, (uint256));")?;
        writeln!(self.out, "  return (pos + 32, value);")?;
        writeln!(self.out, "}}")?;
        Ok(())
    }

    fn output_open_contract(&mut self) -> Result<()> {
        writeln!(
            self.out,
            "\ncontract {} {{",
            self.generator.config.module_name
        )?;
        self.out.indent();
        Ok(())
    }

    fn output_close_contract(&mut self) -> Result<()> {
        self.out.unindent();
        writeln!(
            self.out,
            "\n}} // end of contract {}",
            self.generator.config.module_name
        )?;
        Ok(())
    }


}

/// Installer for generated source files in C++.
pub struct Installer {
    install_dir: PathBuf,
}

impl Installer {
    pub fn new(install_dir: PathBuf) -> Self {
        Installer { install_dir }
    }

    fn create_header_file(&self, name: &str) -> Result<std::fs::File> {
        let dir_path = &self.install_dir;
        std::fs::create_dir_all(dir_path)?;
        std::fs::File::create(dir_path.join(name.to_string() + ".sol"))
    }

    fn runtime_installation_message(name: &str) {
        eprintln!("Not installing sources for published crate {}", name);
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_module(
        &self,
        config: &crate::CodeGeneratorConfig,
        registry: &Registry,
    ) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_header_file(&config.module_name)?;
        let generator = CodeGenerator::new(config);
        generator.output(&mut file, registry)
    }

    fn install_serde_runtime(&self) -> std::result::Result<(), Self::Error> {
        Self::runtime_installation_message("serde");
        Ok(())
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        Self::runtime_installation_message("bincode");
        Ok(())
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        Self::runtime_installation_message("bcs");
        Ok(())
    }
}

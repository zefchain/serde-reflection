// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig,
};
use heck::SnakeCase;
use phf::phf_set;
use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    io::{Result, Write},
    path::PathBuf,
};

/// Main configuration object for code-generation in solidity
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
}

/// Shared state for the code generation of a solidity source file.
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

fn output_generic_bcs_deserialize<T: std::io::Write>(
    out: &mut IndentedWriter<T>,
    key_name: &str,
    code_name: &str,
    need_memory: bool,
) -> Result<()> {
    let data_location = get_data_location(need_memory);
    writeln!(
        out,
        r#"
function bcs_deserialize_{key_name}(bytes memory input)
    internal
    pure
    returns ({code_name}{data_location})
{{
    uint256 new_pos;
    {code_name}{data_location} value;
    (new_pos, value) = bcs_deserialize_offset_{key_name}(0, input);
    require(new_pos == input.length, "incomplete deserialization");
    return value;
}}"#
    )?;
    Ok(())
}

static KEYWORDS: phf::Set<&str> = phf_set! {
    "abstract", "after", "alias", "anonymous",
    "as", "assembly", "break", "catch", "constant",
    "continue", "constructor", "contract", "delete",
    "do", "else", "emit", "enum", "error", "event",
    "external", "fallback", "for", "function", "if",
    "immutable", "import", "indexed", "interface",
    "internal", "is", "library", "mapping", "memory",
    "modifier", "new", "override", "payable", "pragma",
    "private", "public", "pure", "receive", "return",
    "returns", "revert", "storage", "struct", "throw",
    "try", "type", "unchecked", "using", "virtual",
    "view", "while", "addmod", "blockhash", "ecrecover",
    "keccak256", "mulmod", "sha256", "ripemd160",
    "block", "msg", "tx", "balance", "transfer", "send",
    "call", "delegatecall", "staticcall", "this",
    "super", "gwei", "finney", "szabo", "ether",
    "seconds", "minutes", "hours", "days", "weeks",
    "years", "wei", "hex", "address", "bool", "bytes",
    "string", "int", "int8", "int16", "int32", "int64",
    "int128", "int256", "uint", "uint8", "uint16",
    "uint32", "uint64", "uint128", "uint256",
    "bytes1", "bytes2", "bytes3", "bytes4", "bytes5",
    "bytes6", "bytes7", "bytes8", "bytes9", "bytes10",
    "bytes11", "bytes12", "bytes13", "bytes14", "bytes15",
    "bytes16", "bytes17", "bytes18", "bytes19", "bytes20",
    "bytes21", "bytes22", "bytes23", "bytes24", "bytes25",
    "bytes26", "bytes27", "bytes28", "bytes29", "bytes30",
    "bytes31", "bytes32"
};

fn safe_variable(s: &str) -> String {
    if KEYWORDS.contains(s) {
        s.to_owned() + "_"
    } else {
        s.to_string()
    }
}

/// Returns true if `s` is a valid Solidity identifier: matches
/// `[a-zA-Z_$][a-zA-Z0-9_$]*` and is not a reserved keyword.
fn is_solidity_identifier(s: &str) -> bool {
    if s.is_empty() || KEYWORDS.contains(s) {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' && first != '$' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

#[derive(Clone, Debug, PartialEq)]
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

    pub fn need_memory(&self) -> bool {
        matches!(self, Primitive::Unit | Primitive::Bytes | Primitive::Str)
    }

    pub fn output<T: std::io::Write>(&self, out: &mut IndentedWriter<T>) -> Result<()> {
        use Primitive::*;
        match self {
            Unit => writeln!(
                out,
                r#"
struct empty_struct {{
    int8 val;
}}

function bcs_serialize_empty_struct(empty_struct memory input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result;
    return result;
}}

function bcs_deserialize_offset_empty_struct(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, empty_struct memory)
{{
    int8 val = 0;
    return (pos, empty_struct(val));
}}"#
            )?,
            Bool => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_bool(bool input)
    internal
    pure
    returns (bytes memory)
{{
    return abi.encodePacked(input);
}}

function bcs_deserialize_offset_bool(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, bool)
{{
    uint8 val = uint8(input[pos]);
    bool result = false;
    if (val == 1) {{
        result = true;
    }} else {{
        require(val == 0);
    }}
    return (pos + 1, result);
}}"#
                )?;
            }
            I8 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_int8(int8 input)
    internal
    pure
    returns (bytes memory)
{{
    return abi.encodePacked(input);
}}

function bcs_deserialize_offset_int8(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, int8)
{{
    int16 val = int16(uint16(uint8(input[pos])));
    if (val < 128) {{
        return (pos + 1, int8(val));
    }} else {{
        return (pos + 1, int8(val - 256));
    }}
}}"#
                )?;
            }
            I16 => writeln!(
                out,
                r#"
function bcs_serialize_int16(int16 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(2);
    uint16 uinput;
    if (input >= 0) {{
        uinput = uint16(input);
    }} else {{
        int32 input_32 = int32(input) + 65536;
        uinput = uint16(uint32(input_32));
    }}
    return bcs_serialize_uint16(uinput);
}}

function bcs_deserialize_offset_int16(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, int16)
{{
    uint256 new_pos;
    uint16 uresult;
    (new_pos, uresult) = bcs_deserialize_offset_uint16(pos, input);
    int16 result;
    if (uresult < 32768) {{
        result = int16(uresult);
        return (new_pos, result);
    }} else {{
        int32 result_32 = int32(uint32(uresult)) - 65536;
        result = int16(result_32);
    }}
    return (new_pos, result);
}}"#
            )?,
            I32 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_int32(int32 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(4);
    uint32 uinput;
    if (input >= 0) {{
        uinput = uint32(input);
    }} else {{
        int64 input_64 = int64(input) + 4294967296;
        uinput = uint32(uint64(input_64));
    }}
    return bcs_serialize_uint32(uinput);
}}

function bcs_deserialize_offset_int32(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, int32)
{{
    uint256 new_pos;
    uint32 uresult;
    (new_pos, uresult) = bcs_deserialize_offset_uint32(pos, input);
    int32 result;
    if (uresult < 2147483648) {{
        result = int32(uresult);
        return (new_pos, result);
    }} else {{
        int64 result_64 = int64(uint64(uresult)) - 4294967296;
        result = int32(result_64);
    }}
    return (new_pos, result);
}}"#
                )?;
            }
            I64 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_int64(int64 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(8);
    uint64 uinput;
    if (input >= 0) {{
        uinput = uint64(input);
    }} else {{
        int128 input_128 = int128(input) + 18446744073709551616;
        uinput = uint64(uint128(input_128));
    }}
    return bcs_serialize_uint64(uinput);
}}

function bcs_deserialize_offset_int64(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, int64)
{{
    uint256 new_pos;
    uint64 uresult;
    (new_pos, uresult) = bcs_deserialize_offset_uint64(pos, input);
    int64 result;
    if (uresult < 9223372036854775808) {{
        result = int64(uresult);
        return (new_pos, result);
    }} else {{
        int128 result_128 = int128(uint128(uresult)) - 18446744073709551616;
        result = int64(result_128);
    }}
    return (new_pos, result);
}}"#
                )?;
            }
            I128 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_int128(int128 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(16);
    uint128 uinput;
    if (input >= 0) {{
        uinput = uint128(input);
    }} else {{
        int256 input_256 = int256(input) + 340282366920938463463374607431768211456;
        uinput = uint128(uint256(input_256));
    }}
    return bcs_serialize_uint128(uinput);
}}

function bcs_deserialize_offset_int128(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, int128)
{{
    uint256 new_pos;
    uint128 uresult;
    (new_pos, uresult) = bcs_deserialize_offset_uint128(pos, input);
    int128 result;
    if (uresult < 170141183460469231731687303715884105728) {{
        result = int128(uresult);
        return (new_pos, result);
    }} else {{
        int256 result_256 = int256(uint256(uresult)) - 340282366920938463463374607431768211456;
        result = int128(result_256);
    }}
    return (new_pos, result);
}}"#
                )?;
            }
            U8 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_uint8(uint8 input)
    internal
    pure
    returns (bytes memory)
{{
  return abi.encodePacked(input);
}}

function bcs_deserialize_offset_uint8(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, uint8)
{{
    uint8 value = uint8(input[pos]);
    return (pos + 1, value);
}}"#
                )?;
            }
            U16 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_uint16(uint16 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(2);
    uint16 value = input;
    result[0] = bytes1(uint8(value));
    value = value >> 8;
    result[1] = bytes1(uint8(value));
    return result;
}}

function bcs_deserialize_offset_uint16(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, uint16)
{{
    uint16 value = uint8(input[pos+1]);
    value = value << 8;
    value += uint8(input[pos]);
    return (pos + 2, value);
}}"#
                )?;
            }
            U32 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_uint32(uint32 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(4);
    uint32 value = input;
    result[0] = bytes1(uint8(value));
    for (uint i=1; i<4; i++) {{
        value = value >> 8;
        result[i] = bytes1(uint8(value));
    }}
    return result;
}}

function bcs_deserialize_offset_uint32(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, uint32)
{{
    uint32 value = uint8(input[pos + 3]);
    for (uint256 i=0; i<3; i++) {{
        value = value << 8;
        value += uint8(input[pos + 2 - i]);
    }}
    return (pos + 4, value);
}}"#
                )?;
            }
            U64 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_uint64(uint64 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(8);
    uint64 value = input;
    result[0] = bytes1(uint8(value));
    for (uint i=1; i<8; i++) {{
        value = value >> 8;
        result[i] = bytes1(uint8(value));
    }}
    return result;
}}

function bcs_deserialize_offset_uint64(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, uint64)
{{
    uint64 value = uint8(input[pos + 7]);
    for (uint256 i=0; i<7; i++) {{
        value = value << 8;
        value += uint8(input[pos + 6 - i]);
    }}
    return (pos + 8, value);
}}"#
                )?;
            }
            U128 => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_uint128(uint128 input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result = new bytes(16);
    uint128 value = input;
    result[0] = bytes1(uint8(value));
    for (uint i=1; i<16; i++) {{
        value = value >> 8;
        result[i] = bytes1(uint8(value));
    }}
    return result;
}}

function bcs_deserialize_offset_uint128(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, uint128)
{{
    uint128 value = uint8(input[pos + 15]);
    for (uint256 i=0; i<15; i++) {{
        value = value << 8;
        value += uint8(input[pos + 14 - i]);
    }}
    return (pos + 16, value);
}}"#
                )?;
            }
            Char => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_bytes1(bytes1 input)
    internal
    pure
    returns (bytes memory)
{{
    return abi.encodePacked(input);
}}

function bcs_deserialize_offset_bytes1(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, bytes1)
{{
    bytes1 result = bytes1(input[pos]);
    return (pos + 1, result);
}}"#
                )?;
            }
            Str => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_string(string memory input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory input_bytes = bytes(input);
    uint256 number_bytes = input_bytes.length;
    uint256 number_char = 0;
    uint256 pos = 0;
    while (true) {{
        if (uint8(input_bytes[pos]) < 128) {{
            number_char += 1;
        }}
        pos += 1;
        if (pos == number_bytes) {{
            break;
        }}
    }}
    bytes memory result_len = bcs_serialize_uleb128(number_char);
    return abi.encodePacked(result_len, input);
}}

function bcs_deserialize_offset_string(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, string memory)
{{
    uint256 len;
    uint256 new_pos;
    (new_pos, len) = bcs_deserialize_offset_uleb128(pos, input);
    uint256 shift = 0;
    for (uint256 i=0; i<len; i++) {{
        while (true) {{
            bytes1 val = input[new_pos + shift];
            shift += 1;
            if (uint8(val) < 128) {{
                break;
            }}
        }}
    }}
    bytes memory result_bytes = new bytes(shift);
    for (uint256 i=0; i<shift; i++) {{
        result_bytes[i] = input[new_pos + i];
    }}
    string memory result = string(result_bytes);
    return (new_pos + shift, result);
}}
"#
                )?;
            }
            Bytes => {
                writeln!(
                    out,
                    r#"
function bcs_serialize_bytes(bytes memory input)
    internal
    pure
    returns (bytes memory)
{{
    uint256 len = input.length;
    bytes memory result = bcs_serialize_uleb128(len);
    return abi.encodePacked(result, input);
}}

function bcs_deserialize_offset_bytes(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, bytes memory)
{{
    uint256 len;
    uint256 new_pos;
    (new_pos, len) = bcs_deserialize_offset_uleb128(pos, input);
    bytes memory result = new bytes(len);
    for (uint256 u=0; u<len; u++) {{
        result[u] = input[new_pos + u];
    }}
    return (new_pos + len, result);
}}"#
                )?;
            }
        }
        Ok(())
    }
}

/// One variant of a complex (struct-backed) enum.
///
/// `index` is the variant index from the original Serde registry (the
/// `BTreeMap` key in `ContainerFormat::Enum`), preserved verbatim so the
/// generated BCS encoding agrees with the source. `uleb128` is its ULEB128
/// encoding, precomputed at parse time so the generated Solidity can embed
/// the discriminant bytes as a `hex"..."` literal on the serialize side
/// without calling `bcs_serialize_uleb128`.
#[derive(Clone, Debug, PartialEq)]
struct EnumVariant {
    index: u64,
    uleb128: Vec<u8>,
    name: String,
    value: Option<SolFormat>,
}

fn uleb128_encode(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            bytes.push(byte);
            return bytes;
        }
        bytes.push(byte | 0x80);
    }
}

fn hex_literal(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(2 + bytes.len() * 2 + 1);
    s.push_str("hex\"");
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s.push('"');
    s
}

#[derive(Clone, Debug, PartialEq)]
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
    Struct {
        name: String,
        formats: Vec<Named<SolFormat>>,
    },
    /// An option encapsulated as a solidity struct.
    Option(Box<SolFormat>),
    /// A Tuplearray encapsulated as a solidity struct.
    TupleArray { format: Box<SolFormat>, size: usize },
    /// A complex enum encapsulated as a solidity struct.
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
    /// A Tuplearray of N U8 has the native type bytesN
    BytesN { size: usize },
    /// An option of boolean
    OptionBool,
}

impl SolFormat {
    pub fn key_name(&self) -> String {
        use SolFormat::*;
        match self {
            Primitive(primitive) => primitive.name(),
            TypeName(name) => name.to_string(),
            Option(format) => format!("opt_{}", format.key_name()),
            Seq(format) => format!("seq_{}", format.key_name()),
            TupleArray { format, size } => format!("tuplearray{}_{}", size, format.key_name()),
            Struct { name, formats: _ } => name.to_string(),
            SimpleEnum { name, names: _ } => name.to_string(),
            Enum { name, variants: _ } => name.to_string(),
            BytesN { size } => format!("bytes{size}"),
            OptionBool => "OptionBool".to_string(),
        }
    }

    pub fn output<T: std::io::Write>(
        &self,
        out: &mut IndentedWriter<T>,
        sol_registry: &SolRegistry,
    ) -> Result<()> {
        use SolFormat::*;
        match self {
            Primitive(primitive) => {
                primitive.output(out)?;
                let full_name = primitive.name();
                let need_memory = primitive.need_memory();
                output_generic_bcs_deserialize(out, &full_name, &full_name, need_memory)?;
            }
            TypeName(_) => {
                // by definition for TypeName the code already exists
            }
            Option(format) => {
                let key_name = format.key_name();
                let qualified_code_name = sol_registry.qualified_code_name(format);
                let full_name = format!("opt_{key_name}");
                let data_location = sol_registry.data_location(format);
                let ser_fn = sol_registry.qualified_fn_name("bcs_serialize", &key_name);
                let deser_fn = sol_registry.qualified_fn_name("bcs_deserialize_offset", &key_name);
                writeln!(
                    out,
                    r#"
struct {full_name} {{
    bool has_value;
    {qualified_code_name} value;
}}

function bcs_serialize_{full_name}({full_name} memory input)
    internal
    pure
    returns (bytes memory)
{{
    if (input.has_value) {{
        return abi.encodePacked(uint8(1), {ser_fn}(input.value));
    }} else {{
        return abi.encodePacked(uint8(0));
    }}
}}

function bcs_deserialize_offset_{full_name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {full_name} memory)
{{
    uint256 new_pos;
    bool has_value;
    (new_pos, has_value) = bcs_deserialize_offset_bool(pos, input);
    {qualified_code_name}{data_location} value;
    if (has_value) {{
        (new_pos, value) = {deser_fn}(new_pos, input);
    }}
    return (new_pos, {full_name}(has_value, value));
}}"#
                )?;
                output_generic_bcs_deserialize(out, &full_name, &full_name, true)?;
            }
            Seq(format) => {
                let inner_key_name = format.key_name();
                let qualified_inner_code_name = sol_registry.qualified_code_name(format);
                let code_name = format!("{qualified_inner_code_name}[]");
                let key_name = format!("seq_{inner_key_name}");
                let data_location = sol_registry.data_location(format);
                let inner_ser_fn = sol_registry.qualified_fn_name("bcs_serialize", &inner_key_name);
                let inner_deser_fn =
                    sol_registry.qualified_fn_name("bcs_deserialize_offset", &inner_key_name);
                writeln!(
                    out,
                    r#"
function bcs_serialize_{key_name}({code_name} memory input)
    internal
    pure
    returns (bytes memory)
{{
    uint256 len = input.length;
    bytes memory result = bcs_serialize_uleb128(len);
    for (uint256 i=0; i<len; i++) {{
        result = abi.encodePacked(result, {inner_ser_fn}(input[i]));
    }}
    return result;
}}

function bcs_deserialize_offset_{key_name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {code_name} memory)
{{
    uint256 len;
    uint256 new_pos;
    (new_pos, len) = bcs_deserialize_offset_uleb128(pos, input);
    {qualified_inner_code_name}[] memory result;
    result = new {qualified_inner_code_name}[](len);
    {qualified_inner_code_name}{data_location} value;
    for (uint256 i=0; i<len; i++) {{
        (new_pos, value) = {inner_deser_fn}(new_pos, input);
        result[i] = value;
    }}
    return (new_pos, result);
}}"#
                )?;
                output_generic_bcs_deserialize(out, &key_name, &code_name, true)?;
            }
            TupleArray { format, size } => {
                let inner_key_name = format.key_name();
                let qualified_inner_code_name = sol_registry.qualified_code_name(format);
                let struct_name = format!("tuplearray{size}_{inner_key_name}");
                let inner_ser_fn = sol_registry.qualified_fn_name("bcs_serialize", &inner_key_name);
                let inner_deser_fn =
                    sol_registry.qualified_fn_name("bcs_deserialize_offset", &inner_key_name);
                writeln!(
                    out,
                    r#"
struct {struct_name} {{
    {qualified_inner_code_name}[] values;
}}

function bcs_serialize_{struct_name}({struct_name} memory input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result;
    for (uint i=0; i<{size}; i++) {{
        result = abi.encodePacked(result, {inner_ser_fn}(input.values[i]));
    }}
    return result;
}}

function bcs_deserialize_offset_{struct_name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {struct_name} memory)
{{
    uint256 new_pos = pos;
    {qualified_inner_code_name} value;
    {qualified_inner_code_name}[] memory values;
    values = new {qualified_inner_code_name}[]({size});
    for (uint i=0; i<{size}; i++) {{
        (new_pos, value) = {inner_deser_fn}(new_pos, input);
        values[i] = value;
    }}
    return (new_pos, {struct_name}(values));
}}"#
                )?;
                output_generic_bcs_deserialize(out, &struct_name, &struct_name, true)?;
            }
            Struct { name, formats } => {
                writeln!(out)?;
                writeln!(out, "struct {name} {{")?;
                for named_format in formats {
                    writeln!(
                        out,
                        "    {} {};",
                        sol_registry.qualified_code_name(&named_format.value),
                        safe_variable(&named_format.name)
                    )?;
                }
                writeln!(
                    out,
                    r#"}}

function bcs_serialize_{name}({name} memory input)
    internal
    pure
    returns (bytes memory)
{{"#
                )?;
                for (index, named_format) in formats.iter().enumerate() {
                    let key_name = named_format.value.key_name();
                    let safe_name = safe_variable(&named_format.name);
                    let ser_fn = sol_registry.qualified_fn_name("bcs_serialize", &key_name);
                    let block = format!("{ser_fn}(input.{safe_name})");
                    let block = if formats.len() > 1 {
                        if index == 0 {
                            format!("bytes memory result = {block}")
                        } else if index < formats.len() - 1 {
                            format!("result = abi.encodePacked(result, {block})")
                        } else {
                            format!("return abi.encodePacked(result, {block})")
                        }
                    } else {
                        format!("return {block}")
                    };
                    writeln!(out, "    {block};")?;
                }
                writeln!(
                    out,
                    r#"}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name} memory)
{{
    uint256 new_pos;"#
                )?;
                for (index, named_format) in formats.iter().enumerate() {
                    let data_location = sol_registry.data_location(&named_format.value);
                    let qualified_code_name = sol_registry.qualified_code_name(&named_format.value);
                    let key_name = named_format.value.key_name();
                    let safe_name = safe_variable(&named_format.name);
                    let start_pos = if index == 0 { "pos" } else { "new_pos" };
                    let deser_fn =
                        sol_registry.qualified_fn_name("bcs_deserialize_offset", &key_name);
                    writeln!(out, "    {qualified_code_name}{data_location} {safe_name};")?;
                    writeln!(
                        out,
                        "    (new_pos, {safe_name}) = {deser_fn}({start_pos}, input);"
                    )?;
                }
                writeln!(
                    out,
                    "    return (new_pos, {name}({}));",
                    formats
                        .iter()
                        .map(|named_format| safe_variable(&named_format.name))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                writeln!(out, "}}")?;
                output_generic_bcs_deserialize(out, name, name, true)?;
            }
            SimpleEnum { name, names } => {
                let names_join = names.join(", ");
                let number_names = names.len();
                writeln!(
                    out,
                    r#"
enum {name} {{ {names_join} }}

function bcs_serialize_{name}({name} input)
    internal
    pure
    returns (bytes memory)
{{
    return bcs_serialize_uleb128(uint256(input));
}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name})
{{
    uint256 new_pos;
    uint256 choice;
    (new_pos, choice) = bcs_deserialize_offset_uleb128(pos, input);"#
                )?;
                for (idx, name_choice) in names.iter().enumerate() {
                    writeln!(
                        out,
                        r#"
    if (choice == {idx}) {{
        return (new_pos, {name}.{name_choice});
    }}"#
                    )?;
                }
                writeln!(
                    out,
                    r#"
    require(choice < {number_names});
}}"#
                )?;
                output_generic_bcs_deserialize(out, name, name, false)?;
            }
            Enum { name, variants } => {
                writeln!(
                    out,
                    r#"
struct {name} {{
    uint64 choice;"#
                )?;
                for variant in variants {
                    let variant_index = variant.index;
                    let variant_name = &variant.name;
                    writeln!(
                        out,
                        "    // choice={variant_index} corresponds to {variant_name}"
                    )?;
                    if let Some(format) = &variant.value {
                        let qualified_code_name = sol_registry.qualified_code_name(format);
                        let snake_name = safe_variable(&variant.name.to_snake_case());
                        writeln!(out, "    {qualified_code_name} {snake_name};")?;
                    }
                }
                writeln!(out, "}}")?;
                let mut entries = Vec::new();
                let mut type_vars = Vec::new();
                for variant in variants {
                    if let Some(format) = &variant.value {
                        let data_location = sol_registry.data_location(format);
                        let snake_name = safe_variable(&variant.name.to_snake_case());
                        let qualified_code_name = sol_registry.qualified_code_name(format);
                        let type_var = format!("{qualified_code_name}{data_location} {snake_name}");
                        type_vars.push(type_var);
                        entries.push(snake_name);
                    } else {
                        type_vars.push(String::new());
                    }
                }
                // If no variant carries a payload (sparse all-Unit enums or
                // all-Unit enums with >256 variants both reach this path), the
                // struct has only the `choice` field, so the suffix must be
                // empty — otherwise we'd emit a trailing-comma `Foo(x, )`.
                let entries_suffix = if entries.is_empty() {
                    String::new()
                } else {
                    format!(", {}", entries.join(", "))
                };
                for (slot, variant) in variants.iter().enumerate() {
                    let snake_name = variant.name.to_snake_case();
                    let type_var = &type_vars[slot];
                    let variant_index = variant.index;
                    writeln!(
                        out,
                        r#"
function {name}_case_{snake_name}({type_var})
    internal
    pure
    returns ({name} memory)
{{"#
                    )?;
                    for (i_slot, other_type_var) in type_vars.iter().enumerate() {
                        if !other_type_var.is_empty() && slot != i_slot {
                            writeln!(out, "    {other_type_var};")?;
                        }
                    }
                    writeln!(
                        out,
                        "    return {name}(uint64({variant_index}){entries_suffix});"
                    )?;
                    writeln!(out, "}}")?;
                }
                writeln!(
                    out,
                    r#"
function bcs_serialize_{name}({name} memory input)
    internal
    pure
    returns (bytes memory)
{{"#
                )?;
                for variant in variants {
                    let variant_index = variant.index;
                    let discriminant_hex = hex_literal(&variant.uleb128);
                    writeln!(out, "    if (input.choice == {variant_index}) {{")?;
                    if let Some(format) = &variant.value {
                        let key_name = format.key_name();
                        let snake_name = safe_variable(&variant.name.to_snake_case());
                        let ser_fn = sol_registry.qualified_fn_name("bcs_serialize", &key_name);
                        writeln!(
                            out,
                            "        return abi.encodePacked({discriminant_hex}, {ser_fn}(input.{snake_name}));"
                        )?;
                    } else {
                        writeln!(out, "        return {discriminant_hex};")?;
                    }
                    writeln!(out, "    }}")?;
                }
                writeln!(
                    out,
                    r#"    revert("invalid variant index");
}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name} memory)
{{
    uint256 new_pos;
    uint256 choice_raw;
    (new_pos, choice_raw) = bcs_deserialize_offset_uleb128(pos, input);
    require(choice_raw <= type(uint64).max, "variant index does not fit in uint64");
    uint64 choice = uint64(choice_raw);"#
                )?;
                let is_contiguous = variants
                    .iter()
                    .enumerate()
                    .all(|(i, v)| v.index == i as u64);
                let validity_check = if is_contiguous {
                    format!("choice < {}", variants.len())
                } else {
                    variants
                        .iter()
                        .map(|v| format!("choice == {}", v.index))
                        .collect::<Vec<_>>()
                        .join(" || ")
                };
                writeln!(out, "    require({validity_check}, \"invalid variant index\");")?;
                for variant in variants {
                    if let Some(format) = &variant.value {
                        let data_location = sol_registry.data_location(format);
                        let snake_name = safe_variable(&variant.name.to_snake_case());
                        let qualified_code_name = sol_registry.qualified_code_name(format);
                        let key_name = format.key_name();
                        let deser_fn =
                            sol_registry.qualified_fn_name("bcs_deserialize_offset", &key_name);
                        let variant_index = variant.index;
                        writeln!(
                            out,
                            "    {qualified_code_name}{data_location} {snake_name};"
                        )?;
                        writeln!(out, "    if (choice == {variant_index}) {{")?;
                        writeln!(
                            out,
                            "        (new_pos, {snake_name}) = {deser_fn}(new_pos, input);"
                        )?;
                        writeln!(out, "    }}")?;
                    }
                }
                writeln!(
                    out,
                    r#"    return (new_pos, {name}(choice{entries_suffix}));
}}"#
                )?;
                output_generic_bcs_deserialize(out, name, name, true)?;
            }
            BytesN { size } => {
                let name = format!("bytes{size}");
                writeln!(
                    out,
                    r#"
function bcs_serialize_{name}({name} input)
    internal
    pure
    returns (bytes memory)
{{
    return abi.encodePacked(input);
}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name})
{{
    {name} dest;
    assembly {{
        dest := mload(add(add(input, 0x20), pos))
    }}
    return (pos + {size}, dest);
}}"#
                )?;
            }
            OptionBool => {
                let name = "OptionBool";
                writeln!(
                    out,
                    r#"
enum {name} {{ None, True, False }}

function bcs_serialize_{name}({name} input)
    internal
    pure
    returns (bytes memory)
{{
    if (input == {name}.None) {{
        return abi.encodePacked(uint8(0));
    }}
    if (input == {name}.False) {{
        return abi.encodePacked(uint8(1), uint8(0));
    }}
    return abi.encodePacked(uint8(1), uint8(1));
}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name})
{{
    uint8 choice = uint8(input[pos]);
    if (choice == 0) {{
       return (pos + 1, {name}.None);
    }} else {{
        require(choice == 1);
        uint8 value = uint8(input[pos + 1]);
        if (value == 0) {{
            return (pos + 2, {name}.False);
        }} else {{
            require(value == 1);
            return (pos + 2, {name}.True);
        }}
    }}
}}"#
                )?;
                output_generic_bcs_deserialize(out, name, name, false)?;
            }
        }
        Ok(())
    }

    /// Returns the key_names of types this format's generated code calls into.
    /// Used by `locally_needed_types` to determine which types must be emitted.
    fn get_dependency(&self) -> Vec<String> {
        use SolFormat::*;
        match self {
            // Signed integer serializers delegate to their unsigned counterparts,
            // e.g. bcs_serialize_int32 calls bcs_serialize_uint32 internally.
            Primitive(p) => match p {
                crate::solidity::Primitive::I16 => vec!["uint16".into()],
                crate::solidity::Primitive::I32 => vec!["uint32".into()],
                crate::solidity::Primitive::I64 => vec!["uint64".into()],
                crate::solidity::Primitive::I128 => vec!["uint128".into()],
                _ => vec![],
            },
            TypeName(name) => vec![name.to_string()],
            Seq(format) => vec![format.key_name()],
            SimpleEnum { name: _, names: _ } => vec![],
            Struct { name: _, formats } => formats
                .iter()
                .map(|format| format.value.key_name())
                .collect(),
            // Option deserializer calls bcs_deserialize_offset_bool for the tag.
            Option(format) => vec![format.key_name(), "bool".to_string()],
            TupleArray { format, size: _ } => vec![format.key_name()],
            // Variant index bytes are precomputed (hex literal on serialize) and
            // decoded via the preamble's `bcs_deserialize_offset_uleb128` helper, so
            // the enum's own dependencies are just the payload-bearing variants.
            Enum { name: _, variants } => variants
                .iter()
                .flat_map(|variant| match &variant.value {
                    None => vec![],
                    Some(format) => vec![format.key_name()],
                })
                .collect(),
            BytesN { size: _ } => vec![],
            OptionBool => vec![],
        }
    }
}

#[derive(Default)]
struct SolRegistry {
    names: BTreeMap<String, SolFormat>,
    /// Maps external type key_names to their qualified module prefix.
    /// e.g., "Account" → "BridgeTypes"
    external_modules: HashMap<String, String>,
}

impl SolRegistry {
    fn insert(&mut self, sol_format: SolFormat) {
        let key_name = sol_format.key_name();
        // If we insert the signed version, then we also need the unsigned one internally
        match sol_format {
            SolFormat::Primitive(Primitive::I8) => {
                self.names.insert(key_name, sol_format);
                self.names
                    .insert("uint8".to_string(), SolFormat::Primitive(Primitive::U8));
            }
            SolFormat::Primitive(Primitive::I16) => {
                self.names.insert(key_name, sol_format);
                self.names
                    .insert("uint16".to_string(), SolFormat::Primitive(Primitive::U16));
            }
            SolFormat::Primitive(Primitive::I32) => {
                self.names.insert(key_name, sol_format);
                self.names
                    .insert("uint32".to_string(), SolFormat::Primitive(Primitive::U32));
            }
            SolFormat::Primitive(Primitive::I64) => {
                self.names.insert(key_name, sol_format);
                self.names
                    .insert("uint64".to_string(), SolFormat::Primitive(Primitive::U64));
            }
            SolFormat::Primitive(Primitive::I128) => {
                self.names.insert(key_name, sol_format);
                self.names
                    .insert("uint128".to_string(), SolFormat::Primitive(Primitive::U128));
            }
            SolFormat::TypeName(_) => {
                // Typename entries do not need to be inserted.
            }
            _ => {
                self.names.insert(key_name, sol_format);
            }
        }
    }

    /// Returns true if the type is defined in an external module.
    fn is_external(&self, key_name: &str) -> bool {
        self.external_modules.contains_key(key_name)
    }

    /// Qualifies a type name: "Account" → "BridgeTypes.Account" for external types,
    /// or returns the name unchanged for local types.
    fn qualified_type_name(&self, key_name: &str) -> String {
        match self.external_modules.get(key_name) {
            Some(module) => format!("{module}.{key_name}"),
            None => key_name.to_string(),
        }
    }

    /// Qualifies a function name: "bcs_serialize" + "Account" → "BridgeTypes.bcs_serialize_Account"
    /// for external types, or "bcs_serialize_Account" for local types.
    fn qualified_fn_name(&self, fn_prefix: &str, type_key: &str) -> String {
        match self.external_modules.get(type_key) {
            Some(module) => format!("{module}.{fn_prefix}_{type_key}"),
            None => format!("{fn_prefix}_{type_key}"),
        }
    }

    /// Qualifies a code_name, handling Seq types by qualifying the inner type.
    /// e.g., for external Account: "Account[]" → "BridgeTypes.Account[]"
    fn qualified_code_name(&self, format: &SolFormat) -> String {
        match format {
            SolFormat::Seq(inner) => format!("{}[]", self.qualified_code_name(inner)),
            other => self.qualified_type_name(&other.key_name()),
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
                    // Skip dependencies not in self.names (e.g. implicit primitive
                    // deps like "bool" or "uint8" that may not have been parsed).
                    // Primitives have no outgoing dependencies so can't form cycles.
                    let Some(name) = self.names.get(&key) else {
                        continue;
                    };
                    for depend in name.get_dependency() {
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

    fn parse_format(&mut self, format: Format) -> SolFormat {
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
                let format = self.parse_format(*format);
                if format == SolFormat::Primitive(Primitive::Bool) {
                    SolFormat::OptionBool
                } else {
                    SolFormat::Option(Box::new(format))
                }
            }
            Seq(format) => {
                let format = self.parse_format(*format);
                SolFormat::Seq(Box::new(format))
            }
            Map { key, value } => {
                let key = self.parse_format(*key);
                let value = self.parse_format(*value);
                let name = format!("key_values_{}_{}", key.key_name(), value.key_name());
                let formats = vec![
                    Named {
                        name: "key".into(),
                        value: key,
                    },
                    Named {
                        name: "value".into(),
                        value,
                    },
                ];
                let sol_format = SolFormat::Struct { name, formats };
                self.insert(sol_format.clone());
                SolFormat::Seq(Box::new(sol_format))
            }
            Tuple(formats) => {
                let formats = formats
                    .into_iter()
                    .map(|format| self.parse_format(format))
                    .collect::<Vec<_>>();
                let name = format!(
                    "tuple_{}",
                    formats
                        .iter()
                        .map(|format| format.key_name())
                        .collect::<Vec<_>>()
                        .join("_")
                );
                let formats = formats
                    .into_iter()
                    .enumerate()
                    .map(|(idx, format)| Named {
                        name: format!("entry{idx}"),
                        value: format,
                    })
                    .collect();
                SolFormat::Struct { name, formats }
            }
            TupleArray { content, size } => {
                let format = self.parse_format(*content);
                if (1..=32).contains(&size) && format == SolFormat::Primitive(Primitive::U8) {
                    SolFormat::BytesN { size }
                } else {
                    SolFormat::TupleArray {
                        format: Box::new(format),
                        size,
                    }
                }
            }
        };
        self.insert(sol_format.clone());
        sol_format
    }

    fn parse_struct_format(&mut self, name: String, formats: Vec<Named<Format>>) -> SolFormat {
        let formats = formats
            .into_iter()
            .map(|named_format| Named {
                name: named_format.name,
                value: self.parse_format(named_format.value),
            })
            .collect();
        let sol_format = SolFormat::Struct { name, formats };
        self.insert(sol_format.clone());
        sol_format
    }

    fn parse_container_format(&mut self, container_format: Named<ContainerFormat>) {
        use ContainerFormat::*;
        let name = container_format.name;
        let sol_format = match container_format.value {
            UnitStruct => panic!("UnitStruct is not supported in solidity"),
            NewTypeStruct(format) => {
                let format = Named {
                    name: "value".to_string(),
                    value: *format,
                };
                let formats = vec![format];
                self.parse_struct_format(name, formats)
            }
            TupleStruct(formats) => {
                assert!(
                    !formats.is_empty(),
                    "The TupleStruct should be non-trivial in solidity"
                );
                let formats = formats
                    .into_iter()
                    .enumerate()
                    .map(|(idx, value)| Named {
                        name: format!("entry{idx}"),
                        value,
                    })
                    .collect();
                self.parse_struct_format(name, formats)
            }
            Struct(formats) => {
                assert!(
                    !formats.is_empty(),
                    "The struct should be non-trivial in solidity"
                );
                self.parse_struct_format(name, formats)
            }
            Enum(map) => {
                assert!(
                    !map.is_empty(),
                    "The enum should be non-trivial in solidity"
                );
                let is_trivial = map
                    .iter()
                    .all(|(_, v)| matches!(v.value, VariantFormat::Unit));
                // The native-Solidity SimpleEnum path uses the Solidity enum's
                // own discriminant (positional 0..N-1), so it can only be used
                // when the BCS variant indices are exactly 0,1,...,N-1.
                let is_contiguous = map
                    .keys()
                    .enumerate()
                    .all(|(i, k)| u64::from(*k) == i as u64);
                if is_trivial && is_contiguous && map.len() <= 256 {
                    // Solidity native enums are limited to 256 entries.
                    let names = map
                        .into_values()
                        .map(|named_format| named_format.name)
                        .collect();
                    SolFormat::SimpleEnum { name, names }
                } else {
                    let mut variants = Vec::new();
                    for (key, value) in map {
                        use VariantFormat::*;
                        let variant_name = value.name;
                        let concat_name = format!("{name}_{variant_name}");
                        let entry = match value.value {
                            VariantFormat::Unit => None,
                            NewType(format) => Some(self.parse_format(*format)),
                            Tuple(formats) => {
                                let formats = formats
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, value)| Named {
                                        name: format!("entry{idx}"),
                                        value,
                                    })
                                    .collect::<Vec<_>>();
                                Some(self.parse_struct_format(concat_name, formats))
                            }
                            Struct(formats) => Some(self.parse_struct_format(concat_name, formats)),
                            Variable(_) => panic!("Variable is not supported for solidity"),
                        };
                        let index = u64::from(key);
                        variants.push(EnumVariant {
                            index,
                            uleb128: uleb128_encode(index),
                            name: variant_name,
                            value: entry,
                        });
                    }
                    SolFormat::Enum { name, variants }
                }
            }
        };
        self.insert(sol_format);
    }

    fn need_memory(&self, sol_format: &SolFormat) -> bool {
        use SolFormat::*;
        match sol_format {
            Primitive(primitive) => primitive.need_memory(),
            TypeName(name) => {
                let mesg = format!("to find a matching entry for name={name}");
                let sol_format = self.names.get(name).expect(&mesg);
                self.need_memory(sol_format)
            }
            Option(_) => true,
            Seq(_) => true,
            TupleArray { format: _, size: _ } => true,
            Struct {
                name: _,
                formats: _,
            } => true,
            SimpleEnum { name: _, names: _ } => false,
            Enum {
                name: _,
                variants: _,
            } => true,
            BytesN { size: _ } => false,
            OptionBool => false,
        }
    }

    fn data_location(&self, sol_format: &SolFormat) -> String {
        get_data_location(self.need_memory(sol_format))
    }

    /// Returns the set of type key_names that are transitively needed by at least
    /// one non-external registry type. Types only reachable through external types
    /// are excluded.
    ///
    /// The algorithm works in two steps:
    /// 1. Identify "roots": non-external registry keys that are not depended upon
    ///    by any other type in the registry. These are the entry-point types that
    ///    the generated library exposes.
    /// 2. Walk forward from roots through non-external dependencies to find all
    ///    transitively needed types.
    ///
    /// This ensures that types only reachable through external types (e.g. a helper
    /// struct used exclusively by an imported type) are not emitted locally.
    fn locally_needed_types(&self, registry_keys: &[&str]) -> HashSet<String> {
        // Collect all type key_names that are depended upon by ANY type in the
        // registry (including internal types like variant structs and Seq wrappers).
        let mut has_dependents: HashSet<String> = HashSet::new();
        for format in self.names.values() {
            for dep in format.get_dependency() {
                has_dependents.insert(dep);
            }
        }

        // Seed with non-external registry keys that are true roots
        // (not a dependency of any other type).
        let mut needed = HashSet::new();
        let mut frontier: Vec<String> = registry_keys
            .iter()
            .filter(|k| !self.is_external(k) && !has_dependents.contains(**k))
            .map(|k| k.to_string())
            .collect();
        while let Some(key) = frontier.pop() {
            if !needed.insert(key.clone()) {
                continue;
            }
            if let Some(format) = self.names.get(&key) {
                for dep in format.get_dependency() {
                    if !needed.contains(&dep) && !self.is_external(&dep) {
                        frontier.push(dep);
                    }
                }
            }
        }
        needed
    }

    /// Returns true if any locally-needed type uses the `bcs_serialize_uleb128` /
    /// `bcs_deserialize_offset_uleb128` preamble functions: Seq, Str, Bytes use them
    /// for length prefixes, and Enum / SimpleEnum use them for variant indices.
    fn needs_preamble(&self, needed: &HashSet<String>) -> bool {
        needed.iter().any(|key| {
            self.names.get(key).is_some_and(|f| {
                matches!(
                    f,
                    SolFormat::Seq(_)
                        | SolFormat::Primitive(Primitive::Str)
                        | SolFormat::Primitive(Primitive::Bytes)
                        | SolFormat::Enum { .. }
                        | SolFormat::SimpleEnum { .. }
                )
            })
        })
    }
}

impl<'a> CodeGenerator<'a> {
    /// Create a solidity code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.enums.c_style {
            panic!("Solidity does not support generating c-style enums");
        }
        Self { config }
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

        let mut sol_registry = SolRegistry::default();
        // External definitions: module name → list of type names defined in that module.
        // Types present in both the registry and external_definitions are treated as
        // external — they are imported rather than generated locally. This is the
        // intended usage: the registry describes ALL types (for dependency analysis),
        // while external_definitions marks which ones live in another module.
        for (module_name, type_names) in &self.config.external_definitions {
            assert!(
                is_solidity_identifier(module_name),
                "external module name {module_name:?} is not a valid Solidity identifier \
                 (must match [a-zA-Z_$][a-zA-Z0-9_$]* and not be a reserved keyword)"
            );
            for type_name in type_names {
                sol_registry
                    .external_modules
                    .insert(type_name.clone(), module_name.clone());
            }
        }
        for (key, container_format) in registry {
            let container_format = Named {
                name: key.to_string(),
                value: container_format.clone(),
            };
            sol_registry.parse_container_format(container_format);
        }
        if sol_registry.has_circular_dependency() {
            panic!("solidity does not allow for circular dependencies");
        }
        let registry_keys: Vec<&str> = registry.keys().map(|k| k.as_str()).collect();
        let needed = sol_registry.locally_needed_types(&registry_keys);

        emitter.output_license()?;
        emitter.output_imports()?;
        emitter.output_open_library()?;
        if sol_registry.needs_preamble(&needed) {
            emitter.output_preamble()?;
        }
        for sol_format in sol_registry.names.values() {
            let key = sol_format.key_name();
            if needed.contains(&key) && !sol_registry.is_external(&key) {
                sol_format.output(&mut emitter.out, &sol_registry)?;
            }
        }

        emitter.output_close_library()?;
        Ok(())
    }
}

impl<'a, T> SolEmitter<'a, T>
where
    T: std::io::Write,
{
    fn output_license(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"/// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;"#
        )?;
        Ok(())
    }

    fn output_imports(&mut self) -> Result<()> {
        let modules: BTreeSet<&str> = self
            .generator
            .config
            .external_definitions
            .keys()
            .map(|s| s.as_str())
            .collect();
        for module in modules {
            writeln!(self.out, "import \"{module}.sol\";")?;
        }
        Ok(())
    }

    fn output_preamble(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
function bcs_serialize_uleb128(uint256 x)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result;
    bytes1 entry;
    while (true) {{
        if (x < 128) {{
            entry = bytes1(uint8(x));
            return abi.encodePacked(result, entry);
        }} else {{
            uint256 xb = x >> 7;
            uint256 remainder = x - (xb << 7);
            require(remainder < 128);
            entry = bytes1(uint8(remainder) + 128);
            result = abi.encodePacked(result, entry);
            x = xb;
        }}
    }}
    require(false, "This line is unreachable");
    return result;
}}

function bcs_deserialize_offset_uleb128(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, uint256)
{{
    uint256 idx = 0;
    while (true) {{
        if (uint8(input[pos + idx]) < 128) {{
            uint256 result = 0;
            uint256 power = 1;
            for (uint256 u=0; u<idx; u++) {{
                uint8 val = uint8(input[pos + u]) - 128;
                result += power * uint256(val);
                power *= 128;
            }}
            result += power * uint8(input[pos + idx]);
            return (pos + idx + 1, result);
        }}
        idx += 1;
    }}
    require(false, "This line is unreachable");
    return (0,0);
}}"#
        )?;
        Ok(())
    }

    fn output_open_library(&mut self) -> Result<()> {
        writeln!(
            self.out,
            "\nlibrary {} {{",
            self.generator.config.module_name
        )?;
        self.out.indent();
        Ok(())
    }

    fn output_close_library(&mut self) -> Result<()> {
        self.out.unindent();
        writeln!(
            self.out,
            "\n}} // end of library {}",
            self.generator.config.module_name
        )?;
        Ok(())
    }
}

/// Installer for generated source files in solidity
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
        eprintln!("Not installing sources for published crate {name}");
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

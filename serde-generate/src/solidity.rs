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
    collections::{BTreeMap, HashSet},
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
    bytes memory result_len = bcs_serialize_len(number_char);
    return abi.encodePacked(result_len, input);
}}

function bcs_deserialize_offset_string(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, string memory)
{{
    uint256 len;
    uint256 new_pos;
    (new_pos, len) = bcs_deserialize_offset_len(pos, input);
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
    bytes memory result = bcs_serialize_len(len);
    return abi.encodePacked(result, input);
}}

function bcs_deserialize_offset_bytes(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, bytes memory)
{{
    uint256 len;
    uint256 new_pos;
    (new_pos, len) = bcs_deserialize_offset_len(pos, input);
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
        formats: Vec<Named<Option<SolFormat>>>,
    },
    /// A Tuplearray of N U8 has the native type bytesN
    BytesN { size: usize },
    /// An option of boolean
    OptionBool,
}

impl SolFormat {
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
            Struct { name, formats: _ } => name.to_string(),
            SimpleEnum { name, names: _ } => name.to_string(),
            Enum { name, formats: _ } => name.to_string(),
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
                let code_name = format.code_name();
                let full_name = format!("opt_{}", key_name);
                let data_location = sol_registry.data_location(format);
                writeln!(
                    out,
                    r#"
struct {full_name} {{
    bool has_value;
    {code_name} value;
}}

function bcs_serialize_{full_name}({full_name} memory input)
    internal
    pure
    returns (bytes memory)
{{
    if (input.has_value) {{
        return abi.encodePacked(uint8(1), bcs_serialize_{key_name}(input.value));
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
    {code_name}{data_location} value;
    if (has_value) {{
        (new_pos, value) = bcs_deserialize_offset_{key_name}(new_pos, input);
    }}
    return (new_pos, {full_name}(has_value, value));
}}"#
                )?;
                output_generic_bcs_deserialize(out, &full_name, &full_name, true)?;
            }
            Seq(format) => {
                let inner_key_name = format.key_name();
                let inner_code_name = format.code_name();
                let code_name = format!("{}[]", format.code_name());
                let key_name = format!("seq_{}", format.key_name());
                let data_location = sol_registry.data_location(format);
                writeln!(
                    out,
                    r#"
function bcs_serialize_{key_name}({code_name} memory input)
    internal
    pure
    returns (bytes memory)
{{
    uint256 len = input.length;
    bytes memory result = bcs_serialize_len(len);
    for (uint256 i=0; i<len; i++) {{
        result = abi.encodePacked(result, bcs_serialize_{inner_key_name}(input[i]));
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
    (new_pos, len) = bcs_deserialize_offset_len(pos, input);
    {inner_code_name}[] memory result;
    result = new {inner_code_name}[](len);
    {inner_code_name}{data_location} value;
    for (uint256 i=0; i<len; i++) {{
        (new_pos, value) = bcs_deserialize_offset_{inner_key_name}(new_pos, input);
        result[i] = value;
    }}
    return (new_pos, result);
}}"#
                )?;
                output_generic_bcs_deserialize(out, &key_name, &code_name, true)?;
            }
            TupleArray { format, size } => {
                let inner_key_name = format.key_name();
                let inner_code_name = format.code_name();
                let struct_name = format!("tuplearray{}_{}", size, inner_key_name);
                writeln!(
                    out,
                    r#"
struct {struct_name} {{
    {inner_code_name}[] values;
}}

function bcs_serialize_{struct_name}({struct_name} memory input)
    internal
    pure
    returns (bytes memory)
{{
    bytes memory result;
    for (uint i=0; i<{size}; i++) {{
        result = abi.encodePacked(result, bcs_serialize_{inner_key_name}(input.values[i]));
    }}
    return result;
}}

function bcs_deserialize_offset_{struct_name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {struct_name} memory)
{{
    uint256 new_pos = pos;
    {inner_code_name} value;
    {inner_code_name}[] memory values;
    values = new {inner_code_name}[]({size});
    for (uint i=0; i<{size}; i++) {{
        (new_pos, value) = bcs_deserialize_offset_{inner_key_name}(new_pos, input);
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
                        named_format.value.code_name(),
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
                    let block = format!("bcs_serialize_{key_name}(input.{safe_name})");
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
                    let code_name = named_format.value.code_name();
                    let key_name = named_format.value.key_name();
                    let safe_name = safe_variable(&named_format.name);
                    let start_pos = if index == 0 { "pos" } else { "new_pos" };
                    writeln!(out, "    {code_name}{data_location} {safe_name};")?;
                    writeln!(out, "    (new_pos, {safe_name}) = bcs_deserialize_offset_{key_name}({start_pos}, input);")?;
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
    return abi.encodePacked(input);
}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name})
{{
    uint8 choice = uint8(input[pos]);"#
                )?;
                for (idx, name_choice) in names.iter().enumerate() {
                    writeln!(
                        out,
                        r#"
    if (choice == {idx}) {{
        return (pos + 1, {name}.{name_choice});
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
            Enum { name, formats } => {
                let number_names = formats.len();
                writeln!(
                    out,
                    r#"
struct {name} {{
    uint8 choice;"#
                )?;
                for (idx, named_format) in formats.iter().enumerate() {
                    let name = named_format.name.clone();
                    writeln!(out, "    // choice={idx} corresponds to {name}")?;
                    if let Some(format) = &named_format.value {
                        let code_name = format.code_name();
                        let snake_name = safe_variable(&named_format.name.to_snake_case());
                        writeln!(out, "    {code_name} {snake_name};")?;
                    }
                }
                writeln!(out, "}}")?;
                let mut entries = Vec::new();
                let mut type_vars = Vec::new();
                for named_format in formats {
                    if let Some(format) = &named_format.value {
                        let data_location = sol_registry.data_location(format);
                        let snake_name = safe_variable(&named_format.name.to_snake_case());
                        let code_name = format.code_name();
                        let type_var = format!("{code_name}{data_location} {snake_name}");
                        type_vars.push(type_var);
                        entries.push(snake_name);
                    } else {
                        type_vars.push(String::new());
                    }
                }
                let entries = entries.join(", ");
                for (choice, named_format_i) in formats.iter().enumerate() {
                    let snake_name = named_format_i.name.to_snake_case();
                    let type_var = &type_vars[choice];
                    writeln!(
                        out,
                        r#"
function {name}_case_{snake_name}({type_var})
    internal
    pure
    returns ({name} memory)
{{"#
                    )?;
                    for (i_choice, type_var) in type_vars.iter().enumerate() {
                        if !type_var.is_empty() && choice != i_choice {
                            writeln!(out, "    {type_var};")?;
                        }
                    }
                    writeln!(out, "    return {name}(uint8({choice}), {entries});")?;
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
                for (idx, named_format) in formats.iter().enumerate() {
                    if let Some(format) = &named_format.value {
                        let key_name = format.key_name();
                        let snake_name = safe_variable(&named_format.name.to_snake_case());
                        writeln!(out, "    if (input.choice == {idx}) {{")?;
                        writeln!(out, "        return abi.encodePacked(input.choice, bcs_serialize_{key_name}(input.{snake_name}));")?;
                        writeln!(out, "    }}")?;
                    }
                }
                writeln!(
                    out,
                    r#"    return abi.encodePacked(input.choice);
}}

function bcs_deserialize_offset_{name}(uint256 pos, bytes memory input)
    internal
    pure
    returns (uint256, {name} memory)
{{
    uint256 new_pos;
    uint8 choice;
    (new_pos, choice) = bcs_deserialize_offset_uint8(pos, input);"#
                )?;
                let mut entries = Vec::new();
                for (idx, named_format) in formats.iter().enumerate() {
                    if let Some(format) = &named_format.value {
                        let data_location = sol_registry.data_location(format);
                        let snake_name = safe_variable(&named_format.name.to_snake_case());
                        let code_name = format.code_name();
                        let key_name = format.key_name();
                        writeln!(out, "    {code_name}{data_location} {snake_name};")?;
                        writeln!(out, "    if (choice == {idx}) {{")?;
                        writeln!(out, "        (new_pos, {snake_name}) = bcs_deserialize_offset_{key_name}(new_pos, input);")?;
                        writeln!(out, "    }}")?;
                        entries.push(snake_name);
                    }
                }
                writeln!(out, "    require(choice < {number_names});")?;
                let entries = entries.join(", ");
                writeln!(
                    out,
                    r#"    return (new_pos, {name}(choice, {entries}));
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

    fn get_dependency(&self) -> Vec<String> {
        use SolFormat::*;
        match self {
            Primitive(_) => vec![],
            TypeName(name) => vec![name.to_string()],
            Seq(format) => vec![format.key_name()],
            SimpleEnum { name: _, names: _ } => vec![],
            Struct { name: _, formats } => formats
                .iter()
                .map(|format| format.value.key_name())
                .collect(),
            Option(format) => vec![format.key_name()],
            TupleArray { format, size: _ } => vec![format.key_name()],
            Enum { name: _, formats } => formats
                .iter()
                .flat_map(|format| match &format.value {
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
                assert!(map.len() < 256, "The enum should have at most 256 entries");
                let is_trivial = map
                    .iter()
                    .all(|(_, v)| matches!(v.value, VariantFormat::Unit));
                if is_trivial {
                    let names = map
                        .into_values()
                        .map(|named_format| named_format.name)
                        .collect();
                    SolFormat::SimpleEnum { name, names }
                } else {
                    let choice_sol_format = SolFormat::Primitive(Primitive::U8);
                    self.insert(choice_sol_format);
                    let mut formats = Vec::new();
                    for (_key, value) in map {
                        use VariantFormat::*;
                        let name_red = value.name;
                        let concat_name = format!("{}_{}", name, name_red);
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
                        let format = Named {
                            name: name_red,
                            value: entry,
                        };
                        formats.push(format);
                    }
                    SolFormat::Enum { name, formats }
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
                formats: _,
            } => true,
            BytesN { size: _ } => false,
            OptionBool => false,
        }
    }

    fn data_location(&self, sol_format: &SolFormat) -> String {
        get_data_location(self.need_memory(sol_format))
    }
}

impl<'a> CodeGenerator<'a> {
    /// Create a solidity code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.c_style_enums {
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

        emitter.output_license()?;
        emitter.output_open_library()?;
        emitter.output_preamble()?;

        let mut sol_registry = SolRegistry::default();
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
        for sol_format in sol_registry.names.values() {
            sol_format.output(&mut emitter.out, &sol_registry)?;
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

    fn output_preamble(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"
function bcs_serialize_len(uint256 x)
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

function bcs_deserialize_offset_len(uint256 pos, bytes memory input)
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

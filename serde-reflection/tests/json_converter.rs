// Copyright (c) Zefchain Labs, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration tests for the json_converter module
#![cfg(feature = "json")]

use bincode::Options;
use serde::de::{DeserializeSeed, IntoDeserializer};
use serde_json::{json, Value};
use serde_reflection::{
    json_converter::{
        DeserializationContext, DeserializationEnvironment, EmptyEnvironment, SerializationContext,
    },
    ContainerFormat, Format, Named, Registry, VariantFormat,
};
use std::collections::BTreeMap;

// Helper function to deserialize JSON with a given format
fn deserialize_json(format: Format, registry: &Registry, json_str: &str) -> Result<Value, String> {
    let value: serde_json::Value = serde_json::from_str(json_str).unwrap();
    let context = DeserializationContext {
        format,
        registry,
        environment: &EmptyEnvironment,
    };

    let deserializer = value.into_deserializer();
    context
        .deserialize(deserializer)
        .map_err(|e: serde_json::Error| e.to_string())
}

// Helper function to serialize JSON with a given format
fn serialize_json(value: &Value, format: &Format, registry: &Registry) -> Result<Value, String> {
    let context = SerializationContext {
        value,
        format,
        registry,
        environment: &EmptyEnvironment,
    };

    serde_json::to_value(&context).map_err(|e| e.to_string())
}

// ============================================================================
// Primitive Type Tests
// ============================================================================

#[test]
fn test_primitive_bool() {
    let registry = Registry::new();

    let result = deserialize_json(Format::Bool, &registry, "true");
    assert_eq!(result.unwrap(), json!(true));

    let result = deserialize_json(Format::Bool, &registry, "false");
    assert_eq!(result.unwrap(), json!(false));
}

#[test]
fn test_primitive_integers() {
    let registry = Registry::new();

    // Test various integer types
    let test_cases = vec![
        (Format::I8, "42", json!(42)),
        (Format::I16, "1000", json!(1000)),
        (Format::I32, "100000", json!(100000)),
        (
            Format::I64,
            "9223372036854775807",
            json!(9223372036854775807i64),
        ),
        (Format::U8, "255", json!(255)),
        (Format::U16, "65535", json!(65535)),
        (Format::U32, "4294967295", json!(4294967295u64)),
        (
            Format::U64,
            "18446744073709551615",
            json!(18446744073709551615u64),
        ),
    ];

    for (format, input, expected) in test_cases {
        let result = deserialize_json(format, &registry, input);
        assert_eq!(result.unwrap(), expected);
    }
}

#[test]
fn test_primitive_i128_u128() {
    let registry = Registry::new();

    // Small values should be numbers
    let result = deserialize_json(Format::I128, &registry, "42");
    assert_eq!(result.unwrap(), json!(42));

    let result = deserialize_json(Format::U128, &registry, "100");
    assert_eq!(result.unwrap(), json!(100));

    // Very large values should become strings
    // (this depends on the implementation - adjust if needed)
}

#[test]
fn test_primitive_floats() {
    let registry = Registry::new();

    // F32 has limited precision, so we need to account for rounding
    let result = deserialize_json(Format::F32, &registry, "12.34");
    let value = result.unwrap();
    let num = value.as_f64().unwrap();
    assert!((num - 12.34).abs() < 0.01);

    let result = deserialize_json(Format::F64, &registry, "12.718281828");
    assert_eq!(result.unwrap(), json!(12.718281828));
}

#[test]
fn test_primitive_string() {
    let registry = Registry::new();

    let result = deserialize_json(Format::Str, &registry, r#""hello world""#);
    assert_eq!(result.unwrap(), json!("hello world"));
}

#[test]
fn test_primitive_char() {
    let registry = Registry::new();

    let result = deserialize_json(Format::Char, &registry, r#""a""#);
    assert_eq!(result.unwrap(), json!("a"));
}

#[test]
fn test_primitive_unit() {
    let registry = Registry::new();

    let result = deserialize_json(Format::Unit, &registry, "null");
    assert_eq!(result.unwrap(), json!(null));
}

#[test]
fn test_bytes() {
    let registry = Registry::new();

    let result = deserialize_json(Format::Bytes, &registry, "[1, 2, 3, 255]");
    assert_eq!(result.unwrap(), json!([1, 2, 3, 255]));
}

// ============================================================================
// Container Type Tests
// ============================================================================

#[test]
fn test_option_some() {
    let registry = Registry::new();
    let format = Format::Option(Box::new(Format::I32));

    let result = deserialize_json(format, &registry, "42");
    assert_eq!(result.unwrap(), json!(42));
}

#[test]
fn test_option_none() {
    let registry = Registry::new();
    let format = Format::Option(Box::new(Format::I32));

    let result = deserialize_json(format, &registry, "null");
    assert_eq!(result.unwrap(), json!(null));
}

#[test]
fn test_seq() {
    let registry = Registry::new();
    let format = Format::Seq(Box::new(Format::I32));

    let result = deserialize_json(format, &registry, "[1, 2, 3, 4, 5]");
    assert_eq!(result.unwrap(), json!([1, 2, 3, 4, 5]));
}

#[test]
fn test_seq_empty() {
    let registry = Registry::new();
    let format = Format::Seq(Box::new(Format::Str));

    let result = deserialize_json(format, &registry, "[]");
    assert_eq!(result.unwrap(), json!([]));
}

#[test]
fn test_map() {
    let registry = Registry::new();
    let format = Format::Map {
        key: Box::new(Format::Str),
        value: Box::new(Format::I32),
    };

    let result = deserialize_json(format, &registry, r#"{"a": 1, "b": 2, "c": 3}"#);
    assert_eq!(result.unwrap(), json!({"a": 1, "b": 2, "c": 3}));
}

#[test]
fn test_tuple() {
    let registry = Registry::new();
    let format = Format::Tuple(vec![Format::I32, Format::Str, Format::Bool]);

    let result = deserialize_json(format, &registry, r#"[42, "hello", true]"#);
    assert_eq!(result.unwrap(), json!([42, "hello", true]));
}

#[test]
fn test_tuple_array() {
    let registry = Registry::new();
    let format = Format::TupleArray {
        content: Box::new(Format::U8),
        size: 4,
    };

    let result = deserialize_json(format, &registry, "[1, 2, 3, 4]");
    assert_eq!(result.unwrap(), json!([1, 2, 3, 4]));
}

// ============================================================================
// ContainerFormat Tests
// ============================================================================

#[test]
fn test_unit_struct() {
    let mut registry = Registry::new();
    registry.insert("UnitStruct".to_string(), ContainerFormat::UnitStruct);

    let format = Format::TypeName("UnitStruct".to_string());
    let result = deserialize_json(format, &registry, "null");
    assert_eq!(result.unwrap(), json!(null));
}

#[test]
fn test_newtype_struct() {
    let mut registry = Registry::new();
    registry.insert(
        "Age".to_string(),
        ContainerFormat::NewTypeStruct(Box::new(Format::U32)),
    );

    let format = Format::TypeName("Age".to_string());
    let result = deserialize_json(format, &registry, "25");
    assert_eq!(result.unwrap(), json!(25));
}

#[test]
fn test_tuple_struct() {
    let mut registry = Registry::new();
    registry.insert(
        "Point".to_string(),
        ContainerFormat::TupleStruct(vec![Format::I32, Format::I32]),
    );

    let format = Format::TypeName("Point".to_string());
    let result = deserialize_json(format, &registry, "[10, 20]");
    assert_eq!(result.unwrap(), json!([10, 20]));
}

#[test]
fn test_struct() {
    let mut registry = Registry::new();
    registry.insert(
        "Person".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "name".to_string(),
                value: Format::Str,
            },
            Named {
                name: "age".to_string(),
                value: Format::U32,
            },
        ]),
    );

    let format = Format::TypeName("Person".to_string());
    let result = deserialize_json(format, &registry, r#"{"name": "Alice", "age": 30}"#);
    assert_eq!(result.unwrap(), json!({"name": "Alice", "age": 30}));
}

#[test]
fn test_struct_with_sequence_format() {
    let mut registry = Registry::new();
    registry.insert(
        "Person".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "name".to_string(),
                value: Format::Str,
            },
            Named {
                name: "age".to_string(),
                value: Format::U32,
            },
        ]),
    );

    let format = Format::TypeName("Person".to_string());
    // Some formats serialize structs as sequences
    let result = deserialize_json(format, &registry, r#"["Bob", 25]"#);
    assert_eq!(result.unwrap(), json!({"name": "Bob", "age": 25}));
}

// ============================================================================
// Enum Tests
// ============================================================================

#[test]
fn test_enum_unit_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "None".to_string(),
            value: VariantFormat::Unit,
        },
    );
    registry.insert("Option".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Option".to_string());
    let result = deserialize_json(format, &registry, r#"{"None": null}"#);
    assert_eq!(result.unwrap(), json!({"None": null}));
}

#[test]
fn test_enum_newtype_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Some".to_string(),
            value: VariantFormat::NewType(Box::new(Format::I32)),
        },
    );
    registry.insert("Option".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Option".to_string());
    let result = deserialize_json(format, &registry, r#"{"Some": 42}"#);
    assert_eq!(result.unwrap(), json!({"Some": 42}));
}

#[test]
fn test_enum_tuple_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Point".to_string(),
            value: VariantFormat::Tuple(vec![Format::I32, Format::I32]),
        },
    );
    registry.insert("Shape".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Shape".to_string());
    let result = deserialize_json(format, &registry, r#"{"Point": [10, 20]}"#);
    assert_eq!(result.unwrap(), json!({"Point": [10, 20]}));
}

#[test]
fn test_enum_struct_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Rectangle".to_string(),
            value: VariantFormat::Struct(vec![
                Named {
                    name: "width".to_string(),
                    value: Format::U32,
                },
                Named {
                    name: "height".to_string(),
                    value: Format::U32,
                },
            ]),
        },
    );
    registry.insert("Shape".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Shape".to_string());
    let result = deserialize_json(
        format,
        &registry,
        r#"{"Rectangle": {"width": 100, "height": 50}}"#,
    );
    assert_eq!(
        result.unwrap(),
        json!({"Rectangle": {"width": 100, "height": 50}})
    );
}

#[test]
fn test_enum_multiple_variants() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Unit".to_string(),
            value: VariantFormat::Unit,
        },
    );
    variants.insert(
        1,
        Named {
            name: "Newtype".to_string(),
            value: VariantFormat::NewType(Box::new(Format::U16)),
        },
    );
    variants.insert(
        2,
        Named {
            name: "Tuple".to_string(),
            value: VariantFormat::Tuple(vec![Format::U16, Format::Bool]),
        },
    );
    variants.insert(
        3,
        Named {
            name: "Struct".to_string(),
            value: VariantFormat::Struct(vec![Named {
                name: "a".to_string(),
                value: Format::U32,
            }]),
        },
    );
    registry.insert("E".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("E".to_string());

    // Test each variant
    let result = deserialize_json(format.clone(), &registry, r#"{"Unit": null}"#);
    assert_eq!(result.unwrap(), json!({"Unit": null}));

    let result = deserialize_json(format.clone(), &registry, r#"{"Newtype": 42}"#);
    assert_eq!(result.unwrap(), json!({"Newtype": 42}));

    let result = deserialize_json(format.clone(), &registry, r#"{"Tuple": [100, true]}"#);
    assert_eq!(result.unwrap(), json!({"Tuple": [100, true]}));

    let result = deserialize_json(format.clone(), &registry, r#"{"Struct": {"a": 999}}"#);
    assert_eq!(result.unwrap(), json!({"Struct": {"a": 999}}));
}

// ============================================================================
// Nested Structure Tests
// ============================================================================

#[test]
fn test_nested_structs() {
    let mut registry = Registry::new();

    // Define Address struct
    registry.insert(
        "Address".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "street".to_string(),
                value: Format::Str,
            },
            Named {
                name: "city".to_string(),
                value: Format::Str,
            },
        ]),
    );

    // Define Person struct with nested Address
    registry.insert(
        "Person".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "name".to_string(),
                value: Format::Str,
            },
            Named {
                name: "address".to_string(),
                value: Format::TypeName("Address".to_string()),
            },
        ]),
    );

    let format = Format::TypeName("Person".to_string());
    let json_str = r#"{
        "name": "Alice",
        "address": {
            "street": "123 Main St",
            "city": "Springfield"
        }
    }"#;

    let value = deserialize_json(format, &registry, json_str).unwrap();
    assert_eq!(value["name"], json!("Alice"));
    assert_eq!(value["address"]["street"], json!("123 Main St"));
    assert_eq!(value["address"]["city"], json!("Springfield"));
}

#[test]
fn test_seq_of_structs() {
    let mut registry = Registry::new();

    registry.insert(
        "Point".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "x".to_string(),
                value: Format::I32,
            },
            Named {
                name: "y".to_string(),
                value: Format::I32,
            },
        ]),
    );

    let format = Format::Seq(Box::new(Format::TypeName("Point".to_string())));
    let json_str = r#"[
        {"x": 0, "y": 0},
        {"x": 10, "y": 20},
        {"x": -5, "y": 15}
    ]"#;

    let value = deserialize_json(format, &registry, json_str).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 3);
}

#[test]
fn test_option_of_struct() {
    let mut registry = Registry::new();

    registry.insert(
        "Config".to_string(),
        ContainerFormat::Struct(vec![Named {
            name: "enabled".to_string(),
            value: Format::Bool,
        }]),
    );

    let format = Format::Option(Box::new(Format::TypeName("Config".to_string())));

    // Test Some case
    let result = deserialize_json(format.clone(), &registry, r#"{"enabled": true}"#);
    assert_eq!(result.unwrap(), json!({"enabled": true}));

    // Test None case
    let result = deserialize_json(format, &registry, "null");
    assert_eq!(result.unwrap(), json!(null));
}

// ============================================================================
// Error Cases Tests
// ============================================================================

#[test]
fn test_error_unknown_type() {
    let registry = Registry::new();
    let format = Format::TypeName("UnknownType".to_string());

    let result = deserialize_json(format, &registry, "42");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("UnknownType"));
}

#[test]
fn test_error_unknown_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "KnownVariant".to_string(),
            value: VariantFormat::Unit,
        },
    );
    registry.insert("E".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("E".to_string());
    let result = deserialize_json(format, &registry, r#"{"UnknownVariant": null}"#);
    assert!(result.is_err());
}

#[test]
fn test_error_variable_format() {
    let _registry = Registry::new();
    // Variables cannot be deserialized directly
    // This would require creating a Variable which is not publicly constructible
    // So we skip this test as it's an internal invariant
}

// ============================================================================
// Custom Environment Tests
// ============================================================================

struct CustomEnvironment {
    external_value: Value,
}

impl serde_reflection::json_converter::SymbolTableEnvironment for CustomEnvironment {}

impl<'de> DeserializationEnvironment<'de> for CustomEnvironment {
    fn deserialize<D>(&self, name: String, _deserializer: D) -> Result<Value, String>
    where
        D: serde::Deserializer<'de>,
    {
        if name == "ExternalType" {
            Ok(self.external_value.clone())
        } else {
            Err(format!("Unknown external type: {name}"))
        }
    }
}

#[test]
fn test_custom_environment() {
    let registry = Registry::new();
    let env = CustomEnvironment {
        external_value: json!({"custom": "data"}),
    };

    let format = Format::TypeName("ExternalType".to_string());
    let value: serde_json::Value = serde_json::from_str("null").unwrap();

    let context = DeserializationContext {
        format,
        registry: &registry,
        environment: &env,
    };

    let deserializer = value.into_deserializer();
    let result = context.deserialize(deserializer);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json!({"custom": "data"}));
}

// ============================================================================
// Round-trip Tests
// ============================================================================

#[test]
fn test_roundtrip_complex_structure() {
    let mut registry = Registry::new();

    // Define a complex enum
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "User".to_string(),
            value: VariantFormat::Struct(vec![
                Named {
                    name: "id".to_string(),
                    value: Format::U64,
                },
                Named {
                    name: "name".to_string(),
                    value: Format::Str,
                },
                Named {
                    name: "tags".to_string(),
                    value: Format::Seq(Box::new(Format::Str)),
                },
            ]),
        },
    );
    variants.insert(
        1,
        Named {
            name: "Guest".to_string(),
            value: VariantFormat::Unit,
        },
    );

    registry.insert("Entity".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Entity".to_string());
    let json_str = r#"{
        "User": {
            "id": 12345,
            "name": "Alice",
            "tags": ["admin", "verified"]
        }
    }"#;

    let value = deserialize_json(format, &registry, json_str).unwrap();

    // Verify the structure
    let user_obj = value.get("User").unwrap();
    assert_eq!(user_obj["id"], json!(12345));
    assert_eq!(user_obj["name"], json!("Alice"));
    assert_eq!(user_obj["tags"], json!(["admin", "verified"]));
}

// ============================================================================
// Serialization Tests
// ============================================================================

// ============================================================================
// Primitive Type Serialization Tests
// ============================================================================

#[test]
fn test_serialize_primitive_integers() {
    let registry = Registry::new();

    let test_cases = vec![
        (Format::I8, json!(42), json!(42)),
        (Format::I16, json!(1000), json!(1000)),
        (Format::I32, json!(100000), json!(100000)),
        (
            Format::I64,
            json!(9223372036854775807i64),
            json!(9223372036854775807i64),
        ),
        (Format::U8, json!(255), json!(255)),
        (Format::U16, json!(65535), json!(65535)),
        (Format::U32, json!(4294967295u64), json!(4294967295u64)),
        (
            Format::U64,
            json!(18446744073709551615u64),
            json!(18446744073709551615u64),
        ),
    ];

    for (format, input, expected) in test_cases {
        let result = serialize_json(&input, &format, &registry);
        assert_eq!(result.unwrap(), expected);
    }
}

#[test]
fn test_serialize_primitive_i128_u128() {
    let registry = Registry::new();

    // Small values should be numbers
    let result = serialize_json(&json!(42), &Format::I128, &registry);
    assert_eq!(result.unwrap(), json!(42));

    let result = serialize_json(&json!(100), &Format::U128, &registry);
    assert_eq!(result.unwrap(), json!(100));

    // Large values as strings (within i64 range will still be numbers)
    let result = serialize_json(&json!("9223372036854775807"), &Format::I128, &registry);
    assert_eq!(result.unwrap(), json!(9223372036854775807i64));
}

#[test]
fn test_serialize_primitive_floats() {
    let registry = Registry::new();

    // F32 has limited precision, so we need to account for rounding
    let result = serialize_json(&json!(12.34), &Format::F32, &registry);
    let value = result.unwrap();
    let num = value.as_f64().unwrap();
    assert!((num - 12.34).abs() < 0.01);

    let result = serialize_json(&json!(12.718281828), &Format::F64, &registry);
    assert_eq!(result.unwrap(), json!(12.718281828));
}

#[test]
fn test_serialize_bytes() {
    let registry = Registry::new();

    let result = serialize_json(&json!([1, 2, 3, 255]), &Format::Bytes, &registry);
    assert_eq!(result.unwrap(), json!([1, 2, 3, 255]));
}

// ============================================================================
// Container Type Serialization Tests
// ============================================================================

#[test]
fn test_serialize_option_none() {
    let registry = Registry::new();
    let format = Format::Option(Box::new(Format::I32));

    let result = serialize_json(&json!(null), &format, &registry);
    assert_eq!(result.unwrap(), json!(null));
}

#[test]
fn test_serialize_seq_empty() {
    let registry = Registry::new();
    let format = Format::Seq(Box::new(Format::Str));

    let result = serialize_json(&json!([]), &format, &registry);
    assert_eq!(result.unwrap(), json!([]));
}

#[test]
fn test_serialize_map() {
    let registry = Registry::new();
    let format = Format::Map {
        key: Box::new(Format::Str),
        value: Box::new(Format::I32),
    };

    let result = serialize_json(&json!({"a": 1, "b": 2, "c": 3}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"a": 1, "b": 2, "c": 3}));
}

#[test]
fn test_serialize_tuple_array() {
    let registry = Registry::new();
    let format = Format::TupleArray {
        content: Box::new(Format::U8),
        size: 4,
    };

    let result = serialize_json(&json!([1, 2, 3, 4]), &format, &registry);
    assert_eq!(result.unwrap(), json!([1, 2, 3, 4]));
}

// ============================================================================
// ContainerFormat Serialization Tests
// ============================================================================

#[test]
fn test_serialize_unit_struct() {
    let mut registry = Registry::new();
    registry.insert("UnitStruct".to_string(), ContainerFormat::UnitStruct);

    let format = Format::TypeName("UnitStruct".to_string());
    let result = serialize_json(&json!(null), &format, &registry);
    assert_eq!(result.unwrap(), json!(null));
}

#[test]
fn test_serialize_newtype_struct() {
    let mut registry = Registry::new();
    registry.insert(
        "Age".to_string(),
        ContainerFormat::NewTypeStruct(Box::new(Format::U32)),
    );

    let format = Format::TypeName("Age".to_string());
    let result = serialize_json(&json!(25), &format, &registry);
    assert_eq!(result.unwrap(), json!(25));
}

#[test]
fn test_serialize_tuple_struct() {
    let mut registry = Registry::new();
    registry.insert(
        "Point".to_string(),
        ContainerFormat::TupleStruct(vec![Format::I32, Format::I32]),
    );

    let format = Format::TypeName("Point".to_string());
    let result = serialize_json(&json!([10, 20]), &format, &registry);
    assert_eq!(result.unwrap(), json!([10, 20]));
}

#[test]
fn test_serialize_struct() {
    let mut registry = Registry::new();
    registry.insert(
        "Person".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "name".to_string(),
                value: Format::Str,
            },
            Named {
                name: "age".to_string(),
                value: Format::U32,
            },
        ]),
    );

    let format = Format::TypeName("Person".to_string());
    let result = serialize_json(&json!({"name": "Alice", "age": 30}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"name": "Alice", "age": 30}));
}

// ============================================================================
// Enum Serialization Tests
// ============================================================================

#[test]
fn test_serialize_enum_unit_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "None".to_string(),
            value: VariantFormat::Unit,
        },
    );
    registry.insert("Option".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Option".to_string());
    let result = serialize_json(&json!({"None": null}), &format, &registry);
    // When serializing to JSON, serde_json represents unit variants as just the name
    assert_eq!(result.unwrap(), json!("None"));
}

#[test]
fn test_serialize_enum_newtype_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Some".to_string(),
            value: VariantFormat::NewType(Box::new(Format::I32)),
        },
    );
    registry.insert("Option".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Option".to_string());
    let result = serialize_json(&json!({"Some": 42}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"Some": 42}));
}

#[test]
fn test_serialize_enum_tuple_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Point".to_string(),
            value: VariantFormat::Tuple(vec![Format::I32, Format::I32]),
        },
    );
    registry.insert("Shape".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Shape".to_string());
    let result = serialize_json(&json!({"Point": [10, 20]}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"Point": [10, 20]}));
}

#[test]
fn test_serialize_enum_struct_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Rectangle".to_string(),
            value: VariantFormat::Struct(vec![
                Named {
                    name: "width".to_string(),
                    value: Format::U32,
                },
                Named {
                    name: "height".to_string(),
                    value: Format::U32,
                },
            ]),
        },
    );
    registry.insert("Shape".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("Shape".to_string());
    let result = serialize_json(
        &json!({"Rectangle": {"width": 100, "height": 50}}),
        &format,
        &registry,
    );
    assert_eq!(
        result.unwrap(),
        json!({"Rectangle": {"width": 100, "height": 50}})
    );
}

#[test]
fn test_serialize_enum_multiple_variants() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "Unit".to_string(),
            value: VariantFormat::Unit,
        },
    );
    variants.insert(
        1,
        Named {
            name: "Newtype".to_string(),
            value: VariantFormat::NewType(Box::new(Format::U16)),
        },
    );
    variants.insert(
        2,
        Named {
            name: "Tuple".to_string(),
            value: VariantFormat::Tuple(vec![Format::U16, Format::Bool]),
        },
    );
    variants.insert(
        3,
        Named {
            name: "Struct".to_string(),
            value: VariantFormat::Struct(vec![Named {
                name: "a".to_string(),
                value: Format::U32,
            }]),
        },
    );
    registry.insert("E".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("E".to_string());

    // Test each variant
    let result = serialize_json(&json!({"Unit": null}), &format, &registry);
    // When serializing to JSON, serde_json represents unit variants as just the name
    assert_eq!(result.unwrap(), json!("Unit"));

    let result = serialize_json(&json!({"Newtype": 42}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"Newtype": 42}));

    let result = serialize_json(&json!({"Tuple": [100, true]}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"Tuple": [100, true]}));

    let result = serialize_json(&json!({"Struct": {"a": 999}}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"Struct": {"a": 999}}));
}

// ============================================================================
// Nested Structure Serialization Tests
// ============================================================================

#[test]
fn test_serialize_nested_structs() {
    let mut registry = Registry::new();

    // Define Address struct
    registry.insert(
        "Address".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "street".to_string(),
                value: Format::Str,
            },
            Named {
                name: "city".to_string(),
                value: Format::Str,
            },
        ]),
    );

    // Define Person struct with nested Address
    registry.insert(
        "Person".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "name".to_string(),
                value: Format::Str,
            },
            Named {
                name: "address".to_string(),
                value: Format::TypeName("Address".to_string()),
            },
        ]),
    );

    let format = Format::TypeName("Person".to_string());
    let json_value = json!({
        "name": "Alice",
        "address": {
            "street": "123 Main St",
            "city": "Springfield"
        }
    });

    let value = serialize_json(&json_value, &format, &registry).unwrap();
    assert_eq!(value["name"], json!("Alice"));
    assert_eq!(value["address"]["street"], json!("123 Main St"));
    assert_eq!(value["address"]["city"], json!("Springfield"));
}

#[test]
fn test_serialize_seq_of_structs() {
    let mut registry = Registry::new();

    registry.insert(
        "Point".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "x".to_string(),
                value: Format::I32,
            },
            Named {
                name: "y".to_string(),
                value: Format::I32,
            },
        ]),
    );

    let format = Format::Seq(Box::new(Format::TypeName("Point".to_string())));
    let json_value = json!([
        {"x": 0, "y": 0},
        {"x": 10, "y": 20},
        {"x": -5, "y": 15}
    ]);

    let value = serialize_json(&json_value, &format, &registry).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 3);
}

#[test]
fn test_serialize_option_of_struct() {
    let mut registry = Registry::new();

    registry.insert(
        "Config".to_string(),
        ContainerFormat::Struct(vec![Named {
            name: "enabled".to_string(),
            value: Format::Bool,
        }]),
    );

    let format = Format::Option(Box::new(Format::TypeName("Config".to_string())));

    // Test Some case
    let result = serialize_json(&json!({"enabled": true}), &format, &registry);
    assert_eq!(result.unwrap(), json!({"enabled": true}));

    // Test None case
    let result = serialize_json(&json!(null), &format, &registry);
    assert_eq!(result.unwrap(), json!(null));
}

// ============================================================================
// Round-trip Serialization Tests
// ============================================================================

#[test]
fn test_roundtrip_primitives() {
    let registry = Registry::new();

    let test_cases = vec![
        (Format::Bool, json!(true)),
        (Format::I32, json!(42)),
        (Format::U64, json!(18446744073709551615u64)),
        (Format::F64, json!(12.34)),
        (Format::Str, json!("hello")),
        (Format::Char, json!("x")),
        (Format::Unit, json!(null)),
    ];

    for (format, value) in test_cases {
        let serialized = serialize_json(&value, &format, &registry).unwrap();
        assert_eq!(serialized, value);
    }
}

#[test]
fn test_roundtrip_containers() {
    let registry = Registry::new();

    // Seq
    let format = Format::Seq(Box::new(Format::I32));
    let value = json!([1, 2, 3]);
    let serialized = serialize_json(&value, &format, &registry).unwrap();
    assert_eq!(serialized, value);

    // Option
    let format = Format::Option(Box::new(Format::Str));
    let value = json!("test");
    let serialized = serialize_json(&value, &format, &registry).unwrap();
    assert_eq!(serialized, value);

    // Tuple
    let format = Format::Tuple(vec![Format::I32, Format::Str]);
    let value = json!([42, "hello"]);
    let serialized = serialize_json(&value, &format, &registry).unwrap();
    assert_eq!(serialized, value);
}

// ============================================================================
// Bincode Round-trip Tests
// ============================================================================

#[test]
fn test_bincode_roundtrip_struct() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    // Use tracer to extract the format
    let mut tracer = serde_reflection::Tracer::new(serde_reflection::TracerConfig::default());
    let (format, _) = tracer
        .trace_type::<Point>(&serde_reflection::Samples::new())
        .unwrap();
    let registry = tracer.registry().unwrap();

    // Serialize with bincode
    let config = bincode::DefaultOptions::new();
    let point = Point { x: 10, y: 20 };
    let encoded = config.serialize(&point).unwrap();

    // Deserialize to JSON using json_converter
    let mut deserializer = bincode::Deserializer::from_slice(&encoded, config);
    let context = DeserializationContext {
        format: format.clone(),
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let json_value = context.deserialize(&mut deserializer).unwrap();

    // Serialize back to bincode using json_converter
    let serialize_context = SerializationContext {
        value: &json_value,
        format: &format,
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let config = bincode::DefaultOptions::new();
    let re_encoded: Vec<u8> = config.serialize(&serialize_context).unwrap();

    // Deserialize and verify
    let decoded: Point = config.deserialize(&re_encoded).unwrap();
    assert_eq!(decoded, point);
}

#[test]
fn test_bincode_roundtrip_enum() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Message {
        Quit,
        Move { x: i32, y: i32 },
        Write(String),
    }

    // Use tracer to extract the format
    let mut tracer = serde_reflection::Tracer::new(serde_reflection::TracerConfig::default());
    tracer.trace_simple_type::<Message>().unwrap();
    let registry = tracer.registry().unwrap();
    let format = Format::TypeName("Message".to_string());

    // Test Struct variant
    let msg = Message::Move { x: 5, y: 10 };
    let config = bincode::DefaultOptions::new();
    let encoded = config.serialize(&msg).unwrap();

    // Deserialize to JSON
    let mut deserializer = bincode::Deserializer::from_slice(&encoded, config);
    let context = DeserializationContext {
        format: format.clone(),
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let json_value = context.deserialize(&mut deserializer).unwrap();

    // Serialize back to bincode
    let serialize_context = SerializationContext {
        value: &json_value,
        format: &format,
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let config = bincode::DefaultOptions::new();
    let re_encoded: Vec<u8> = config.serialize(&serialize_context).unwrap();

    // Deserialize and verify
    let decoded: Message = config.deserialize(&re_encoded).unwrap();
    assert_eq!(decoded, msg);
}

// ============================================================================
// BCS Round-trip Tests
// ============================================================================

#[test]
fn test_bcs_roundtrip_struct() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }

    // Use tracer to extract the format
    let mut tracer = serde_reflection::Tracer::new(serde_reflection::TracerConfig::default());
    let (format, _) = tracer
        .trace_type::<Point>(&serde_reflection::Samples::new())
        .unwrap();
    let registry = tracer.registry().unwrap();

    // Serialize with BCS
    let point = Point { x: 10, y: 20 };
    let encoded = bcs::to_bytes(&point).unwrap();

    // Deserialize to JSON using json_converter
    let json_value: Value = bcs::from_bytes_seed(
        DeserializationContext {
            format: format.clone(),
            registry: &registry,
            environment: &EmptyEnvironment,
        },
        &encoded,
    )
    .unwrap();

    // Serialize back to BCS using json_converter
    let serialize_context = SerializationContext {
        value: &json_value,
        format: &format,
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let re_encoded = bcs::to_bytes(&serialize_context).unwrap();

    // Deserialize and verify
    let decoded: Point = bcs::from_bytes(&re_encoded).unwrap();
    assert_eq!(decoded, point);
}

#[test]
fn test_bcs_roundtrip_enum() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    enum Message {
        Quit,
        Move { x: i32, y: i32 },
        Write(String),
    }

    // Use tracer to extract the format
    let mut tracer = serde_reflection::Tracer::new(serde_reflection::TracerConfig::default());
    tracer.trace_simple_type::<Message>().unwrap();
    let registry = tracer.registry().unwrap();
    let format = Format::TypeName("Message".to_string());

    // Test Struct variant
    let msg = Message::Move { x: 5, y: 10 };
    let encoded = bcs::to_bytes(&msg).unwrap();

    // Deserialize to JSON
    let json_value: Value = bcs::from_bytes_seed(
        DeserializationContext {
            format: format.clone(),
            registry: &registry,
            environment: &EmptyEnvironment,
        },
        &encoded,
    )
    .unwrap();

    // Serialize back to BCS
    let serialize_context = SerializationContext {
        value: &json_value,
        format: &format,
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let re_encoded = bcs::to_bytes(&serialize_context).unwrap();

    // Deserialize and verify
    let decoded: Message = bcs::from_bytes(&re_encoded).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn test_bcs_roundtrip_complex_types() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct User {
        id: u64,
        name: String,
        tags: Vec<String>,
    }

    // Use tracer to extract the format
    let mut tracer = serde_reflection::Tracer::new(serde_reflection::TracerConfig::default());
    let (format, _) = tracer
        .trace_type::<User>(&serde_reflection::Samples::new())
        .unwrap();
    let registry = tracer.registry().unwrap();

    // Test with complex data
    let user = User {
        id: 12345,
        name: "Alice".to_string(),
        tags: vec!["admin".to_string(), "verified".to_string()],
    };
    let encoded = bcs::to_bytes(&user).unwrap();

    // Deserialize to JSON
    let json_value: Value = bcs::from_bytes_seed(
        DeserializationContext {
            format: format.clone(),
            registry: &registry,
            environment: &EmptyEnvironment,
        },
        &encoded,
    )
    .unwrap();

    // Verify JSON structure
    assert_eq!(json_value["id"], json!(12345));
    assert_eq!(json_value["name"], json!("Alice"));
    assert_eq!(json_value["tags"], json!(["admin", "verified"]));

    // Serialize back to BCS
    let serialize_context = SerializationContext {
        value: &json_value,
        format: &format,
        registry: &registry,
        environment: &EmptyEnvironment,
    };
    let re_encoded = bcs::to_bytes(&serialize_context).unwrap();

    // Deserialize and verify
    let decoded: User = bcs::from_bytes(&re_encoded).unwrap();
    assert_eq!(decoded, user);
}

// ============================================================================
// Error Cases Tests for Serialization
// ============================================================================

#[test]
fn test_serialize_error_wrong_type() {
    let registry = Registry::new();

    // Try to serialize a string as a bool
    let result = serialize_json(&json!("not a bool"), &Format::Bool, &registry);
    assert!(result.is_err());
}

#[test]
fn test_serialize_error_tuple_size_mismatch() {
    let registry = Registry::new();
    let format = Format::Tuple(vec![Format::I32, Format::Str]);

    // Wrong number of elements
    let result = serialize_json(&json!([42]), &format, &registry);
    assert!(result.is_err());
}

#[test]
fn test_serialize_error_missing_field() {
    let mut registry = Registry::new();
    registry.insert(
        "Person".to_string(),
        ContainerFormat::Struct(vec![
            Named {
                name: "name".to_string(),
                value: Format::Str,
            },
            Named {
                name: "age".to_string(),
                value: Format::U32,
            },
        ]),
    );

    let format = Format::TypeName("Person".to_string());
    // Missing "age" field
    let result = serialize_json(&json!({"name": "Alice"}), &format, &registry);
    assert!(result.is_err());
}

#[test]
fn test_serialize_error_unknown_variant() {
    let mut registry = Registry::new();
    let mut variants = BTreeMap::new();
    variants.insert(
        0,
        Named {
            name: "KnownVariant".to_string(),
            value: VariantFormat::Unit,
        },
    );
    registry.insert("E".to_string(), ContainerFormat::Enum(variants));

    let format = Format::TypeName("E".to_string());
    let result = serialize_json(&json!({"UnknownVariant": null}), &format, &registry);
    assert!(result.is_err());
}

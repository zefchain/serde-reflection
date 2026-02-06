// Copyright (c) Zefchain Labs, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dynamic conversion between binary-serialized data and JSON values.
//!
//! This module provides dynamic conversion between binary-serialized data and JSON values.
//! This is useful when you need to inspect or manipulate serialized data without having
//! access to the original Rust types at compile time.
//!
//! # Example
//!
//! ```rust
//! use bincode::Options;
//! use serde::{Serialize, Deserialize};
//! use serde_reflection::{Tracer, TracerConfig, Samples};
//! use serde_reflection::json_converter::{DeserializationContext, SerializationContext, EmptyEnvironment};
//! use serde_json::json;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Point {
//!     x: i32,
//!     y: i32,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Use tracer to extract the format
//! let mut tracer = Tracer::new(TracerConfig::default());
//! let (format, _) = tracer
//!     .trace_type::<Point>(&Samples::new())?;
//! let registry = tracer.registry()?;
//!
//! // Serialize with bincode
//! let config = bincode::DefaultOptions::new();
//! let point = Point { x: 10, y: 20 };
//! let encoded = config.serialize(&point)?;
//!
//! // Deserialize to JSON using DeserializationContext
//! let mut deserializer = bincode::Deserializer::from_slice(&encoded, config);
//! let context = DeserializationContext {
//!     format: format.clone(),
//!     registry: &registry,
//!     environment: &EmptyEnvironment,
//! };
//! let value: serde_json::Value = serde::de::DeserializeSeed::deserialize(context, &mut deserializer)?;
//! assert_eq!(value["x"], json!(10));
//! assert_eq!(value["y"], json!(20));
//!
//! // Serialize JSON back to binary using SerializationContext
//! let context = SerializationContext {
//!     value: &value,
//!     format: &format,
//!     registry: &registry,
//!     environment: &EmptyEnvironment,
//! };
//! let bytes = config.serialize(&context)?;
//! assert_eq!(encoded, bytes);
//! # Ok(())
//! # }
//! ```
//!
//! This approach is particularly useful for cryptographic applications where you need to
//! compute hashes of JSON values using a binary format like [BCS](https://github.com/diem/bcs).

use crate::{ContainerFormat, Format, Named, Registry, VariantFormat};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

/// A deserialization context to create a JSON value from a serialized object in a dynamic
/// format.
pub struct DeserializationContext<'a, E> {
    /// The format of the main value.
    pub format: Format,
    /// The registry of container formats.
    pub registry: &'a Registry,
    /// The environment containing external parsers.
    pub environment: &'a E,
}

use once_cell::sync::Lazy;
use std::{collections::HashSet, sync::Mutex};

static GLOBAL_STRING_SET: Lazy<Mutex<HashSet<&'static str>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

static GLOBAL_FIELDS_SET: Lazy<Mutex<HashSet<&'static [&'static str]>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

/// The requirement for an `environment` that manages a symbol table.
pub trait SymbolTableEnvironment {
    fn get_static_name(&self, name: &str) -> &'static str {
        let mut set = GLOBAL_STRING_SET.lock().unwrap();
        // TODO: use https://github.com/rust-lang/rust/issues/60896 when available
        if let Some(value) = set.get(name) {
            value
        } else {
            set.insert(name.to_string().leak());
            set.get(name).unwrap()
        }
    }

    fn get_static_fields<'a>(
        &self,
        fields: impl IntoIterator<Item = &'a str>,
    ) -> &'static [&'static str] {
        let fields = fields
            .into_iter()
            .map(|name| self.get_static_name(name))
            .collect::<Vec<_>>();
        let mut set = GLOBAL_FIELDS_SET.lock().unwrap();
        // TODO: use https://github.com/rust-lang/rust/issues/60896 when available
        if let Some(value) = set.get(fields.as_slice()) {
            value
        } else {
            set.insert(fields.to_vec().leak());
            set.get(fields.as_slice()).unwrap()
        }
    }
}

/// The requirement for the `environment` objects to help with Deserialize.
pub trait DeserializationEnvironment<'de>: SymbolTableEnvironment {
    /// Deserialize a value of an external type `name`.
    fn deserialize<D>(&self, name: String, deserializer: D) -> Result<Value, String>
    where
        D: Deserializer<'de>;
}

pub struct EmptyEnvironment;

impl SymbolTableEnvironment for EmptyEnvironment {}

impl<'de> DeserializationEnvironment<'de> for EmptyEnvironment {
    fn deserialize<D>(&self, name: String, _deserializer: D) -> Result<Value, String>
    where
        D: Deserializer<'de>,
    {
        Err(format!("No external definition available for {name}"))
    }
}

impl<'a, 'de, E> DeserializeSeed<'de> for DeserializationContext<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        use Format::*;

        match self.format {
            Variable(_) => Err(<D::Error as serde::de::Error>::custom(
                "Required formats cannot contain variables",
            )),
            TypeName(name) => {
                if let Some(container_format) = self.registry.get(&name) {
                    // Process the container format by deserializing according to its structure
                    deserialize_container_format(
                        &name,
                        container_format,
                        self.registry,
                        self.environment,
                        deserializer,
                    )
                } else {
                    Ok(self
                        .environment
                        .deserialize(name, deserializer)
                        .map_err(<D::Error as serde::de::Error>::custom)?)
                }
            }
            Unit => Ok(Value::Null),
            Bool => {
                let value = bool::deserialize(deserializer)?;
                Ok(Value::Bool(value))
            }
            I8 => {
                let value = i8::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            I16 => {
                let value = i16::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            I32 => {
                let value = i32::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            I64 => {
                let value = i64::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            I128 => {
                let value = i128::deserialize(deserializer)?;
                // i128 is too large for JSON Number, so we convert to i64 if possible
                // or use a string representation
                if let Ok(small_value) = i64::try_from(value) {
                    Ok(Value::Number(Number::from(small_value)))
                } else {
                    Ok(Value::String(value.to_string()))
                }
            }
            U8 => {
                let value = u8::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            U16 => {
                let value = u16::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            U32 => {
                let value = u32::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            U64 => {
                let value = u64::deserialize(deserializer)?;
                Ok(Value::Number(Number::from(value)))
            }
            U128 => {
                let value = u128::deserialize(deserializer)?;
                // u128 is too large for JSON Number, so we convert to u64 if possible
                // or use a string representation
                if let Ok(small_value) = u64::try_from(value) {
                    Ok(Value::Number(Number::from(small_value)))
                } else {
                    Ok(Value::String(value.to_string()))
                }
            }
            F32 => {
                let value = f32::deserialize(deserializer)?;
                Number::from_f64(value as f64)
                    .map(Value::Number)
                    .ok_or_else(|| <D::Error as serde::de::Error>::custom("Invalid f32 value"))
            }
            F64 => {
                let value = f64::deserialize(deserializer)?;
                Number::from_f64(value)
                    .map(Value::Number)
                    .ok_or_else(|| <D::Error as serde::de::Error>::custom("Invalid f64 value"))
            }
            Char => {
                let value = char::deserialize(deserializer)?;
                Ok(Value::String(value.to_string()))
            }
            Str => {
                let value = String::deserialize(deserializer)?;
                Ok(Value::String(value))
            }
            Bytes => {
                let value = Vec::<u8>::deserialize(deserializer)?;
                Ok(Value::Array(
                    value
                        .into_iter()
                        .map(|b| Value::Number(Number::from(b)))
                        .collect(),
                ))
            }
            Option(format) => {
                let visitor = OptionVisitor {
                    format: *format,
                    registry: self.registry,
                    environment: self.environment,
                };
                deserializer.deserialize_option(visitor)
            }
            Seq(format) => {
                let visitor = SeqVisitor {
                    format: *format,
                    registry: self.registry,
                    environment: self.environment,
                };
                deserializer.deserialize_seq(visitor)
            }
            Map { key, value } => {
                let visitor = MapVisitor {
                    key_format: *key,
                    value_format: *value,
                    registry: self.registry,
                    environment: self.environment,
                };
                deserializer.deserialize_map(visitor)
            }
            Tuple(formats) => {
                let visitor = TupleVisitor {
                    formats,
                    registry: self.registry,
                    environment: self.environment,
                };
                deserializer.deserialize_tuple(visitor.formats.len(), visitor)
            }
            TupleArray { content, size } => {
                let visitor = TupleArrayVisitor {
                    format: *content,
                    size,
                    registry: self.registry,
                    environment: self.environment,
                };
                deserializer.deserialize_tuple(visitor.size, visitor)
            }
        }
    }
}

struct OptionVisitor<'a, E> {
    format: Format,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for OptionVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an optional value")
    }

    fn visit_none<Err>(self) -> Result<Self::Value, Err>
    where
        Err: serde::de::Error,
    {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let context = DeserializationContext {
            format: self.format,
            registry: self.registry,
            environment: self.environment,
        };
        context.deserialize(deserializer)
    }

    fn visit_unit<Err>(self) -> Result<Self::Value, Err>
    where
        Err: serde::de::Error,
    {
        Ok(Value::Null)
    }
}

struct SeqVisitor<'a, E> {
    format: Format,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for SeqVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(DeserializationContext {
            format: self.format.clone(),
            registry: self.registry,
            environment: self.environment,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
}

struct MapVisitor<'a, E> {
    key_format: Format,
    value_format: Format,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for MapVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        while let Some((key, value)) = map.next_entry_seed(
            DeserializationContext {
                format: self.key_format.clone(),
                registry: self.registry,
                environment: self.environment,
            },
            DeserializationContext {
                format: self.value_format.clone(),
                registry: self.registry,
                environment: self.environment,
            },
        )? {
            // Convert the key Value to a String
            let key_string = match key {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => {
                    return Err(serde::de::Error::custom(
                        "Map keys must be strings, numbers, or booleans",
                    ))
                }
            };
            object.insert(key_string, value);
        }
        Ok(Value::Object(object))
    }
}

struct TupleVisitor<'a, E> {
    formats: Vec<Format>,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for TupleVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        for format in self.formats {
            match seq.next_element_seed(DeserializationContext {
                format,
                registry: self.registry,
                environment: self.environment,
            })? {
                Some(value) => values.push(value),
                None => {
                    return Err(serde::de::Error::custom(
                        "Tuple has fewer elements than expected",
                    ))
                }
            }
        }
        Ok(Value::Array(values))
    }
}

struct TupleArrayVisitor<'a, E> {
    format: Format,
    size: usize,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for TupleArrayVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a tuple array")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        for _ in 0..self.size {
            match seq.next_element_seed(DeserializationContext {
                format: self.format.clone(),
                registry: self.registry,
                environment: self.environment,
            })? {
                Some(value) => values.push(value),
                None => {
                    return Err(serde::de::Error::custom(
                        "Tuple array has fewer elements than expected",
                    ))
                }
            }
        }
        Ok(Value::Array(values))
    }
}

// Helper function to deserialize a container format
fn deserialize_container_format<'a, 'de, E, D>(
    name: &str,
    container_format: &'a ContainerFormat,
    registry: &'a Registry,
    environment: &'a E,
    deserializer: D,
) -> Result<Value, D::Error>
where
    E: DeserializationEnvironment<'de>,
    D: Deserializer<'de>,
{
    use ContainerFormat::*;

    match container_format {
        UnitStruct => {
            // Unit structs deserialize as null
            deserializer.deserialize_unit(UnitStructVisitor)
        }
        NewTypeStruct(format) => {
            // NewType structs unwrap to their inner value
            let name = environment.get_static_name(name);
            let visitor = NewTypeStructVisitor {
                format: (**format).clone(),
                registry,
                environment,
            };
            deserializer.deserialize_newtype_struct(name, visitor)
        }
        TupleStruct(formats) => {
            // Tuple structs deserialize as sequences
            let visitor = TupleStructVisitor {
                formats: formats.clone(),
                registry,
                environment,
            };
            deserializer.deserialize_tuple(formats.len(), visitor)
        }
        Struct(fields) => {
            // Named structs deserialize as maps
            let name = environment.get_static_name(name);
            let static_fields =
                environment.get_static_fields(fields.iter().map(|f| f.name.as_str()));
            let visitor = StructVisitor {
                fields: fields.clone(),
                registry,
                environment,
            };
            deserializer.deserialize_struct(name, static_fields, visitor)
        }
        Enum(variants) => {
            // Enums need special handling
            let name = environment.get_static_name(name);
            let static_fields =
                environment.get_static_fields(variants.iter().map(|(_, v)| v.name.as_str()));
            let visitor = EnumVisitor {
                variants: variants.clone(),
                registry,
                environment,
            };
            deserializer.deserialize_enum(name, static_fields, visitor)
        }
    }
}

struct UnitStructVisitor;

impl<'de> Visitor<'de> for UnitStructVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a unit struct")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Null)
    }
}

struct NewTypeStructVisitor<'a, E> {
    format: Format,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for NewTypeStructVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a newtype struct")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let context = DeserializationContext {
            format: self.format,
            registry: self.registry,
            environment: self.environment,
        };
        context.deserialize(deserializer)
    }
}

struct TupleStructVisitor<'a, E> {
    formats: Vec<Format>,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for TupleStructVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a tuple struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        for format in self.formats {
            match seq.next_element_seed(DeserializationContext {
                format,
                registry: self.registry,
                environment: self.environment,
            })? {
                Some(value) => values.push(value),
                None => {
                    return Err(serde::de::Error::custom(
                        "Tuple struct has fewer elements than expected",
                    ))
                }
            }
        }
        Ok(Value::Array(values))
    }
}

struct StructVisitor<'a, E> {
    fields: Vec<Named<Format>>,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for StructVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a struct")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        for field in self.fields {
            match seq.next_element_seed(DeserializationContext {
                format: field.value,
                registry: self.registry,
                environment: self.environment,
            })? {
                Some(value) => {
                    object.insert(field.name, value);
                }
                None => {
                    return Err(serde::de::Error::custom(
                        "Struct has fewer fields than expected",
                    ))
                }
            }
        }
        Ok(Value::Object(object))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        let fields_map: BTreeMap<_, _> = self
            .fields
            .into_iter()
            .map(|f| (f.name.clone(), f.value))
            .collect();

        while let Some(key) = map.next_key::<String>()? {
            if let Some(format) = fields_map.get(&key) {
                let value = map.next_value_seed(DeserializationContext {
                    format: format.clone(),
                    registry: self.registry,
                    environment: self.environment,
                })?;
                object.insert(key, value);
            } else {
                // Skip unknown fields
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }
        Ok(Value::Object(object))
    }
}

struct EnumVisitor<'a, E> {
    variants: BTreeMap<u32, Named<VariantFormat>>,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for EnumVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an enum")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        // Create a custom deserializer for the variant identifier that can handle
        // both string names (e.g., JSON) and integer indices (e.g., bincode)
        let variant_visitor = VariantIdentifierVisitor {
            variants: &self.variants,
        };

        let (variant_info, variant_data) = data.variant_seed(variant_visitor)?;
        let (variant_name, variant_format) = variant_info;

        let variant_value = deserialize_variant_format(
            &variant_format.value,
            self.registry,
            self.environment,
            variant_data,
        )?;

        // Return a JSON object with the variant name as key
        let mut object = serde_json::Map::new();
        object.insert(variant_name, variant_value);
        Ok(Value::Object(object))
    }
}

struct VariantIdentifierVisitor<'a> {
    variants: &'a BTreeMap<u32, Named<VariantFormat>>,
}

impl<'de> serde::de::DeserializeSeed<'de> for VariantIdentifierVisitor<'_> {
    type Value = (String, Named<VariantFormat>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_identifier(self)
    }
}

impl<'de> Visitor<'de> for VariantIdentifierVisitor<'_> {
    type Value = (String, Named<VariantFormat>);

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Handle integer variant indices (e.g., bincode)
        let variant = self
            .variants
            .get(&(value as u32))
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown variant index: {}", value)))?;
        Ok((variant.name.clone(), variant.clone()))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Handle string variant names (e.g., JSON)
        let variant = self
            .variants
            .values()
            .find(|v| v.name == value)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown variant: {}", value)))?;
        Ok((variant.name.clone(), variant.clone()))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // Handle byte string variant names
        let value_str = std::str::from_utf8(value)
            .map_err(|_| serde::de::Error::custom("Invalid UTF-8 in variant name"))?;
        self.visit_str(value_str)
    }
}

fn deserialize_variant_format<'a, 'de, E, A>(
    variant_format: &VariantFormat,
    registry: &'a Registry,
    environment: &'a E,
    variant_data: A,
) -> Result<Value, A::Error>
where
    E: DeserializationEnvironment<'de>,
    A: serde::de::VariantAccess<'de>,
{
    use VariantFormat::*;

    match variant_format {
        Variable(_) => Err(serde::de::Error::custom(
            "Variant format cannot contain variables",
        )),
        Unit => {
            variant_data.unit_variant()?;
            Ok(Value::Null)
        }
        NewType(format) => {
            let context = DeserializationContext {
                format: (**format).clone(),
                registry,
                environment,
            };
            variant_data.newtype_variant_seed(context)
        }
        Tuple(formats) => {
            let visitor = TupleVariantVisitor {
                formats: formats.clone(),
                registry,
                environment,
            };
            variant_data.tuple_variant(formats.len(), visitor)
        }
        Struct(fields) => {
            let static_fields =
                environment.get_static_fields(fields.iter().map(|v| v.name.as_str()));
            let visitor = StructVariantVisitor {
                fields: fields.clone(),
                registry,
                environment,
            };
            variant_data.struct_variant(static_fields, visitor)
        }
    }
}

struct TupleVariantVisitor<'a, E> {
    formats: Vec<Format>,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for TupleVariantVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a tuple variant")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        for format in self.formats {
            match seq.next_element_seed(DeserializationContext {
                format,
                registry: self.registry,
                environment: self.environment,
            })? {
                Some(value) => values.push(value),
                None => {
                    return Err(serde::de::Error::custom(
                        "Tuple variant has fewer elements than expected",
                    ))
                }
            }
        }
        Ok(Value::Array(values))
    }
}

struct StructVariantVisitor<'a, E> {
    fields: Vec<Named<Format>>,
    registry: &'a Registry,
    environment: &'a E,
}

impl<'a, 'de, E> Visitor<'de> for StructVariantVisitor<'a, E>
where
    E: DeserializationEnvironment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a struct variant")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        for field in self.fields {
            match seq.next_element_seed(DeserializationContext {
                format: field.value,
                registry: self.registry,
                environment: self.environment,
            })? {
                Some(value) => {
                    object.insert(field.name, value);
                }
                None => {
                    return Err(serde::de::Error::custom(
                        "Struct variant has fewer fields than expected",
                    ))
                }
            }
        }
        Ok(Value::Object(object))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        let fields_map: BTreeMap<_, _> = self
            .fields
            .into_iter()
            .map(|f| (f.name.clone(), f.value))
            .collect();

        while let Some(key) = map.next_key::<String>()? {
            if let Some(format) = fields_map.get(&key) {
                let value = map.next_value_seed(DeserializationContext {
                    format: format.clone(),
                    registry: self.registry,
                    environment: self.environment,
                })?;
                object.insert(key, value);
            } else {
                // Skip unknown fields
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }
        Ok(Value::Object(object))
    }
}

/// A serialization context to convert a JSON value to a serialized object in a dynamic format.
pub struct SerializationContext<'a, E> {
    /// The JSON value to serialize.
    pub value: &'a Value,
    /// The format to serialize to.
    pub format: &'a Format,
    /// The registry of container formats.
    pub registry: &'a Registry,
    /// The environment containing external serializers.
    pub environment: &'a E,
}

/// The requirement for the `environment` object for serialization.
pub trait SerializationEnvironment: SymbolTableEnvironment {
    /// Serialize a value of an external type `name`.
    fn serialize<S>(&self, name: &str, value: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl SerializationEnvironment for EmptyEnvironment {
    fn serialize<S>(&self, name: &str, _value: &Value, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom(format!(
            "No external serializer available for {name}"
        )))
    }
}

impl<'a, E> Serialize for SerializationContext<'a, E>
where
    E: SerializationEnvironment,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use Format::*;

        match self.format {
            Variable(_) => Err(serde::ser::Error::custom(
                "Required formats cannot contain variables",
            )),
            TypeName(name) => {
                if let Some(container_format) = self.registry.get(name) {
                    serialize_container_format(
                        name,
                        container_format,
                        self.value,
                        self.registry,
                        self.environment,
                        serializer,
                    )
                } else {
                    self.environment.serialize(name, self.value, serializer)
                }
            }
            Unit => serializer.serialize_unit(),
            Bool => match self.value {
                Value::Bool(b) => serializer.serialize_bool(*b),
                _ => Err(serde::ser::Error::custom("Expected bool value")),
            },
            I8 => serialize_integer::<S, i8>(self.value, serializer),
            I16 => serialize_integer::<S, i16>(self.value, serializer),
            I32 => serialize_integer::<S, i32>(self.value, serializer),
            I64 => serialize_integer::<S, i64>(self.value, serializer),
            I128 => serialize_integer_or_string::<S, i128>(self.value, serializer),
            U8 => serialize_integer::<S, u8>(self.value, serializer),
            U16 => serialize_integer::<S, u16>(self.value, serializer),
            U32 => serialize_integer::<S, u32>(self.value, serializer),
            U64 => serialize_integer::<S, u64>(self.value, serializer),
            U128 => serialize_integer_or_string::<S, u128>(self.value, serializer),
            F32 => serialize_f32(self.value, serializer),
            F64 => serialize_f64(self.value, serializer),
            Char => match self.value {
                Value::String(s) => {
                    let mut chars = s.chars();
                    if let Some(c) = chars.next() {
                        if chars.next().is_none() {
                            serializer.serialize_char(c)
                        } else {
                            Err(serde::ser::Error::custom(
                                "Expected single character string",
                            ))
                        }
                    } else {
                        Err(serde::ser::Error::custom("Expected non-empty string"))
                    }
                }
                _ => Err(serde::ser::Error::custom("Expected string for char")),
            },
            Str => match self.value {
                Value::String(s) => serializer.serialize_str(s),
                _ => Err(serde::ser::Error::custom("Expected string value")),
            },
            Bytes => match self.value {
                Value::Array(arr) => {
                    let bytes: Result<Vec<u8>, _> = arr
                        .iter()
                        .map(|v| match v {
                            Value::Number(n) => n
                                .as_u64()
                                .and_then(|n| u8::try_from(n).ok())
                                .ok_or_else(|| {
                                    serde::ser::Error::custom("Invalid byte value in array")
                                }),
                            _ => Err(serde::ser::Error::custom("Expected number in byte array")),
                        })
                        .collect();
                    serializer.serialize_bytes(&bytes?)
                }
                _ => Err(serde::ser::Error::custom("Expected array for bytes")),
            },
            Option(inner_format) => match self.value {
                Value::Null => serializer.serialize_none(),
                _ => serializer.serialize_some(&SerializationContext {
                    value: self.value,
                    format: inner_format,
                    registry: self.registry,
                    environment: self.environment,
                }),
            },
            Seq(inner_format) => match self.value {
                Value::Array(arr) => {
                    let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                    for item in arr {
                        seq.serialize_element(&SerializationContext {
                            value: item,
                            format: inner_format,
                            registry: self.registry,
                            environment: self.environment,
                        })?;
                    }
                    seq.end()
                }
                _ => Err(serde::ser::Error::custom("Expected array for sequence")),
            },
            Map { key, value } => match self.value {
                Value::Object(obj) => {
                    let mut map = serializer.serialize_map(Some(obj.len()))?;
                    for (k, v) in obj {
                        map.serialize_entry(
                            &SerializationContext {
                                value: &Value::String(k.clone()),
                                format: key,
                                registry: self.registry,
                                environment: self.environment,
                            },
                            &SerializationContext {
                                value: v,
                                format: value,
                                registry: self.registry,
                                environment: self.environment,
                            },
                        )?;
                    }
                    map.end()
                }
                _ => Err(serde::ser::Error::custom("Expected object for map")),
            },
            Tuple(formats) => match self.value {
                Value::Array(arr) => {
                    if arr.len() != formats.len() {
                        return Err(serde::ser::Error::custom(format!(
                            "Expected tuple of length {}, got {}",
                            formats.len(),
                            arr.len()
                        )));
                    }
                    let mut tuple = serializer.serialize_tuple(formats.len())?;
                    for (item, format) in arr.iter().zip(formats.iter()) {
                        tuple.serialize_element(&SerializationContext {
                            value: item,
                            format,
                            registry: self.registry,
                            environment: self.environment,
                        })?;
                    }
                    tuple.end()
                }
                _ => Err(serde::ser::Error::custom("Expected array for tuple")),
            },
            TupleArray { content, size } => match self.value {
                Value::Array(arr) => {
                    if arr.len() != *size {
                        return Err(serde::ser::Error::custom(format!(
                            "Expected array of length {}, got {}",
                            size,
                            arr.len()
                        )));
                    }
                    let mut tuple = serializer.serialize_tuple(*size)?;
                    for item in arr {
                        tuple.serialize_element(&SerializationContext {
                            value: item,
                            format: content,
                            registry: self.registry,
                            environment: self.environment,
                        })?;
                    }
                    tuple.end()
                }
                _ => Err(serde::ser::Error::custom("Expected array for tuple array")),
            },
        }
    }
}

fn serialize_integer<S, I>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    I: TryFrom<i64> + TryFrom<u64> + Serialize,
{
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let converted = I::try_from(i)
                    .map_err(|_| serde::ser::Error::custom("Integer out of range"))?;
                converted.serialize(serializer)
            } else if let Some(u) = n.as_u64() {
                let converted = I::try_from(u)
                    .map_err(|_| serde::ser::Error::custom("Integer out of range"))?;
                converted.serialize(serializer)
            } else {
                Err(serde::ser::Error::custom("Invalid number"))
            }
        }
        _ => Err(serde::ser::Error::custom("Expected number")),
    }
}

fn serialize_integer_or_string<S, I>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    I: TryFrom<i64> + TryFrom<u64> + std::str::FromStr + Serialize,
{
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let converted = I::try_from(i)
                    .map_err(|_| serde::ser::Error::custom("Integer out of range"))?;
                converted.serialize(serializer)
            } else if let Some(u) = n.as_u64() {
                let converted = I::try_from(u)
                    .map_err(|_| serde::ser::Error::custom("Integer out of range"))?;
                converted.serialize(serializer)
            } else {
                Err(serde::ser::Error::custom("Invalid number"))
            }
        }
        Value::String(s) => {
            let converted = s
                .parse::<I>()
                .map_err(|_| serde::ser::Error::custom("Invalid integer string"))?;
            converted.serialize(serializer)
        }
        _ => Err(serde::ser::Error::custom("Expected number or string")),
    }
}

fn serialize_f32<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                serializer.serialize_f32(f as f32)
            } else {
                Err(serde::ser::Error::custom("Invalid float"))
            }
        }
        _ => Err(serde::ser::Error::custom("Expected number for float")),
    }
}

fn serialize_f64<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                serializer.serialize_f64(f)
            } else {
                Err(serde::ser::Error::custom("Invalid float"))
            }
        }
        _ => Err(serde::ser::Error::custom("Expected number for float")),
    }
}

fn serialize_container_format<S, E>(
    name: &str,
    container_format: &ContainerFormat,
    value: &Value,
    registry: &Registry,
    environment: &E,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    E: SerializationEnvironment,
{
    use ContainerFormat::*;

    let static_name = environment.get_static_name(name);

    match container_format {
        UnitStruct => serializer.serialize_unit_struct(static_name),
        NewTypeStruct(format) => {
            let context = SerializationContext {
                value,
                format,
                registry,
                environment,
            };
            serializer.serialize_newtype_struct(static_name, &context)
        }
        TupleStruct(formats) => match value {
            Value::Array(arr) => {
                if arr.len() != formats.len() {
                    return Err(serde::ser::Error::custom(format!(
                        "Expected tuple struct of length {}, got {}",
                        formats.len(),
                        arr.len()
                    )));
                }
                let mut tuple_struct =
                    serializer.serialize_tuple_struct(static_name, formats.len())?;
                for (item, format) in arr.iter().zip(formats.iter()) {
                    tuple_struct.serialize_field(&SerializationContext {
                        value: item,
                        format,
                        registry,
                        environment,
                    })?;
                }
                tuple_struct.end()
            }
            _ => Err(serde::ser::Error::custom("Expected array for tuple struct")),
        },
        Struct(fields) => match value {
            Value::Object(obj) => {
                let mut struct_ser = serializer.serialize_struct(static_name, fields.len())?;
                for field in fields {
                    let field_value = obj.get(&field.name).ok_or_else(|| {
                        serde::ser::Error::custom(format!("Missing field: {}", field.name))
                    })?;
                    let static_field_name = environment.get_static_name(&field.name);
                    struct_ser.serialize_field(
                        static_field_name,
                        &SerializationContext {
                            value: field_value,
                            format: &field.value,
                            registry,
                            environment,
                        },
                    )?;
                }
                struct_ser.end()
            }
            _ => Err(serde::ser::Error::custom("Expected object for struct")),
        },
        Enum(variants) => match value {
            Value::Object(obj) => {
                if obj.len() != 1 {
                    return Err(serde::ser::Error::custom(
                        "Expected object with single variant key",
                    ));
                }
                let (variant_name, variant_value) = obj.iter().next().unwrap();

                // Find the variant by name
                let (variant_index, variant_format) = variants
                    .iter()
                    .find(|(_, v)| v.name == *variant_name)
                    .ok_or_else(|| {
                        serde::ser::Error::custom(format!("Unknown variant: {}", variant_name))
                    })?;

                serialize_enum_variant(
                    static_name,
                    *variant_index,
                    variant_name,
                    &variant_format.value,
                    variant_value,
                    registry,
                    environment,
                    serializer,
                )
            }
            _ => Err(serde::ser::Error::custom("Expected object for enum")),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn serialize_enum_variant<S, E>(
    enum_name: &'static str,
    variant_index: u32,
    variant_name: &str,
    variant_format: &VariantFormat,
    value: &Value,
    registry: &Registry,
    environment: &E,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    E: SerializationEnvironment,
{
    use VariantFormat::*;

    let static_variant_name = environment.get_static_name(variant_name);

    match variant_format {
        Variable(_) => Err(serde::ser::Error::custom(
            "Variant format cannot contain variables",
        )),
        Unit => match value {
            Value::Null => {
                serializer.serialize_unit_variant(enum_name, variant_index, static_variant_name)
            }
            _ => Err(serde::ser::Error::custom("Expected null for unit variant")),
        },
        NewType(format) => {
            let context = SerializationContext {
                value,
                format,
                registry,
                environment,
            };
            serializer.serialize_newtype_variant(
                enum_name,
                variant_index,
                static_variant_name,
                &context,
            )
        }
        Tuple(formats) => match value {
            Value::Array(arr) => {
                if arr.len() != formats.len() {
                    return Err(serde::ser::Error::custom(format!(
                        "Expected tuple variant of length {}, got {}",
                        formats.len(),
                        arr.len()
                    )));
                }
                let mut tuple_variant = serializer.serialize_tuple_variant(
                    enum_name,
                    variant_index,
                    static_variant_name,
                    formats.len(),
                )?;
                for (item, format) in arr.iter().zip(formats.iter()) {
                    tuple_variant.serialize_field(&SerializationContext {
                        value: item,
                        format,
                        registry,
                        environment,
                    })?;
                }
                tuple_variant.end()
            }
            _ => Err(serde::ser::Error::custom(
                "Expected array for tuple variant",
            )),
        },
        Struct(fields) => match value {
            Value::Object(obj) => {
                let mut struct_variant = serializer.serialize_struct_variant(
                    enum_name,
                    variant_index,
                    static_variant_name,
                    fields.len(),
                )?;
                for field in fields {
                    let field_value = obj.get(&field.name).ok_or_else(|| {
                        serde::ser::Error::custom(format!("Missing field: {}", field.name))
                    })?;
                    let static_field_name = environment.get_static_name(&field.name);
                    struct_variant.serialize_field(
                        static_field_name,
                        &SerializationContext {
                            value: field_value,
                            format: &field.value,
                            registry,
                            environment,
                        },
                    )?;
                }
                struct_variant.end()
            }
            _ => Err(serde::ser::Error::custom(
                "Expected object for struct variant",
            )),
        },
    }
}

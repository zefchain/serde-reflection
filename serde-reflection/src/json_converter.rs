// Copyright (c) Zefchain Labs, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dynamic conversion to JSON values

use crate::{ContainerFormat, Format, Named, Registry, VariantFormat};
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

/// A deserialization context to create a JSON value from a serialized object in a dynamic
/// format.
pub struct Context<'a, E> {
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

/// The requirement for the `environment` object.
pub trait Environment<'de> {
    /// Deserialize a value of an external type `name`.
    fn deserialize<D>(&self, name: String, deserializer: D) -> Result<Value, String>
    where
        D: Deserializer<'de>;

    fn leak_name(&self, name: &str) -> &'static str {
        let mut set = GLOBAL_STRING_SET.lock().unwrap();
        // TODO: use https://github.com/rust-lang/rust/issues/60896 when available
        if let Some(value) = set.get(name) {
            value
        } else {
            set.insert(name.to_string().leak());
            set.get(name).unwrap()
        }
    }

    fn leak_fields(&self, fields: &[&str]) -> &'static [&'static str] {
        let fields = fields
            .iter()
            .map(|name| self.leak_name(name))
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

pub struct EmptyEnvironment;

impl<'de> Environment<'de> for EmptyEnvironment {
    fn deserialize<D>(&self, name: String, _deserializer: D) -> Result<Value, String>
    where
        D: Deserializer<'de>,
    {
        Err(format!("No external definition available for {name}"))
    }
}

impl<'a, 'de, E> DeserializeSeed<'de> for Context<'a, E>
where
    E: Environment<'de>,
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
    E: Environment<'de>,
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
        let context = Context {
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
    E: Environment<'de>,
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
        while let Some(value) = seq.next_element_seed(Context {
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
    E: Environment<'de>,
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
            Context {
                format: self.key_format.clone(),
                registry: self.registry,
                environment: self.environment,
            },
            Context {
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
    E: Environment<'de>,
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
            match seq.next_element_seed(Context {
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
    E: Environment<'de>,
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
            match seq.next_element_seed(Context {
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
    E: Environment<'de>,
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
            let name = environment.leak_name(name);
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
            let name = environment.leak_name(name);
            let static_fields = environment.leak_fields(
                fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            let visitor = StructVisitor {
                fields: fields.clone(),
                registry,
                environment,
            };
            deserializer.deserialize_struct(name, static_fields, visitor)
        }
        Enum(variants) => {
            // Enums need special handling
            let name = environment.leak_name(name);
            let static_fields = environment.leak_fields(
                variants
                    .iter()
                    .map(|(_, v)| v.name.as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
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
    E: Environment<'de>,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a newtype struct")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let context = Context {
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
    E: Environment<'de>,
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
            match seq.next_element_seed(Context {
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
    E: Environment<'de>,
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
            match seq.next_element_seed(Context {
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
                let value = map.next_value_seed(Context {
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
    E: Environment<'de>,
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
    E: Environment<'de>,
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
            let context = Context {
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
            let static_fields = environment.leak_fields(
                fields
                    .iter()
                    .map(|v| v.name.as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
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
    E: Environment<'de>,
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
            match seq.next_element_seed(Context {
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
    E: Environment<'de>,
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
            match seq.next_element_seed(Context {
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
                let value = map.next_value_seed(Context {
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

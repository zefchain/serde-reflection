// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    de::Deserializer,
    error::{Error, Result},
    format::*,
    ser::Serializer,
    value::Value,
};
use erased_discriminant::Discriminant;
use once_cell::sync::Lazy;
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use std::any::TypeId;
use std::collections::BTreeMap;

/// A map of container formats.
pub type Registry = BTreeMap<String, ContainerFormat>;

/// Structure to drive the tracing of Serde serialization and deserialization.
/// This typically aims at computing a `Registry`.
#[derive(Debug)]
pub struct Tracer {
    /// Hold configuration options.
    pub(crate) config: TracerConfig,

    /// Formats of the named containers discovered so far, while tracing
    /// serialization and/or deserialization.
    pub(crate) registry: Registry,

    /// Enums that have detected to be yet incomplete (i.e. missing variants)
    /// while tracing deserialization.
    pub(crate) incomplete_enums: BTreeMap<String, EnumProgress>,

    /// Discriminant associated with each variant of each enum.
    pub(crate) discriminants: BTreeMap<(TypeId, VariantId<'static>), Discriminant>,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum EnumProgress {
    /// There are variant names that have not yet been traced.
    NamedVariantsRemaining,
    /// There are variant numbers that have not yet been traced.
    IndexedVariantsRemaining,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(crate) enum VariantId<'a> {
    Index(u32),
    Name(&'a str),
}

/// User inputs, aka "samples", recorded during serialization.
/// This will help passing user-defined checks during deserialization.
#[derive(Debug, Default)]
pub struct Samples {
    pub(crate) values: BTreeMap<&'static str, Value>,
}

impl Samples {
    /// Create a new structure to hold value samples.
    pub fn new() -> Self {
        Self::default()
    }

    /// Obtain a (serialized) sample.
    pub fn value(&self, name: &'static str) -> Option<&Value> {
        self.values.get(name)
    }
}

/// Configuration object to create a tracer.
#[derive(Debug)]
pub struct TracerConfig {
    pub(crate) is_human_readable: bool,
    pub(crate) record_samples_for_newtype_structs: bool,
    pub(crate) record_samples_for_tuple_structs: bool,
    pub(crate) record_samples_for_structs: bool,
    pub(crate) default_bool_value: bool,
    pub(crate) default_u8_value: u8,
    pub(crate) default_u16_value: u16,
    pub(crate) default_u32_value: u32,
    pub(crate) default_u64_value: u64,
    pub(crate) default_u128_value: u128,
    pub(crate) default_i8_value: i8,
    pub(crate) default_i16_value: i16,
    pub(crate) default_i32_value: i32,
    pub(crate) default_i64_value: i64,
    pub(crate) default_i128_value: i128,
    pub(crate) default_f32_value: f32,
    pub(crate) default_f64_value: f64,
    pub(crate) default_char_value: char,
    pub(crate) default_borrowed_str_value: &'static str,
    pub(crate) default_string_value: String,
    pub(crate) default_borrowed_bytes_value: &'static [u8],
    pub(crate) default_byte_buf_value: Vec<u8>,
}

impl Default for TracerConfig {
    /// Create a new structure to hold value samples.
    fn default() -> Self {
        Self {
            is_human_readable: false,
            record_samples_for_newtype_structs: true,
            record_samples_for_tuple_structs: false,
            record_samples_for_structs: false,
            default_bool_value: false,
            default_u8_value: 0,
            default_u16_value: 0,
            default_u32_value: 0,
            default_u64_value: 0,
            default_u128_value: 0,
            default_i8_value: 0,
            default_i16_value: 0,
            default_i32_value: 0,
            default_i64_value: 0,
            default_i128_value: 0,
            default_f32_value: 0.0,
            default_f64_value: 0.0,
            default_char_value: 'A',
            default_borrowed_str_value: "",
            default_string_value: String::new(),
            default_borrowed_bytes_value: b"",
            default_byte_buf_value: Vec::new(),
        }
    }
}

macro_rules! define_default_value_setter {
    ($method:ident, $ty:ty) => {
        /// The default serialized value for this primitive type.
        pub fn $method(mut self, value: $ty) -> Self {
            self.$method = value;
            self
        }
    };
}

impl TracerConfig {
    /// Whether to trace the human readable encoding of (de)serialization.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_human_readable(mut self, value: bool) -> Self {
        self.is_human_readable = value;
        self
    }

    /// Record samples of newtype structs during serialization and inject them during deserialization.
    pub fn record_samples_for_newtype_structs(mut self, value: bool) -> Self {
        self.record_samples_for_newtype_structs = value;
        self
    }

    /// Record samples of tuple structs during serialization and inject them during deserialization.
    pub fn record_samples_for_tuple_structs(mut self, value: bool) -> Self {
        self.record_samples_for_tuple_structs = value;
        self
    }

    /// Record samples of (regular) structs during serialization and inject them during deserialization.
    pub fn record_samples_for_structs(mut self, value: bool) -> Self {
        self.record_samples_for_structs = value;
        self
    }

    define_default_value_setter!(default_bool_value, bool);
    define_default_value_setter!(default_u8_value, u8);
    define_default_value_setter!(default_u16_value, u16);
    define_default_value_setter!(default_u32_value, u32);
    define_default_value_setter!(default_u64_value, u64);
    define_default_value_setter!(default_u128_value, u128);
    define_default_value_setter!(default_i8_value, i8);
    define_default_value_setter!(default_i16_value, i16);
    define_default_value_setter!(default_i32_value, i32);
    define_default_value_setter!(default_i64_value, i64);
    define_default_value_setter!(default_i128_value, i128);
    define_default_value_setter!(default_f32_value, f32);
    define_default_value_setter!(default_f64_value, f64);
    define_default_value_setter!(default_char_value, char);
    define_default_value_setter!(default_borrowed_str_value, &'static str);
    define_default_value_setter!(default_string_value, String);
    define_default_value_setter!(default_borrowed_bytes_value, &'static [u8]);
    define_default_value_setter!(default_byte_buf_value, Vec<u8>);
}

impl Tracer {
    /// Start tracing deserialization.
    pub fn new(config: TracerConfig) -> Self {
        Self {
            config,
            registry: BTreeMap::new(),
            incomplete_enums: BTreeMap::new(),
            discriminants: BTreeMap::new(),
        }
    }

    /// Trace the serialization of a particular value.
    /// * Nested containers will be added to the tracing registry, indexed by
    ///   their (non-qualified) name.
    /// * Sampled Rust values will be inserted into `samples` to benefit future calls
    ///   to the `trace_type_*` methods.
    pub fn trace_value<T>(&mut self, samples: &mut Samples, value: &T) -> Result<(Format, Value)>
    where
        T: ?Sized + Serialize,
    {
        let serializer = Serializer::new(self, samples);
        let (mut format, sample) = value.serialize(serializer)?;
        format.reduce();
        Ok((format, sample))
    }

    /// Trace a single deserialization of a particular type.
    /// * Nested containers will be added to the tracing registry, indexed by
    ///   their (non-qualified) name.
    /// * As a byproduct of deserialization, we also return a value of type `T`.
    /// * Tracing deserialization of a type may fail if this type or some dependencies
    ///   have implemented a custom deserializer that validates data. The solution is
    ///   to make sure that `samples` holds enough sampled Rust values to cover all the
    ///   custom types.
    pub fn trace_type_once<'de, T>(&mut self, samples: &'de Samples) -> Result<(Format, T)>
    where
        T: Deserialize<'de>,
    {
        let mut format = Format::unknown();
        let deserializer = Deserializer::new(self, samples, &mut format);
        let value = T::deserialize(deserializer)?;
        format.reduce();
        Ok((format, value))
    }

    /// Same as `trace_type_once` for seeded deserialization.
    pub fn trace_type_once_with_seed<'de, S>(
        &mut self,
        samples: &'de Samples,
        seed: S,
    ) -> Result<(Format, S::Value)>
    where
        S: DeserializeSeed<'de>,
    {
        let mut format = Format::unknown();
        let deserializer = Deserializer::new(self, samples, &mut format);
        let value = seed.deserialize(deserializer)?;
        format.reduce();
        Ok((format, value))
    }

    /// Same as `trace_type_once` but if `T` is an enum, we repeat the process
    /// until all variants of `T` are covered.
    /// We accumulate and return all the sampled values at the end.
    pub fn trace_type<'de, T>(&mut self, samples: &'de Samples) -> Result<(Format, Vec<T>)>
    where
        T: Deserialize<'de>,
    {
        let mut values = Vec::new();
        loop {
            let (format, value) = self.trace_type_once::<T>(samples)?;
            values.push(value);
            if let Format::TypeName(name) = &format {
                if let Some(&progress) = self.incomplete_enums.get(name) {
                    // Restart the analysis to find more variants of T.
                    self.incomplete_enums.remove(name);
                    if let EnumProgress::NamedVariantsRemaining = progress {
                        values.pop().unwrap();
                    }
                    continue;
                }
            }
            return Ok((format, values));
        }
    }

    /// Trace a type `T` that is simple enough that no samples of values are needed.
    /// * If `T` is an enum, the tracing iterates until all variants of `T` are covered.
    /// * Accumulate and return all the sampled values at the end.
    ///   This is merely a shortcut for `self.trace_type` with a fixed empty set of samples.
    pub fn trace_simple_type<'de, T>(&mut self) -> Result<(Format, Vec<T>)>
    where
        T: Deserialize<'de>,
    {
        static SAMPLES: Lazy<Samples> = Lazy::new(Samples::new);
        self.trace_type(&SAMPLES)
    }

    /// Same as `trace_type` for seeded deserialization.
    pub fn trace_type_with_seed<'de, S>(
        &mut self,
        samples: &'de Samples,
        seed: S,
    ) -> Result<(Format, Vec<S::Value>)>
    where
        S: DeserializeSeed<'de> + Clone,
    {
        let mut values = Vec::new();
        loop {
            let (format, value) = self.trace_type_once_with_seed(samples, seed.clone())?;
            values.push(value);
            if let Format::TypeName(name) = &format {
                if let Some(&progress) = self.incomplete_enums.get(name) {
                    // Restart the analysis to find more variants of T.
                    self.incomplete_enums.remove(name);
                    if let EnumProgress::NamedVariantsRemaining = progress {
                        values.pop().unwrap();
                    }
                    continue;
                }
            }
            return Ok((format, values));
        }
    }

    /// Finish tracing and recover a map of normalized formats.
    /// Returns an error if we detect incompletely traced types.
    /// This may happen in a few of cases:
    /// * We traced serialization of user-provided values but we are still missing the content
    ///   of an option type, the content of a sequence type, the key or the value of a dictionary type.
    /// * We traced deserialization of an enum type but we detect that some enum variants are still missing.
    pub fn registry(self) -> Result<Registry> {
        let mut registry = self.registry;
        for (name, format) in registry.iter_mut() {
            format
                .normalize()
                .map_err(|_| Error::UnknownFormatInContainer(name.clone()))?;
        }
        if self.incomplete_enums.is_empty() {
            Ok(registry)
        } else {
            Err(Error::MissingVariants(
                self.incomplete_enums.into_keys().collect(),
            ))
        }
    }

    /// Same as registry but always return a value, even if we detected issues.
    /// This should only be use for debugging.
    pub fn registry_unchecked(self) -> Registry {
        let mut registry = self.registry;
        for format in registry.values_mut() {
            format.normalize().unwrap_or(());
        }
        registry
    }

    pub(crate) fn record_container(
        &mut self,
        samples: &mut Samples,
        name: &'static str,
        format: ContainerFormat,
        value: Value,
        record_value: bool,
    ) -> Result<(Format, Value)> {
        self.registry.entry(name.to_string()).unify(format)?;
        if record_value {
            samples.values.insert(name, value.clone());
        }
        Ok((Format::TypeName(name.into()), value))
    }

    pub(crate) fn record_variant(
        &mut self,
        samples: &mut Samples,
        name: &'static str,
        variant_index: u32,
        variant_name: &'static str,
        variant: VariantFormat,
        variant_value: Value,
    ) -> Result<(Format, Value)> {
        let mut variants = BTreeMap::new();
        variants.insert(
            variant_index,
            Named {
                name: variant_name.into(),
                value: variant,
            },
        );
        let format = ContainerFormat::Enum(variants);
        let value = Value::Variant(variant_index, Box::new(variant_value));
        self.record_container(samples, name, format, value, false)
    }

    pub(crate) fn get_sample<'de, 'a>(
        &'a self,
        samples: &'de Samples,
        name: &'static str,
    ) -> Option<(&'a ContainerFormat, &'de Value)> {
        match samples.value(name) {
            Some(value) => {
                let format = self
                    .registry
                    .get(name)
                    .expect("recorded containers should have a format already");
                Some((format, value))
            }
            None => None,
        }
    }
}

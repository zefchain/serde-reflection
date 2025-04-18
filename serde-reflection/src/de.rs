// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    error::{Error, Result},
    format::{ContainerFormat, ContainerFormatEntry, Format, FormatHolder, Named, VariantFormat},
    trace::{EnumProgress, Samples, Tracer, VariantId},
    value::IntoSeqDeserializer,
};
use erased_discriminant::Discriminant;
use serde::de::{
    self,
    value::{BorrowedStrDeserializer, U32Deserializer},
    DeserializeSeed, IntoDeserializer, Visitor,
};
use std::collections::btree_map::{BTreeMap, Entry};

/// Deserialize a single value.
/// * The lifetime 'a is set by the deserialization call site and the
///   `&'a mut` references used to return tracing results.
/// * The lifetime 'de is fixed and the `&'de` reference meant to let us
///   borrow values from previous serialization runs.
pub struct Deserializer<'de, 'a> {
    tracer: &'a mut Tracer,
    samples: &'de Samples,
    format: &'a mut Format,
}

impl<'de, 'a> Deserializer<'de, 'a> {
    pub fn new(tracer: &'a mut Tracer, samples: &'de Samples, format: &'a mut Format) -> Self {
        Deserializer {
            tracer,
            samples,
            format,
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for Deserializer<'de, 'a> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported("deserialize_any"))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Bool)?;
        visitor.visit_bool(self.tracer.config.default_bool_value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::I8)?;
        visitor.visit_i8(self.tracer.config.default_i8_value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::I16)?;
        visitor.visit_i16(self.tracer.config.default_i16_value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::I32)?;
        visitor.visit_i32(self.tracer.config.default_i32_value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::I64)?;
        visitor.visit_i64(self.tracer.config.default_i64_value)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::I128)?;
        visitor.visit_i128(self.tracer.config.default_i128_value)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::U8)?;
        visitor.visit_u8(self.tracer.config.default_u8_value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::U16)?;
        visitor.visit_u16(self.tracer.config.default_u16_value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::U32)?;
        visitor.visit_u32(self.tracer.config.default_u32_value)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::U64)?;
        visitor.visit_u64(self.tracer.config.default_u64_value)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::U128)?;
        visitor.visit_u128(self.tracer.config.default_u128_value)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::F32)?;
        visitor.visit_f32(self.tracer.config.default_f32_value)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::F64)?;
        visitor.visit_f64(self.tracer.config.default_f64_value)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Char)?;
        visitor.visit_char(self.tracer.config.default_char_value)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Str)?;
        visitor.visit_borrowed_str(self.tracer.config.default_borrowed_str_value)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Str)?;
        visitor.visit_string(self.tracer.config.default_string_value.clone())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Bytes)?;
        visitor.visit_borrowed_bytes(self.tracer.config.default_borrowed_bytes_value)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Bytes)?;
        visitor.visit_byte_buf(self.tracer.config.default_byte_buf_value.clone())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut format = Format::unknown();
        self.format
            .unify(Format::Option(Box::new(format.clone())))?;
        if format.is_unknown() {
            let inner = Deserializer::new(self.tracer, self.samples, &mut format);
            visitor.visit_some(inner)
        } else {
            // Cut exploration.
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::Unit)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::TypeName(name.into()))?;
        self.tracer
            .registry
            .entry(name.to_string())
            .unify(ContainerFormat::UnitStruct)?;
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::TypeName(name.into()))?;
        if self.tracer.config.record_samples_for_newtype_structs {
            // If a value was recorded during serialization, use it.
            if let Some((format, sample)) = self.tracer.get_sample(self.samples, name) {
                return visitor
                    .visit_newtype_struct(sample.into_deserializer())
                    .map_err(|err| match err {
                        Error::DeserializationError(msg) => {
                            let mut format = format.clone();
                            format.reduce();
                            Error::UnexpectedDeserializationFormat(name, format, msg)
                        }
                        _ => err,
                    });
            }
        }
        // Pre-update the registry.
        let mut format = Format::unknown();
        self.tracer
            .registry
            .entry(name.to_string())
            .unify(ContainerFormat::NewTypeStruct(Box::new(format.clone())))?;
        // Compute the format.
        let inner = Deserializer::new(self.tracer, self.samples, &mut format);
        visitor.visit_newtype_struct(inner)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut format = Format::unknown();
        self.format.unify(Format::Seq(Box::new(format.clone())))?;
        if format.is_unknown() {
            // Simulate vector of size 1.
            let inner =
                SeqDeserializer::new(self.tracer, self.samples, std::iter::once(&mut format));
            visitor.visit_seq(inner)
        } else {
            // Cut exploration with a vector of size 0.
            let inner = SeqDeserializer::new(self.tracer, self.samples, std::iter::empty());
            visitor.visit_seq(inner)
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut formats: Vec<_> = std::iter::repeat_with(Format::unknown).take(len).collect();
        self.format.unify(Format::Tuple(formats.clone()))?;
        let inner = SeqDeserializer::new(self.tracer, self.samples, formats.iter_mut());
        visitor.visit_seq(inner)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::TypeName(name.into()))?;
        if self.tracer.config.record_samples_for_tuple_structs {
            // If a value was recorded during serialization, use it.
            if let Some((format, sample)) = self.tracer.get_sample(self.samples, name) {
                let result = || visitor.visit_seq(sample.seq_values()?.into_seq_deserializer());
                return result().map_err(|err| match err {
                    Error::DeserializationError(msg) => {
                        let mut format = format.clone();
                        format.reduce();
                        Error::UnexpectedDeserializationFormat(name, format, msg)
                    }
                    _ => err,
                });
            }
        }
        // Pre-update the registry.
        let mut formats: Vec<_> = std::iter::repeat_with(Format::unknown).take(len).collect();
        self.tracer
            .registry
            .entry(name.to_string())
            .unify(ContainerFormat::TupleStruct(formats.clone()))?;
        // Compute the formats.
        let inner = SeqDeserializer::new(self.tracer, self.samples, formats.iter_mut());
        visitor.visit_seq(inner)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut key_format = Format::unknown();
        let mut value_format = Format::unknown();
        self.format.unify(Format::Map {
            key: Box::new(key_format.clone()),
            value: Box::new(value_format.clone()),
        })?;
        if key_format.is_unknown() || value_format.is_unknown() {
            // Simulate a map with one entry.
            let inner = SeqDeserializer::new(
                self.tracer,
                self.samples,
                vec![&mut key_format, &mut value_format].into_iter(),
            );
            visitor.visit_map(inner)
        } else {
            // Stop exploration.
            let inner = SeqDeserializer::new(self.tracer, self.samples, std::iter::empty());
            visitor.visit_map(inner)
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.format.unify(Format::TypeName(name.into()))?;
        if self.tracer.config.record_samples_for_structs {
            // If a value was recorded during serialization, use it.
            if let Some((format, sample)) = self.tracer.get_sample(self.samples, name) {
                let result = || visitor.visit_seq(sample.seq_values()?.into_seq_deserializer());
                return result().map_err(|err| match err {
                    Error::DeserializationError(msg) => {
                        let mut format = format.clone();
                        format.reduce();
                        Error::UnexpectedDeserializationFormat(name, format, msg)
                    }
                    _ => err,
                });
            }
        }
        // Pre-update the registry.
        let mut formats: Vec<_> = fields
            .iter()
            .map(|&name| Named {
                name: name.into(),
                value: Format::unknown(),
            })
            .collect();
        self.tracer
            .registry
            .entry(name.to_string())
            .unify(ContainerFormat::Struct(formats.clone()))?;
        // Compute the formats.
        let inner = SeqDeserializer::new(
            self.tracer,
            self.samples,
            formats.iter_mut().map(|named| &mut named.value),
        );
        visitor.visit_seq(inner)
    }

    // Assumption: The first variant(s) should be "base cases", i.e. not cause infinite recursion
    // while constructing sample values.
    #[allow(clippy::map_entry)] // false positive https://github.com/rust-lang/rust-clippy/issues/9470
    fn deserialize_enum<V>(
        self,
        enum_name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if variants.is_empty() {
            return Err(Error::NotSupported("deserialize_enum with 0 variants"));
        }

        let enum_type_id = typeid::of::<V::Value>();
        self.format.unify(Format::TypeName(enum_name.into()))?;
        // Pre-update the registry.
        self.tracer
            .registry
            .entry(enum_name.to_string())
            .unify(ContainerFormat::Enum(BTreeMap::new()))?;
        let known_variants = match self.tracer.registry.get_mut(enum_name) {
            Some(ContainerFormat::Enum(x)) => x,
            _ => unreachable!(),
        };

        // If the enum is already marked as incomplete, visit the first index, hoping
        // to avoid recursion.
        if self.tracer.incomplete_enums.contains_key(enum_name) {
            return visitor.visit_enum(EnumDeserializer::new(
                self.tracer,
                self.samples,
                VariantId::Index(0),
                &mut VariantFormat::unknown(),
            ));
        }

        // First, visit each of the variants by name according to `variants`. Later, we
        // will revisit them by u32 index until an index matching each of the named
        // variants has been determined.
        let provisional_min = u32::MAX - (variants.len() - 1) as u32;
        for (i, &variant_name) in variants.iter().enumerate() {
            if self
                .tracer
                .discriminants
                .contains_key(&(enum_type_id, VariantId::Name(variant_name)))
            {
                continue;
            }
            // Insert into known_variants with a provisional index.
            let provisional_index = provisional_min + i as u32;
            let variant = known_variants
                .entry(provisional_index)
                .or_insert_with(|| Named {
                    name: variant_name.to_owned(),
                    value: VariantFormat::unknown(),
                });
            self.tracer
                .incomplete_enums
                .insert(enum_name.into(), EnumProgress::NamedVariantsRemaining);
            // Compute the discriminant and format for this variant.
            let mut value = variant.value.clone();
            let enum_value = visitor.visit_enum(EnumDeserializer::new(
                self.tracer,
                self.samples,
                VariantId::Name(variant_name),
                &mut value,
            ))?;
            let discriminant = Discriminant::of(&enum_value);
            self.tracer
                .discriminants
                .insert((enum_type_id, VariantId::Name(variant_name)), discriminant);
            return Ok(enum_value);
        }

        // We know the discriminant for every variant name. Now visit them again
        // by index to find the u32 id that goes with each name.
        //
        // If there are no provisional entries waiting for an index, just go
        // with index 0.
        let mut index = 0;
        if known_variants.range(provisional_min..).next().is_some() {
            self.tracer
                .incomplete_enums
                .insert(enum_name.into(), EnumProgress::IndexedVariantsRemaining);
            while known_variants.contains_key(&index)
                && self
                    .tracer
                    .discriminants
                    .contains_key(&(enum_type_id, VariantId::Index(index)))
            {
                index += 1;
            }
        }

        // Compute the discriminant and format for this variant.
        let mut value = VariantFormat::unknown();
        let enum_value = visitor.visit_enum(EnumDeserializer::new(
            self.tracer,
            self.samples,
            VariantId::Index(index),
            &mut value,
        ))?;
        let discriminant = Discriminant::of(&enum_value);
        self.tracer.discriminants.insert(
            (enum_type_id, VariantId::Index(index)),
            discriminant.clone(),
        );

        // Rewrite provisional entries for which we now know a u32 index.
        let known_variants = match self.tracer.registry.get_mut(enum_name) {
            Some(ContainerFormat::Enum(x)) => x,
            _ => unreachable!(),
        };

        let mut has_indexed_variants_remaining = false;
        for provisional_index in provisional_min..=u32::MAX {
            if let Entry::Occupied(provisional_entry) = known_variants.entry(provisional_index) {
                if self.tracer.discriminants
                    [&(enum_type_id, VariantId::Name(&provisional_entry.get().name))]
                    == discriminant
                {
                    let provisional_entry = provisional_entry.remove();
                    match known_variants.entry(index) {
                        Entry::Vacant(vacant) => {
                            vacant.insert(provisional_entry);
                        }
                        Entry::Occupied(mut existing_entry) => {
                            // Discard the provisional entry's name and just
                            // keep the existing one.
                            existing_entry
                                .get_mut()
                                .value
                                .unify(provisional_entry.value)?;
                        }
                    }
                } else {
                    has_indexed_variants_remaining = true;
                }
            }
        }
        if let Some(existing_entry) = known_variants.get_mut(&index) {
            existing_entry.value.unify(value)?;
        }
        if has_indexed_variants_remaining {
            // Signal that the top-level tracing must continue.
            self.tracer
                .incomplete_enums
                .insert(enum_name.into(), EnumProgress::IndexedVariantsRemaining);
        } else {
            // Signal that the top-level tracing is complete for this enum.
            self.tracer.incomplete_enums.remove(enum_name);
        }

        Ok(enum_value)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported("deserialize_identifier"))
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::NotSupported("deserialize_ignored_any"))
    }

    fn is_human_readable(&self) -> bool {
        self.tracer.config.is_human_readable
    }
}

struct SeqDeserializer<'de, 'a, I> {
    tracer: &'a mut Tracer,
    samples: &'de Samples,
    formats: I,
}

impl<'de, 'a, I> SeqDeserializer<'de, 'a, I> {
    fn new(tracer: &'a mut Tracer, samples: &'de Samples, formats: I) -> Self {
        Self {
            tracer,
            samples,
            formats,
        }
    }
}

impl<'de, 'a, I> de::SeqAccess<'de> for SeqDeserializer<'de, 'a, I>
where
    I: Iterator<Item = &'a mut Format>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        let format = match self.formats.next() {
            Some(x) => x,
            None => return Ok(None),
        };
        let inner = Deserializer::new(self.tracer, self.samples, format);
        seed.deserialize(inner).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        self.formats.size_hint().1
    }
}

impl<'de, 'a, I> de::MapAccess<'de> for SeqDeserializer<'de, 'a, I>
where
    // Must have an even number of elements
    I: Iterator<Item = &'a mut Format>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        let format = match self.formats.next() {
            Some(x) => x,
            None => return Ok(None),
        };
        let inner = Deserializer::new(self.tracer, self.samples, format);
        seed.deserialize(inner).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let format = match self.formats.next() {
            Some(x) => x,
            None => unreachable!(),
        };
        let inner = Deserializer::new(self.tracer, self.samples, format);
        seed.deserialize(inner)
    }

    fn size_hint(&self) -> Option<usize> {
        self.formats.size_hint().1.map(|x| x / 2)
    }
}

struct EnumDeserializer<'de, 'a> {
    tracer: &'a mut Tracer,
    samples: &'de Samples,
    variant_id: VariantId<'static>,
    format: &'a mut VariantFormat,
}

impl<'de, 'a> EnumDeserializer<'de, 'a> {
    fn new(
        tracer: &'a mut Tracer,
        samples: &'de Samples,
        variant_id: VariantId<'static>,
        format: &'a mut VariantFormat,
    ) -> Self {
        Self {
            tracer,
            samples,
            variant_id,
            format,
        }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumDeserializer<'de, 'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let value = match self.variant_id {
            VariantId::Index(index) => seed.deserialize(U32Deserializer::new(index)),
            VariantId::Name(name) => seed.deserialize(BorrowedStrDeserializer::new(name)),
        }?;
        Ok((value, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for EnumDeserializer<'de, 'a> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        self.format.unify(VariantFormat::Unit)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let mut format = Format::unknown();
        self.format
            .unify(VariantFormat::NewType(Box::new(format.clone())))?;
        let inner = Deserializer::new(self.tracer, self.samples, &mut format);
        seed.deserialize(inner)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut formats: Vec<_> = std::iter::repeat_with(Format::unknown).take(len).collect();
        self.format.unify(VariantFormat::Tuple(formats.clone()))?;
        let inner = SeqDeserializer::new(self.tracer, self.samples, formats.iter_mut());
        visitor.visit_seq(inner)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut formats: Vec<_> = fields
            .iter()
            .map(|&name| Named {
                name: name.into(),
                value: Format::unknown(),
            })
            .collect();
        self.format.unify(VariantFormat::Struct(formats.clone()))?;

        let inner = SeqDeserializer::new(
            self.tracer,
            self.samples,
            formats.iter_mut().map(|named| &mut named.value),
        );
        visitor.visit_seq(inner)
    }
}

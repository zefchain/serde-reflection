// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    common,
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig, Encoding,
};
use heck::{CamelCase, MixedCase, SnakeCase};
use include_dir::include_dir as include_directory;
use serde_reflection::{ContainerFormat, Format, FormatHolder, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap, HashMap},
    io::{Result, Write},
    path::{Path, PathBuf},
};

/// Main configuration object for code-generation in Dart.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
}

/// Shared state for the code generation of a Dart source file.
struct DartEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
    /// Current namespace (e.g. vec!["my_package", "my_module", "MyClass"])
    current_namespace: Vec<String>,
    // A reference to the registry so we can look up information for special cases
    registry: &'a Registry,
}

impl<'a> CodeGenerator<'a> {
    /// Create a Dart code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        let mut external_qualified_names = HashMap::new();
        for (namespace, names) in &config.external_definitions {
            for name in names {
                external_qualified_names
                    .insert(name.to_string(), format!("{}.{}", namespace, name));
            }
        }
        Self { config }
    }

    /// Output class definitions for `registry`.
    pub fn output(&self, install_dir: std::path::PathBuf, registry: &Registry) -> Result<()> {
        let current_namespace = self
            .config
            .module_name
            .split('.')
            .map(String::from)
            .collect::<Vec<_>>();

        let mut dir_path = install_dir;
        std::fs::create_dir_all(&dir_path)?;
        dir_path = dir_path.join("lib").join("src");
        for part in &current_namespace {
            dir_path = dir_path.join(part);
        }
        std::fs::create_dir_all(&dir_path)?;

        for (name, format) in registry {
            self.write_container_class(
                &dir_path,
                current_namespace.clone(),
                name,
                format,
                registry,
            )?;
        }
        self.write_helper_class(&dir_path, current_namespace.clone(), registry)?;
        self.write_library(&dir_path, current_namespace, registry)?;
        Ok(())
    }

    fn write_library(
        &self,
        install_dir: &Path,
        current_namespace: Vec<String>,
        registry: &Registry,
    ) -> Result<()> {
        let mut file =
            std::fs::File::create(install_dir.join(self.config.module_name.clone() + ".dart"))?;
        let mut emitter = DartEmitter {
            out: IndentedWriter::new(&mut file, IndentConfig::Space(2)),
            generator: self,
            current_namespace,
            registry,
        };

        writeln!(
            &mut emitter.out,
            r#"// ignore_for_file: unused_import
library {}_types;

import 'dart:typed_data';
import 'package:meta/meta.dart';
import 'package:tuple/tuple.dart';
import '../serde/serde.dart';"#,
            self.config.module_name,
        )?;

        for encoding in &self.config.encodings {
            writeln!(
                &mut emitter.out,
                "import '../{0}/{0}.dart';",
                encoding.name()
            )?;
        }

        if let Some(files) = &self.config.external_definitions.get("import") {
            for file in *files {
                writeln!(&mut emitter.out, "import '{0}';", file)?;
            }
        }

        writeln!(&mut emitter.out, "\nexport '../serde/serde.dart';")?;

        writeln!(&mut emitter.out, "\npart 'trait_helpers.dart';")?;
        for name in registry.keys() {
            writeln!(&mut emitter.out, "part '{}.dart';", name.to_snake_case())?;
        }

        Ok(())
    }

    fn write_container_class(
        &self,
        dir_path: &std::path::Path,
        current_namespace: Vec<String>,
        name: &str,
        format: &ContainerFormat,
        registry: &Registry,
    ) -> Result<()> {
        let mut file =
            std::fs::File::create(dir_path.join(name.to_string().to_snake_case() + ".dart"))?;
        let mut emitter = DartEmitter {
            out: IndentedWriter::new(&mut file, IndentConfig::Space(2)),
            generator: self,
            current_namespace,
            registry,
        };

        emitter.output_preamble()?;
        emitter.output_container(name, format)
    }

    fn write_helper_class(
        &self,
        dir_path: &std::path::Path,
        current_namespace: Vec<String>,
        registry: &Registry,
    ) -> Result<()> {
        let mut file = std::fs::File::create(dir_path.join("trait_helpers.dart"))?;
        let mut emitter = DartEmitter {
            out: IndentedWriter::new(&mut file, IndentConfig::Space(2)),
            generator: self,
            current_namespace,
            registry,
        };

        emitter.output_preamble()?;
        emitter.output_trait_helpers(registry)
    }
}

impl<'a, T> DartEmitter<'a, T>
where
    T: Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        writeln!(
            self.out,
            "// ignore_for_file: type=lint, type=warning\npart of '{}.dart';",
            self.generator.config.module_name
        )?;

        Ok(())
    }

    fn get_field_container_type(&self, name: &str) -> Option<&ContainerFormat> {
        match self.registry.get(name) {
            Some(container) => Some(container),
            None => None,
        }
    }

    // in Dart enums cannot have a static method added to them
    // yet so we must call the extension class instead
    fn get_class(&self, name: &str) -> String {
        if self.generator.config.c_style_enums {
            use ContainerFormat::Enum;
            match self.get_field_container_type(name) {
                // if we have an enum AND all of that enum's members are Unit
                // then we will generate an extension class name
                Some(Enum(variants))
                    if variants.values().all(|f| f.value == VariantFormat::Unit) =>
                {
                    format!("{}Extension", name)
                }
                _ => name.to_string(),
            }
        } else {
            name.to_string()
        }
    }

    fn quote_qualified_name(&self, name: &str) -> String {
        match name {
            "List" => "List_".to_string(),
            "Map" => "Map_".to_string(),
            name => name.to_string(),
        }
    }

    fn quote_field(&self, name: &str) -> String {
        match name {
            "hashCode" => "hashCode_".to_string(),
            "runtimeType" => "runtimeType_".to_string(),
            name => name.to_string(),
        }
    }

    fn quote_type(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(x) => self.quote_qualified_name(x),
            Unit => "Unit".into(),
            Bool => "bool".into(),
            I8 => "int".into(),
            I16 => "int".into(),
            I32 => "int".into(),
            I64 => "int".into(),
            I128 => "Int128".into(),
            U8 => "int".into(),
            U16 => "int".into(),
            U32 => "int".into(),
            U64 => "Uint64".into(),
            U128 => "Uint128".into(),
            F32 => "double".into(),
            F64 => "double".into(),
            Char => "int".into(),
            Str => "String".into(),
            Bytes => "Bytes".into(),

            Option(format) => format!("{}?", self.quote_type(format)),
            Seq(format) => format!("List<{}>", self.quote_type(format)),
            Map { key, value } => {
                format!("Map<{}, {}>", self.quote_type(key), self.quote_type(value))
            }
            Tuple(formats) => format!("Tuple{}<{}>", formats.len(), self.quote_types(formats)),
            TupleArray { content, size: _ } => format!("List<{}>", self.quote_type(content)),
            Variable(_) => panic!("unexpected value"),
        }
    }

    fn quote_types(&self, formats: &[Format]) -> String {
        formats
            .iter()
            .map(|f| self.quote_type(f))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn quote_serialize_value(&self, value: &str, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(_) => format!("{}.serialize(serializer);", value),
            Unit => format!("serializer.serializeUnit({});", value),
            Bool => format!("serializer.serializeBool({});", value),
            I8 => format!("serializer.serializeInt8({});", value),
            I16 => format!("serializer.serializeInt16({});", value),
            I32 => format!("serializer.serializeInt32({});", value),
            I64 => format!("serializer.serializeInt64({});", value),
            I128 => format!("serializer.serializeInt128({});", value),
            U8 => format!("serializer.serializeUint8({});", value),
            U16 => format!("serializer.serializeUint16({});", value),
            U32 => format!("serializer.serializeUint32({});", value),
            U64 => format!("serializer.serializeUint64({});", value),
            U128 => format!("serializer.serializeUint128({});", value),
            F32 => format!("serializer.serializeFloat32({});", value),
            F64 => format!("serializer.serializeFloat64({});", value),
            Char => format!("serializer.serializeChar({});", value),
            Str => format!("serializer.serializeString({});", value),
            Bytes => format!("serializer.serializeBytes({});", value),
            _ => format!(
                "{}.serialize{}({}, serializer);",
                self.quote_qualified_name("TraitHelpers"),
                common::mangle_type(format).to_camel_case(),
                value
            ),
        }
    }

    fn quote_deserialize(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(name) => {
                format!(
                    "{}.deserialize(deserializer)",
                    self.quote_qualified_name(&self.get_class(name))
                )
            }
            Unit => "deserializer.deserializeUnit()".to_string(),
            Bool => "deserializer.deserializeBool()".to_string(),
            I8 => "deserializer.deserializeInt8()".to_string(),
            I16 => "deserializer.deserializeInt16()".to_string(),
            I32 => "deserializer.deserializeInt32()".to_string(),
            I64 => "deserializer.deserializeInt64()".to_string(),
            I128 => "deserializer.deserializeInt128()".to_string(),
            U8 => "deserializer.deserializeUint8()".to_string(),
            U16 => "deserializer.deserializeUint16()".to_string(),
            U32 => "deserializer.deserializeUint32()".to_string(),
            U64 => "deserializer.deserializeUint64()".to_string(),
            U128 => "deserializer.deserializeUint128()".to_string(),
            F32 => "deserializer.deserializeFloat32()".to_string(),
            F64 => "deserializer.deserializeFloat64()".to_string(),
            Char => "deserializer.deserializeChar()".to_string(),
            Str => "deserializer.deserializeString()".to_string(),
            Bytes => "deserializer.deserializeBytes()".to_string(),
            _ => format!(
                "{}.deserialize{}(deserializer)",
                self.quote_qualified_name("TraitHelpers"),
                common::mangle_type(format).to_camel_case(),
            ),
        }
    }

    fn enter_class(&mut self, name: &str) {
        self.out.indent();
        self.current_namespace.push(name.to_string());
    }

    fn leave_class(&mut self) {
        self.out.unindent();
        self.current_namespace.pop();
    }

    fn output_trait_helpers(&mut self, registry: &Registry) -> Result<()> {
        let mut subtypes = BTreeMap::new();
        for format in registry.values() {
            format
                .visit(&mut |f| {
                    if Self::needs_helper(f) {
                        subtypes.insert(common::mangle_type(f), f.clone());
                    }
                    Ok(())
                })
                .unwrap();
        }
        writeln!(self.out, "class TraitHelpers {{")?;
        self.enter_class("TraitHelpers");
        for (mangled_name, subtype) in &subtypes {
            self.output_serialization_helper(mangled_name, subtype)?;
            self.output_deserialization_helper(mangled_name, subtype)?;
        }
        self.leave_class();
        writeln!(self.out, "}}\n")
    }

    fn needs_helper(format: &Format) -> bool {
        use Format::*;
        matches!(
            format,
            Option(_) | Seq(_) | Map { .. } | Tuple(_) | TupleArray { .. }
        )
    }

    fn output_serialization_helper(&mut self, name: &str, format0: &Format) -> Result<()> {
        use Format::*;

        write!(
            self.out,
            "static void serialize{}({} value, BinarySerializer serializer) {{",
            name.to_camel_case(),
            self.quote_type(format0)
        )?;
        self.out.indent();
        match format0 {
            Option(format) => {
                write!(
                    self.out,
                    r#"
if (value == null) {{
    serializer.serializeOptionTag(false);
}} else {{
    serializer.serializeOptionTag(true);
    {}
}}
"#,
                    self.quote_serialize_value("value", format)
                )?;
            }

            Seq(format) => {
                write!(
                    self.out,
                    r#"
serializer.serializeLength(value.length);
for (final item in value) {{
    {}
}}
"#,
                    self.quote_serialize_value("item", format)
                )?;
            }

            Map { key, value } => {
                write!(
                    self.out,
                    r#"
serializer.serializeLength(value.length);
final offsets = List<int>.filled(value.length, 0);
var count = 0;
value.entries.forEach((entry) {{
  offsets[count++] = serializer.offset;
  {}
  {}
}});
"#,
                    self.quote_serialize_value("entry.key", key),
                    self.quote_serialize_value("entry.value", value)
                )?;
            }

            Tuple(formats) => {
                writeln!(self.out)?;
                for (index, format) in formats.iter().enumerate() {
                    let expr = format!("value.item{}", index + 1);
                    writeln!(self.out, "{}", self.quote_serialize_value(&expr, format))?;
                }
            }

            TupleArray { content, size } => {
                write!(
                    self.out,
                    r#"
assert (value.length == {});
for (final item in value) {{
    {}
}}
"#,
                    size,
                    self.quote_serialize_value("item", content),
                )?;
            }

            _ => panic!("unexpected case"),
        }
        self.out.unindent();
        writeln!(self.out, "}}\n")
    }

    fn output_deserialization_helper(&mut self, name: &str, format0: &Format) -> Result<()> {
        use Format::*;

        write!(
            self.out,
            "static {} deserialize{}(BinaryDeserializer deserializer) {{",
            self.quote_type(format0),
            name.to_camel_case(),
        )?;
        self.out.indent();
        match format0 {
            Option(format) => {
                write!(
                    self.out,
                    r#"
final tag = deserializer.deserializeOptionTag();
if (tag) {{
    return {};
}} else {{
    return null;
}}
"#,
                    self.quote_deserialize(format),
                )?;
            }

            Seq(format) => {
                write!(
                    self.out,
                    r#"
final length = deserializer.deserializeLength();
return List.generate(length, (_) => {0});
"#,
                    self.quote_deserialize(format)
                )?;
            }

            Map { key, value } => {
                write!(
                    self.out,
                    r#"
final length = deserializer.deserializeLength();
final obj = <{0}, {1}>{{}};
var previousKeyStart = 0;
var previousKeyEnd = 0;
for (var i = 0; i < length; i++) {{
    final keyStart = deserializer.offset;
    {0} key = {2};
    final keyEnd = deserializer.offset;
    if (i > 0) {{
        deserializer.checkThatKeySlicesAreIncreasing(
            Slice(previousKeyStart, previousKeyEnd),
            Slice(keyStart, keyEnd),
        );
    }}
    previousKeyStart = keyStart;
    previousKeyEnd = keyEnd;
    {1} value = {3};
    obj.putIfAbsent(key, () => value);
}}
return obj;
"#,
                    self.quote_type(key),
                    self.quote_type(value),
                    self.quote_deserialize(key),
                    self.quote_deserialize(value),
                )?;
            }

            Tuple(formats) => {
                write!(
                    self.out,
                    r#"
return {}({}
);
"#,
                    self.quote_type(format0),
                    formats
                        .iter()
                        .map(|f| format!("\n    {}", self.quote_deserialize(f)))
                        .collect::<Vec<_>>()
                        .join(",")
                )?;
            }

            TupleArray { content, size } => {
                write!(
                    self.out,
                    r#"
final obj = List<{0}>.filled({1}, 0);
for (var i = 0; i < {1}; i++) {{
    obj[i] = {2};
}}
return obj;
"#,
                    self.quote_type(content),
                    size,
                    self.quote_deserialize(content)
                )?;
            }

            _ => panic!("unexpected case"),
        }
        self.out.unindent();
        writeln!(self.out, "}}\n")
    }

    fn output_container(&mut self, name: &str, format: &ContainerFormat) -> Result<()> {
        use ContainerFormat::*;
        let fields = match format {
            UnitStruct => Vec::new(),
            NewTypeStruct(format) => {
                vec![Named {
                    name: "value".to_string(),
                    value: format.as_ref().clone(),
                }]
            }
            TupleStruct(formats) => formats
                .iter()
                .enumerate()
                .map(|(i, f)| Named {
                    name: format!("field{}", i),
                    value: f.clone(),
                })
                .collect::<Vec<_>>(),
            Struct(fields) => fields.clone(),
            Enum(variants) => {
                // When we find an enum with all Unit variants, we ser/de as a regular Dart enum.
                if self.generator.config.c_style_enums
                    && variants.values().all(|f| f.value == VariantFormat::Unit)
                {
                    self.output_enum_container(name, variants)?;
                } else {
                    self.output_enum_class_container(name, variants)?;
                }
                return Ok(());
            }
        };
        self.output_struct_or_variant_container(None, None, name, &fields)
    }

    fn output_struct_or_variant_container(
        &mut self,
        variant_base: Option<&str>,
        variant_index: Option<u32>,
        name: &str,
        fields: &[Named<Format>],
    ) -> Result<()> {
        let field_count = fields.len();

        // Beginning of class
        writeln!(self.out)?;
        self.output_comment(name)?;
        if let Some(base) = variant_base {
            writeln!(
                self.out,
                "@immutable\nclass {} extends {} {{",
                self.quote_qualified_name(name),
                base
            )?;
        } else {
            writeln!(
                self.out,
                "@immutable\nclass {} {{",
                self.quote_qualified_name(name)
            )?;
        }
        self.enter_class(name);

        // Constructor.
        writeln!(
            self.out,
            "const {}({}",
            self.quote_qualified_name(name),
            if !fields.is_empty() { "{" } else { "" }
        )?;
        self.out.indent();
        for field in fields.iter() {
            let field_name = self.quote_field(&field.name.to_mixed_case());
            match &field.value {
                Format::Option(_) => writeln!(self.out, "this.{},", field_name)?,
                _ => writeln!(self.out, "required this.{},", field_name)?,
            }
        }
        self.out.unindent();
        if variant_base.is_some() {
            writeln!(
                self.out,
                "{}) : super();",
                if !fields.is_empty() { "}" } else { "" }
            )?;
        } else {
            writeln!(self.out, "{});", if !fields.is_empty() { "}" } else { "" })?;
        }

        if self.generator.config.serialization {
            // Deserialize (struct) or Load (variant)
            if variant_index.is_none() {
                writeln!(
                    self.out,
                    "\nstatic {} deserialize(BinaryDeserializer deserializer) {{",
                    self.quote_qualified_name(name)
                )?;
            } else {
                writeln!(
                    self.out,
                    "\nstatic {} load(BinaryDeserializer deserializer) {{",
                    self.quote_qualified_name(name)
                )?;
            }

            self.out.indent();
            writeln!(self.out, "deserializer.increaseContainerDepth();")?;
            writeln!(
                self.out,
                "final instance = {}(",
                self.quote_qualified_name(name)
            )?;
            self.out.indent();
            for field in fields {
                writeln!(
                    self.out,
                    "{}: {},",
                    self.quote_field(&field.name.to_mixed_case()),
                    self.quote_deserialize(&field.value)
                )?;
            }
            self.out.unindent();
            writeln!(self.out, ");")?;
            writeln!(self.out, "deserializer.decreaseContainerDepth();")?;
            writeln!(self.out, "return instance;")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            if variant_index.is_none() {
                for encoding in &self.generator.config.encodings {
                    self.output_class_deserialize_for_encoding(name, *encoding)?;
                }
            }
        }

        // Fields
        if !fields.is_empty() {
            writeln!(self.out)?;
        }
        for field in fields {
            writeln!(
                self.out,
                "final {} {};",
                self.quote_type(&field.value),
                self.quote_field(&field.name.to_mixed_case())
            )?;
        }
        if !fields.is_empty() {
            writeln!(self.out)?;

            let cls_name = self.quote_qualified_name(name);
            writeln!(self.out, "{} copyWith({{", cls_name)?;
            self.out.indent();
            for field in fields {
                let field_name = self.quote_field(&field.name.to_mixed_case());
                let field_type = self.quote_type(&field.value);

                match field.value {
                    Format::Option(_) => {
                        writeln!(self.out, "{} Function()? {},", field_type, field_name)?
                    }
                    _ => writeln!(self.out, "{}? {},", field_type, field_name)?,
                }
            }
            self.out.unindent();
            writeln!(self.out, "}}) {{")?;
            self.out.indent();
            writeln!(self.out, "return {}(", cls_name)?;
            self.out.indent();

            for field in fields {
                let field_name = self.quote_field(&field.name.to_mixed_case());

                match field.value {
                    Format::Option(_) => {
                        writeln!(self.out, "{0}: {0} == null ? this.{0} : {0}(),", field_name)?
                    }
                    _ => writeln!(self.out, "{0}: {0} ?? this.{0},", field_name)?,
                }
            }
            self.out.unindent();
            writeln!(self.out, ");")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
        }

        // Serialize
        if self.generator.config.serialization {
            writeln!(self.out, "\nvoid serialize(BinarySerializer serializer) {{",)?;
            self.out.indent();
            writeln!(self.out, "serializer.increaseContainerDepth();")?;
            if let Some(index) = variant_index {
                writeln!(self.out, "serializer.serializeVariantIndex({});", index)?;
            }
            for field in fields {
                writeln!(
                    self.out,
                    "{}",
                    self.quote_serialize_value(
                        &self.quote_field(&field.name.to_mixed_case()),
                        &field.value
                    )
                )?;
            }
            writeln!(self.out, "serializer.decreaseContainerDepth();")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            if variant_index.is_none() {
                for encoding in &self.generator.config.encodings {
                    self.output_class_serialize_for_encoding(*encoding)?;
                }
            }
        }

        // Equality
        write!(self.out, "\n@override")?;
        write!(self.out, "\nbool operator ==(Object other) {{")?;
        self.out.indent();

        writeln!(self.out, "\nif (identical(this, other)) return true;")?;
        writeln!(
            self.out,
            "if (other.runtimeType != runtimeType) return false;"
        )?;
        write!(self.out, "\nreturn other is {}", name)?;

        self.out.indent();
        for field in fields.iter() {
            // because the Dart functions of listEquals and mapEquals accept nullable
            // we only care about the data type and can discard the enclosing Format::Option
            let value = if let Format::Option(value) = &field.value {
                value
            } else {
                &field.value
            };

            let stmt = match value {
                Format::Seq(_) => {
                    format!(
                        "listEquals({0}, other.{0})",
                        self.quote_field(&field.name.to_mixed_case())
                    )
                }
                Format::TupleArray {
                    content: _,
                    size: _,
                } => format!(
                    "listEquals({0}, other.{0})",
                    self.quote_field(&field.name.to_mixed_case())
                ),
                Format::Map { .. } => {
                    format!(
                        "mapEquals({0}, other.{0})",
                        self.quote_field(&field.name.to_mixed_case())
                    )
                }
                _ => format!(
                    "{0} == other.{0}",
                    self.quote_field(&field.name.to_mixed_case())
                ),
            };

            write!(self.out, "\n&& {}", stmt)?;
        }
        writeln!(self.out, ";")?;
        self.out.unindent();

        self.out.unindent();
        writeln!(self.out, "}}")?;

        // Hashing
        write!(self.out, "\n@override")?;
        if field_count == 0 {
            writeln!(self.out, "\nint get hashCode => runtimeType.hashCode;")?;
        } else if field_count == 1 {
            writeln!(
                self.out,
                "\nint get hashCode => {}.hashCode;",
                fields.first().unwrap().name.to_mixed_case()
            )?;
        } else {
            let use_hash_all = field_count > 20;

            if use_hash_all {
                writeln!(self.out, "\nint get hashCode => Object.hashAll([")?;
            } else {
                writeln!(self.out, "\nint get hashCode => Object.hash(")?;
            }

            self.out.indent();
            self.out.indent();
            self.out.indent();

            for field in fields {
                writeln!(
                    self.out,
                    "{},",
                    self.quote_field(&field.name.to_mixed_case())
                )?;
            }

            self.out.unindent();

            if use_hash_all {
                writeln!(self.out, "]);")?;
            } else {
                writeln!(self.out, ");")?;
            }

            self.out.unindent();
            self.out.unindent();
        }

        // Generate a toString implementation in each class
        writeln!(self.out, "\n@override\nString toString() {{")?;
        self.out.indent();
        writeln!(self.out, "String? fullString;")?;
        writeln!(self.out, "\nassert(() {{")?;
        self.out.indent();
        writeln!(self.out, "fullString = '$runtimeType('")?;
        self.out.indent();
        for (index, field) in fields.iter().enumerate() {
            if index == field_count - 1 {
                writeln!(
                    self.out,
                    "'{0}: ${0}'",
                    self.quote_field(&field.name.to_mixed_case())
                )?;
            } else {
                writeln!(
                    self.out,
                    "'{0}: ${0}, '",
                    self.quote_field(&field.name.to_mixed_case())
                )?;
            }
        }
        writeln!(self.out, "')';")?;
        self.out.unindent();
        writeln!(self.out, "return true;")?;
        self.out.unindent();
        writeln!(self.out, "}}());")?;
        writeln!(self.out, "\nreturn fullString ?? '{}';", name)?;
        self.out.unindent();
        writeln!(self.out, "}}")?;

        self.out.unindent();
        // End of class
        self.leave_class();
        writeln!(self.out, "}}")
    }

    fn output_class_serialize_for_encoding(&mut self, encoding: Encoding) -> Result<()> {
        writeln!(
            self.out,
            r#"
Uint8List {0}Serialize() {{
    final serializer = {1}Serializer();
    serialize(serializer);
    return serializer.bytes;
}}"#,
            encoding.name(),
            encoding.name().to_camel_case(),
        )
    }

    fn output_class_deserialize_for_encoding(
        &mut self,
        name: &str,
        encoding: Encoding,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
static {klass} {encoding}Deserialize(Uint8List input) {{
  final deserializer = {encoding_class}Deserializer(input);
  final value = {static_class}.deserialize(deserializer);
  if (deserializer.offset < input.length) {{
    throw Exception('Some input bytes were not read');
  }}
  return value;
}}"#,
            klass = self.quote_qualified_name(name),
            static_class = self.quote_qualified_name(&self.get_class(name)),
            encoding = encoding.name(),
            encoding_class = encoding.name().to_camel_case()
        )
    }

    fn output_enum_container(
        &mut self,
        name: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "enum {} {{", self.quote_qualified_name(name))?;
        self.enter_class(name);

        for variant in variants.values() {
            writeln!(
                self.out,
                "{},",
                self.quote_field(&variant.name.to_mixed_case())
            )?;
        }

        self.out.unindent();
        writeln!(self.out, "}}\n")?;

        if self.generator.config.serialization {
            writeln!(
                self.out,
                "extension {name}Extension on {n} {{",
                name = name,
                n = self.quote_qualified_name(name)
            )?;
            self.out.indent();
            write!(
                self.out,
                "static {} deserialize(BinaryDeserializer deserializer) {{",
                self.quote_qualified_name(name)
            )?;
            self.out.indent();
            writeln!(
                self.out,
                r#"
final index = deserializer.deserializeVariantIndex();
switch (index) {{"#,
            )?;
            self.out.indent();
            for (index, variant) in variants {
                writeln!(
                    self.out,
                    "case {}: return {}.{};",
                    index,
                    self.quote_qualified_name(name),
                    self.quote_field(&variant.name.to_mixed_case()),
                )?;
            }
            writeln!(
                self.out,
                "default: throw Exception('Unknown variant index for {}: ' + index.toString());",
                self.quote_qualified_name(name),
            )?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
            self.out.unindent();
            writeln!(self.out, "}}\n")?;

            write!(self.out, "void serialize(BinarySerializer serializer) {{")?;

            self.out.indent();
            writeln!(
                self.out,
                r#"
switch (this) {{"#,
            )?;
            self.out.indent();
            for (index, variant) in variants {
                writeln!(
                    self.out,
                    "case {}.{}: return serializer.serializeVariantIndex({});",
                    self.quote_qualified_name(name),
                    self.quote_field(&variant.name.to_mixed_case()),
                    index,
                )?;
            }
            self.out.unindent();
            writeln!(self.out, "}}")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            for encoding in &self.generator.config.encodings {
                self.output_class_serialize_for_encoding(*encoding)?;
                self.output_class_deserialize_for_encoding(name, *encoding)?;
            }
        }
        self.out.unindent();
        self.out.unindent();

        writeln!(self.out, "}}\n")?;

        self.leave_class();
        Ok(())
    }

    fn output_enum_class_container(
        &mut self,
        name: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(
            self.out,
            "abstract class {} {{",
            self.quote_qualified_name(name)
        )?;
        self.enter_class(name);
        writeln!(self.out, "const {}();", self.quote_qualified_name(name))?;

        if self.generator.config.serialization {
            writeln!(self.out, "\nvoid serialize(BinarySerializer serializer);")?;
            write!(
                self.out,
                "\nstatic {} deserialize(BinaryDeserializer deserializer) {{",
                self.quote_qualified_name(name)
            )?;
            self.out.indent();
            writeln!(
                self.out,
                r#"
int index = deserializer.deserializeVariantIndex();
switch (index) {{"#,
            )?;
            self.out.indent();
            for (index, variant) in variants {
                writeln!(
                    self.out,
                    "case {}: return {}{}.load(deserializer);",
                    index,
                    self.quote_qualified_name(name).to_camel_case(),
                    self.quote_field(&variant.name),
                )?;
            }
            writeln!(
                self.out,
                "default: throw Exception('Unknown variant index for {}: ' + index.toString());",
                self.quote_qualified_name(name),
            )?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            for encoding in &self.generator.config.encodings {
                self.output_class_serialize_for_encoding(*encoding)?;
                self.output_class_deserialize_for_encoding(name, *encoding)?;
            }
        }
        self.out.unindent();
        self.out.unindent();

        writeln!(self.out, "}}\n")?;

        self.output_variants(name, variants)?;
        self.leave_class();
        Ok(())
    }

    fn output_variants(
        &mut self,
        base: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        for (index, variant) in variants {
            self.output_variant(
                base,
                *index,
                &format!("{}{}", base, &variant.name),
                &variant.value,
            )?;
        }
        Ok(())
    }

    fn output_variant(
        &mut self,
        base: &str,
        index: u32,
        name: &str,
        variant: &VariantFormat,
    ) -> Result<()> {
        use VariantFormat::*;
        let fields = match variant {
            Unit => Vec::new(),
            NewType(format) => vec![Named {
                name: "value".to_string(),
                value: format.as_ref().clone(),
            }],
            Tuple(formats) => formats
                .iter()
                .enumerate()
                .map(|(i, f)| Named {
                    name: format!("field{}", i),
                    value: f.clone(),
                })
                .collect(),
            Struct(fields) => fields.clone(),
            Variable(_) => panic!("incorrect value"),
        };
        self.output_struct_or_variant_container(
            Some(&self.quote_qualified_name(base)),
            Some(index),
            name,
            &fields,
        )
    }

    fn output_comment(&mut self, name: &str) -> std::io::Result<()> {
        let mut path = self.current_namespace.clone();
        path.push(name.to_string());
        if let Some(doc) = self.generator.config.comments.get(&path) {
            let text = textwrap::indent(doc, "/// ").replace("\n\n", "\n///\n");
            write!(self.out, "{}", text)?;
        }
        Ok(())
    }
}

/// Installer for generated source files in Dart.
pub struct Installer {
    install_dir: PathBuf,
}

impl Installer {
    pub fn new(install_dir: PathBuf) -> Self {
        Installer { install_dir }
    }

    fn install_runtime(
        &self,
        source_dir: include_dir::Dir,
        path: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir_path = self.install_dir.join(path);
        std::fs::create_dir_all(&dir_path)?;
        for entry in source_dir.files() {
            let mut file = std::fs::File::create(dir_path.join(entry.path()))?;
            file.write_all(entry.contents())?;
        }
        Ok(())
    }

    fn write_package(&self, install_dir: &Path, module_name: &str) -> Result<()> {
        let mut file = std::fs::File::create(install_dir.join("pubspec.yaml"))?;
        let mut out = IndentedWriter::new(&mut file, IndentConfig::Space(2));
        writeln!(
            &mut out,
            r#"name: {}

environment:
  sdk: '>=3.0.0 <4.0.0'

dependencies:
  meta: ^1.0.0
  tuple: ^2.0.0
"#,
            module_name
        )?;
        Ok(())
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_module(
        &self,
        config: &CodeGeneratorConfig,
        registry: &Registry,
    ) -> std::result::Result<(), Self::Error> {
        let generator = CodeGenerator::new(config);
        generator.output(self.install_dir.clone(), registry)?;

        // Write the `pubspec.yaml` package manifest file.
        if config.package_manifest {
            self.write_package(&self.install_dir, &config.module_name)?;
        }

        // Write the main module file to export the public API.
        std::fs::write(
            self.install_dir
                .join("lib")
                .join(format!("{}.dart", &config.module_name)),
            format!(
                "export 'src/{name}/{name}.dart';",
                name = &config.module_name
            ),
        )?;

        Ok(())
    }

    fn install_serde_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(include_directory!("runtime/dart/serde"), "lib/src/serde")
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(
            include_directory!("runtime/dart/bincode"),
            "lib/src/bincode",
        )
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(include_directory!("runtime/dart/bcs"), "lib/src/bcs")
    }
}

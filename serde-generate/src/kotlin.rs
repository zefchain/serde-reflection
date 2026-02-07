// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    common,
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig, Encoding,
};
use heck::CamelCase;
use include_dir::include_dir as include_directory;
use serde_reflection::{ContainerFormat, Format, FormatHolder, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap, HashMap},
    io::{Result, Write},
    path::PathBuf,
};

/// Main configuration object for code-generation in Kotlin.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
    /// Mapping from external type names to fully-qualified class names.
    /// Derived from `config.external_definitions`.
    external_qualified_names: HashMap<String, String>,
}

/// Shared state for the code generation of a Kotlin source file.
struct KotlinEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
    /// Current namespace (e.g. vec!["com", "my_org", "my_package", "MyClass"])
    current_namespace: Vec<String>,
}

impl<'a> CodeGenerator<'a> {
    /// Create a Kotlin code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.enums.c_style {
            panic!("Kotlin does not support generating c-style enums");
        }
        let mut external_qualified_names = HashMap::new();
        for (namespace, names) in &config.external_definitions {
            for name in names {
                external_qualified_names
                    .insert(name.to_string(), format!("{}.{}", namespace, name));
            }
        }
        Self {
            config,
            external_qualified_names,
        }
    }

    /// Output class definitions for `registry` in separate source files.
    /// Source files will be created in a subdirectory of `install_dir` corresponding to the
    /// package name (if any, otherwise `install_dir` itself).
    pub fn write_source_files(
        &self,
        install_dir: std::path::PathBuf,
        registry: &Registry,
    ) -> Result<()> {
        let current_namespace = self
            .config
            .module_name
            .split('.')
            .map(String::from)
            .collect::<Vec<_>>();

        let mut dir_path = install_dir;
        for part in &current_namespace {
            dir_path = dir_path.join(part);
        }
        std::fs::create_dir_all(&dir_path)?;

        for (name, format) in registry {
            self.write_container_class(&dir_path, current_namespace.clone(), name, format)?;
        }
        if self.config.serialization {
            self.write_helper_class(&dir_path, current_namespace, registry)?;
        }
        Ok(())
    }

    fn write_container_class(
        &self,
        dir_path: &std::path::Path,
        current_namespace: Vec<String>,
        name: &str,
        format: &ContainerFormat,
    ) -> Result<()> {
        let mut file = std::fs::File::create(dir_path.join(name.to_string() + ".kt"))?;
        let mut emitter = KotlinEmitter {
            out: IndentedWriter::new(&mut file, IndentConfig::Space(4)),
            generator: self,
            current_namespace,
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
        let mut file = std::fs::File::create(dir_path.join("TraitHelpers.kt"))?;
        let mut emitter = KotlinEmitter {
            out: IndentedWriter::new(&mut file, IndentConfig::Space(4)),
            generator: self,
            current_namespace,
        };

        emitter.output_preamble()?;
        emitter.output_trait_helpers(registry)
    }
}

impl<'a, T> KotlinEmitter<'a, T>
where
    T: Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        writeln!(self.out, "package {}\n", self.generator.config.module_name)?;
        Ok(())
    }

    /// Compute a reference to the registry type `name`.
    fn quote_qualified_name(&self, name: &str) -> String {
        self.generator
            .external_qualified_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("{}.{}", self.generator.config.module_name, name))
    }

    fn output_comment(&mut self, name: &str) -> std::io::Result<()> {
        let mut path = self.current_namespace.clone();
        path.push(name.to_string());
        if let Some(doc) = self.generator.config.comments.get(&path) {
            let text = textwrap::indent(doc, "// ").replace("\n\n", "\n//\n");
            write!(self.out, "{}", text)?;
        }
        Ok(())
    }

    fn output_custom_code(&mut self) -> std::io::Result<()> {
        if let Some(code) = self
            .generator
            .config
            .custom_code
            .get(&self.current_namespace)
        {
            writeln!(self.out, "\n{}", code)?;
        }
        Ok(())
    }

    fn quote_type(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(x) => self.quote_qualified_name(x),
            Unit => "Unit".into(),
            Bool => "Boolean".into(),
            I8 => "Byte".into(),
            I16 => "Short".into(),
            I32 => "Int".into(),
            I64 => "Long".into(),
            I128 => "com.novi.serde.Int128".into(),
            U8 => "UByte".into(),
            U16 => "UShort".into(),
            U32 => "UInt".into(),
            U64 => "ULong".into(),
            U128 => "com.novi.serde.UInt128".into(),
            F32 => "Float".into(),
            F64 => "Double".into(),
            Char => "Char".into(),
            Str => "String".into(),
            Bytes => "com.novi.serde.Bytes".into(),

            Option(format) => {
                let inner = self.quote_type(format);
                if inner.ends_with('?') {
                    inner
                } else {
                    format!("{}?", inner)
                }
            }
            Seq(format) => format!("kotlin.collections.List<{}>", self.quote_type(format)),
            Map { key, value } => format!(
                "kotlin.collections.Map<{}, {}>",
                self.quote_type(key),
                self.quote_type(value)
            ),
            Tuple(formats) => match formats.len() {
                2 => format!(
                    "Pair<{}, {}>",
                    self.quote_type(&formats[0]),
                    self.quote_type(&formats[1])
                ),
                3 => format!(
                    "Triple<{}, {}, {}>",
                    self.quote_type(&formats[0]),
                    self.quote_type(&formats[1]),
                    self.quote_type(&formats[2])
                ),
                _ => format!(
                    "com.novi.serde.Tuple{}<{}>",
                    formats.len(),
                    self.quote_types(formats)
                ),
            },
            TupleArray { content, size: _ } => {
                format!("kotlin.collections.List<{}>", self.quote_type(content))
            }

            Variable(_) => panic!("unexpected value"),
        }
    }

    fn quote_types<'b, I>(&'b self, formats: I) -> String
    where
        I: IntoIterator<Item = &'b Format>,
    {
        formats
            .into_iter()
            .map(|format| self.quote_type(format))
            .collect::<Vec<_>>()
            .join(", ")
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
        writeln!(self.out, "object TraitHelpers {{")?;
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

    fn quote_serialize_value(&self, value: &str, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(_) => format!("{}.serialize(serializer)", value),
            Unit => format!("serializer.serialize_unit({})", value),
            Bool => format!("serializer.serialize_bool({})", value),
            I8 => format!("serializer.serialize_i8({})", value),
            I16 => format!("serializer.serialize_i16({})", value),
            I32 => format!("serializer.serialize_i32({})", value),
            I64 => format!("serializer.serialize_i64({})", value),
            I128 => format!("serializer.serialize_i128({})", value),
            U8 => format!("serializer.serialize_u8({})", value),
            U16 => format!("serializer.serialize_u16({})", value),
            U32 => format!("serializer.serialize_u32({})", value),
            U64 => format!("serializer.serialize_u64({})", value),
            U128 => format!("serializer.serialize_u128({})", value),
            F32 => format!("serializer.serialize_f32({})", value),
            F64 => format!("serializer.serialize_f64({})", value),
            Char => format!("serializer.serialize_char({})", value),
            Str => format!("serializer.serialize_str({})", value),
            Bytes => format!("serializer.serialize_bytes({})", value),
            _ => format!(
                "TraitHelpers.serialize_{}({}, serializer)",
                common::mangle_type(format),
                value
            ),
        }
    }

    fn quote_deserialize(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(name) => format!(
                "{}.deserialize(deserializer)",
                self.quote_qualified_name(name)
            ),
            Unit => "deserializer.deserialize_unit()".to_string(),
            Bool => "deserializer.deserialize_bool()".to_string(),
            I8 => "deserializer.deserialize_i8()".to_string(),
            I16 => "deserializer.deserialize_i16()".to_string(),
            I32 => "deserializer.deserialize_i32()".to_string(),
            I64 => "deserializer.deserialize_i64()".to_string(),
            I128 => "deserializer.deserialize_i128()".to_string(),
            U8 => "deserializer.deserialize_u8()".to_string(),
            U16 => "deserializer.deserialize_u16()".to_string(),
            U32 => "deserializer.deserialize_u32()".to_string(),
            U64 => "deserializer.deserialize_u64()".to_string(),
            U128 => "deserializer.deserialize_u128()".to_string(),
            F32 => "deserializer.deserialize_f32()".to_string(),
            F64 => "deserializer.deserialize_f64()".to_string(),
            Char => "deserializer.deserialize_char()".to_string(),
            Str => "deserializer.deserialize_str()".to_string(),
            Bytes => "deserializer.deserialize_bytes()".to_string(),
            _ => format!(
                "TraitHelpers.deserialize_{}(deserializer)",
                common::mangle_type(format),
            ),
        }
    }

    fn output_serialization_helper(&mut self, name: &str, format0: &Format) -> Result<()> {
        use Format::*;

        write!(
            self.out,
            "@Throws(com.novi.serde.SerializationError::class)\nfun serialize_{}(value: {}, serializer: com.novi.serde.Serializer) {{",
            name,
            self.quote_type(format0)
        )?;
        self.out.indent();
        match format0 {
            Option(format) => {
                write!(
                    self.out,
                    r#"
if (value == null) {{
    serializer.serialize_option_tag(false)
}} else {{
    serializer.serialize_option_tag(true)
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
serializer.serialize_len(value.size.toLong())
for (item in value) {{
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
serializer.serialize_len(value.size.toLong())
val offsets = IntArray(value.size)
var count = 0
for (entry in value.entries) {{
    offsets[count++] = serializer.get_buffer_offset()
    {}
    {}
}}
serializer.sort_map_entries(offsets)
"#,
                    self.quote_serialize_value("entry.key", key),
                    self.quote_serialize_value("entry.value", value)
                )?;
            }

            Tuple(formats) => {
                writeln!(self.out)?;
                for (index, format) in formats.iter().enumerate() {
                    let expr = match formats.len() {
                        2 => match index {
                            0 => "value.first".to_string(),
                            1 => "value.second".to_string(),
                            _ => unreachable!(),
                        },
                        3 => match index {
                            0 => "value.first".to_string(),
                            1 => "value.second".to_string(),
                            2 => "value.third".to_string(),
                            _ => unreachable!(),
                        },
                        _ => format!("value.field{}", index),
                    };
                    writeln!(self.out, "{}", self.quote_serialize_value(&expr, format))?;
                }
            }

            TupleArray { content, size } => {
                write!(
                    self.out,
                    r#"
if (value.size != {0}) {{
    throw IllegalArgumentException("Invalid length for fixed-size array: " + value.size + " instead of " + {0})
}}
for (item in value) {{
    {1}
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
            "@Throws(com.novi.serde.DeserializationError::class)\nfun deserialize_{}(deserializer: com.novi.serde.Deserializer): {} {{",
            name,
            self.quote_type(format0),
        )?;
        self.out.indent();
        match format0 {
            Option(format) => {
                write!(
                    self.out,
                    r#"
val tag = deserializer.deserialize_option_tag()
return if (!tag) {{
    null
}} else {{
    {}
}}
"#,
                    self.quote_deserialize(format),
                )?;
            }

            Seq(format) => {
                write!(
                    self.out,
                    r#"
val length = deserializer.deserialize_len()
val obj = ArrayList<{0}>(length.toInt())
var i = 0L
while (i < length) {{
    obj.add({1})
    i += 1
}}
return obj
"#,
                    self.quote_type(format),
                    self.quote_deserialize(format)
                )?;
            }

            Map { key, value } => {
                write!(
                    self.out,
                    r#"
val length = deserializer.deserialize_len()
val obj = HashMap<{0}, {1}>()
var previousKeyStart = 0
var previousKeyEnd = 0
var i = 0L
while (i < length) {{
    val keyStart = deserializer.get_buffer_offset()
    val key = {2}
    val keyEnd = deserializer.get_buffer_offset()
    if (i > 0) {{
        deserializer.check_that_key_slices_are_increasing(
            com.novi.serde.Slice(previousKeyStart, previousKeyEnd),
            com.novi.serde.Slice(keyStart, keyEnd))
    }}
    previousKeyStart = keyStart
    previousKeyEnd = keyEnd
    val value = {3}
    obj[key] = value
    i += 1
}}
return obj
"#,
                    self.quote_type(key),
                    self.quote_type(value),
                    self.quote_deserialize(key),
                    self.quote_deserialize(value),
                )?;
            }

            Tuple(formats) => {
                let constructor = match formats.len() {
                    2 => "Pair".to_string(),
                    3 => "Triple".to_string(),
                    _ => self.quote_type(format0),
                };
                write!(
                    self.out,
                    r#"
return {0}({1}
)
"#,
                    constructor,
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
val obj = ArrayList<{0}>({1})
for (i in 0 until {1}) {{
    obj.add({2})
}}
return obj
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
        self.output_struct_or_variant_container(Some(base), Some(index), name, &fields)
    }

    fn output_variants(
        &mut self,
        base: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        for (index, variant) in variants {
            self.output_variant(base, *index, &variant.name, &variant.value)?;
        }
        Ok(())
    }

    fn output_fields_in_constructor(
        &mut self,
        class_name: &str,
        fields: &[Named<Format>],
    ) -> Result<()> {
        self.out.indent();
        let mut base_path = self.current_namespace.clone();
        base_path.push(class_name.to_string());
        for (index, field) in fields.iter().enumerate() {
            let mut path = base_path.clone();
            path.push(field.name.to_string());
            if let Some(doc) = self.generator.config.comments.get(&path) {
                let text = textwrap::indent(doc, "// ").replace("\n\n", "\n//\n");
                write!(self.out, "{}", text)?;
            }
            let separator = if index + 1 == fields.len() { "" } else { "," };
            writeln!(
                self.out,
                "val {}: {}{}",
                field.name,
                self.quote_type(&field.value),
                separator
            )?;
        }
        self.out.unindent();
        Ok(())
    }

    fn output_struct_or_variant_container(
        &mut self,
        variant_base: Option<&str>,
        variant_index: Option<u32>,
        name: &str,
        fields: &[Named<Format>],
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        match (variant_base, fields.is_empty()) {
            (Some(base), true) => {
                writeln!(self.out, "object {} : {}() {{", name, base)?;
            }
            (Some(base), false) => {
                writeln!(self.out, "data class {}(", name)?;
                self.output_fields_in_constructor(name, fields)?;
                writeln!(self.out, ") : {}() {{", base)?;
            }
            (None, true) => {
                writeln!(self.out, "class {} {{", name)?;
            }
            (None, false) => {
                writeln!(self.out, "data class {}(", name)?;
                self.output_fields_in_constructor(name, fields)?;
                writeln!(self.out, ") {{")?;
            }
        }
        self.enter_class(name);

        if self.generator.config.serialization {
            let prefix = if variant_index.is_some() {
                "override "
            } else {
                ""
            };
            writeln!(
                self.out,
                "\n@Throws(com.novi.serde.SerializationError::class)\n{}fun serialize(serializer: com.novi.serde.Serializer) {{",
                prefix
            )?;
            self.out.indent();
            writeln!(self.out, "serializer.increase_container_depth()")?;
            if let Some(index) = variant_index {
                writeln!(self.out, "serializer.serialize_variant_index({})", index)?;
            }
            for field in fields {
                writeln!(
                    self.out,
                    "{}",
                    self.quote_serialize_value(&format!("this.{}", field.name), &field.value)
                )?;
            }
            writeln!(self.out, "serializer.decrease_container_depth()")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            if variant_index.is_none() {
                for encoding in &self.generator.config.encodings {
                    self.output_class_serialize_for_encoding(*encoding)?;
                }
            }
        }

        if self.generator.config.serialization {
            if variant_index.is_some() {
                if fields.is_empty() {
                    writeln!(
                        self.out,
                        "\n@Throws(com.novi.serde.DeserializationError::class)\nfun load(deserializer: com.novi.serde.Deserializer): {} {{",
                        name
                    )?;
                    self.out.indent();
                    writeln!(self.out, "deserializer.increase_container_depth()")?;
                    writeln!(self.out, "deserializer.decrease_container_depth()")?;
                    writeln!(self.out, "return {}", name)?;
                    self.out.unindent();
                    writeln!(self.out, "}}")?;
                } else {
                    writeln!(self.out, "\ncompanion object {{")?;
                    self.out.indent();
                    writeln!(
                        self.out,
                        "@Throws(com.novi.serde.DeserializationError::class)\nfun load(deserializer: com.novi.serde.Deserializer): {} {{",
                        name
                    )?;
                    self.out.indent();
                    writeln!(self.out, "deserializer.increase_container_depth()")?;
                    for field in fields {
                        writeln!(
                            self.out,
                            "val {} = {}",
                            field.name,
                            self.quote_deserialize(&field.value)
                        )?;
                    }
                    writeln!(self.out, "deserializer.decrease_container_depth()")?;
                    let result = format!(
                        "{}({})",
                        name,
                        fields
                            .iter()
                            .map(|f| f.name.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    writeln!(self.out, "return {}", result)?;
                    self.out.unindent();
                    writeln!(self.out, "}}")?;
                    self.out.unindent();
                    writeln!(self.out, "}}")?;
                }
            } else {
                writeln!(self.out, "\ncompanion object {{")?;
                self.out.indent();
                writeln!(
                    self.out,
                    "@Throws(com.novi.serde.DeserializationError::class)\nfun deserialize(deserializer: com.novi.serde.Deserializer): {} {{",
                    name
                )?;
                self.out.indent();
                writeln!(self.out, "deserializer.increase_container_depth()")?;
                for field in fields {
                    writeln!(
                        self.out,
                        "val {} = {}",
                        field.name,
                        self.quote_deserialize(&field.value)
                    )?;
                }
                writeln!(self.out, "deserializer.decrease_container_depth()")?;
                let result = if fields.is_empty() {
                    format!("{}()", name)
                } else {
                    format!(
                        "{}({})",
                        name,
                        fields
                            .iter()
                            .map(|f| f.name.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                writeln!(self.out, "return {}", result)?;
                self.out.unindent();
                writeln!(self.out, "}}")?;

                for encoding in &self.generator.config.encodings {
                    self.output_class_deserialize_for_encoding(name, *encoding)?;
                }
                self.out.unindent();
                writeln!(self.out, "}}")?;
            }
        }

        if variant_base.is_none() && fields.is_empty() {
            writeln!(
                self.out,
                r#"
override fun equals(other: Any?): Boolean {{
    return other is {0}
}}

override fun hashCode(): Int {{
    return 7
}}"#,
                name
            )?;
        }

        self.output_custom_code()?;
        self.leave_class();
        writeln!(self.out, "}}")
    }

    fn output_enum_container(
        &mut self,
        name: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "sealed class {} {{", name)?;
        self.enter_class(name);
        if self.generator.config.serialization {
            writeln!(
                self.out,
                "@Throws(com.novi.serde.SerializationError::class)\nabstract fun serialize(serializer: com.novi.serde.Serializer)"
            )?;
            writeln!(self.out, "\ncompanion object {{")?;
            self.out.indent();
            writeln!(
                self.out,
                "@Throws(com.novi.serde.DeserializationError::class)\nfun deserialize(deserializer: com.novi.serde.Deserializer): {} {{",
                name
            )?;
            self.out.indent();
            writeln!(
                self.out,
                "val index = deserializer.deserialize_variant_index()"
            )?;
            writeln!(self.out, "return when (index) {{")?;
            self.out.indent();
            for (index, variant) in variants {
                writeln!(self.out, "{} -> {}.load(deserializer)", index, variant.name,)?;
            }
            writeln!(
                self.out,
                "else -> throw com.novi.serde.DeserializationError(\"Unknown variant index for {}: \" + index)",
                name
            )?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;
            for encoding in &self.generator.config.encodings {
                self.output_class_deserialize_for_encoding(name, *encoding)?;
            }
            self.out.unindent();
            writeln!(self.out, "}}")?;

            for encoding in &self.generator.config.encodings {
                self.output_class_serialize_for_encoding(*encoding)?;
            }
        }

        self.output_variants(name, variants)?;
        self.output_custom_code()?;
        self.leave_class();
        writeln!(self.out, "}}\n")
    }

    fn output_class_serialize_for_encoding(&mut self, encoding: Encoding) -> Result<()> {
        writeln!(
            self.out,
            r#"
@Throws(com.novi.serde.SerializationError::class)
fun {0}Serialize(): ByteArray {{
    val serializer = com.novi.{0}.{1}Serializer()
    serialize(serializer)
    return serializer.get_bytes()
}}"#,
            encoding.name(),
            encoding.name().to_camel_case()
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
@Throws(com.novi.serde.DeserializationError::class)
fun {1}Deserialize(input: ByteArray): {0} {{
    val deserializer = com.novi.{1}.{2}Deserializer(input)
    val value = deserialize(deserializer)
    if (deserializer.get_buffer_offset() < input.size) {{
        throw com.novi.serde.DeserializationError("Some input bytes were not read")
    }}
    return value
}}"#,
            name,
            encoding.name(),
            encoding.name().to_camel_case()
        )
    }

    fn output_container(&mut self, name: &str, format: &ContainerFormat) -> Result<()> {
        use ContainerFormat::*;
        let fields = match format {
            UnitStruct => Vec::new(),
            NewTypeStruct(format) => vec![Named {
                name: "value".to_string(),
                value: format.as_ref().clone(),
            }],
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
                self.output_enum_container(name, variants)?;
                return Ok(());
            }
        };
        self.output_struct_or_variant_container(None, None, name, &fields)
    }
}

/// Installer for generated source files in Kotlin.
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
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_module(
        &self,
        config: &CodeGeneratorConfig,
        registry: &Registry,
    ) -> std::result::Result<(), Self::Error> {
        let generator = CodeGenerator::new(config);
        generator.write_source_files(self.install_dir.clone(), registry)?;
        Ok(())
    }

    fn install_serde_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(
            include_directory!("runtime/kotlin/com/novi/serde"),
            "com/novi/serde",
        )
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(
            include_directory!("runtime/kotlin/com/novi/bincode"),
            "com/novi/bincode",
        )
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(
            include_directory!("runtime/kotlin/com/novi/bcs"),
            "com/novi/bcs",
        )
    }
}

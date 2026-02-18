// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    common,
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig, Encoding,
};
use heck::CamelCase;
use serde_reflection::{ContainerFormat, Format, FormatHolder, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap, HashMap},
    io::{Result, Write},
    path::PathBuf,
};

/// Main configuration object for code-generation in Go.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
    /// Module path where to find the serde runtime packages (serde, bcs, bincode).
    /// Default: "github.com/novifinancial/serde-reflection/serde-generate/runtime/golang".
    serde_module_path: String,
    /// Mapping from external type names to fully-qualified class names (e.g. "MyClass" -> "com.my_org.my_package.MyClass").
    /// Derived from `config.external_definitions`.
    external_qualified_names: HashMap<String, String>,
}

/// Shared state for the code generation of a Go source file.
struct GoEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
    /// Current namespace (e.g. vec!["com", "my_org", "my_package", "MyClass"])
    current_namespace: Vec<String>,
}

impl<'a> CodeGenerator<'a> {
    /// Create a Go code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.enums.c_style {
            panic!("Go does not support generating c-style enums");
        }
        let mut external_qualified_names = HashMap::new();
        for (namespace, names) in &config.external_definitions {
            let package_name = {
                let path = namespace.rsplitn(2, '/').collect::<Vec<_>>();
                if path.len() <= 1 {
                    namespace
                } else {
                    path[0]
                }
            };
            for name in names {
                external_qualified_names.insert(name.to_string(), format!("{package_name}.{name}"));
            }
        }
        Self {
            config,
            serde_module_path:
                "github.com/novifinancial/serde-reflection/serde-generate/runtime/golang"
                    .to_string(),
            external_qualified_names,
        }
    }

    /// Whether the package providing Serde definitions is located within a different module.
    pub fn with_serde_module_path(mut self, serde_module_path: String) -> Self {
        self.serde_module_path = serde_module_path;
        self
    }

    /// Output class definitions for `registry`.
    pub fn output(&self, out: &mut dyn Write, registry: &Registry) -> Result<()> {
        let current_namespace = self
            .config
            .module_name
            .split('.')
            .map(String::from)
            .collect::<Vec<_>>();

        let mut emitter = GoEmitter {
            // `go fmt` indents using tabs so let's do the same.
            out: IndentedWriter::new(out, IndentConfig::Tab),
            generator: self,
            current_namespace,
        };

        emitter.output_preamble(registry)?;

        for (name, format) in registry {
            emitter.output_container(name, format)?;
        }

        if self.config.serialization {
            emitter.output_trait_helpers(registry)?;
        }

        Ok(())
    }
}

impl<'a, T> GoEmitter<'a, T>
where
    T: Write,
{
    fn output_preamble(&mut self, registry: &Registry) -> Result<()> {
        writeln!(
            self.out,
            "package {}\n\n",
            self.generator.config.module_name
        )?;
        // Go does not support disabling warnings on unused imports.
        if registry.is_empty() {
            return Ok(());
        }
        writeln!(self.out, "import (")?;
        self.out.indent();
        if self.generator.config.serialization
            && (Self::has_enum(registry) || !self.generator.config.encodings.is_empty())
        {
            writeln!(self.out, "\"fmt\"")?;
        }
        if self.generator.config.serialization || Self::has_int128(registry) {
            writeln!(self.out, "\"{}/serde\"", self.generator.serde_module_path)?;
        }
        if self.generator.config.serialization {
            for encoding in &self.generator.config.encodings {
                writeln!(
                    self.out,
                    "\"{}/{}\"",
                    self.generator.serde_module_path,
                    encoding.name()
                )?;
            }
        }
        for path in self.generator.config.external_definitions.keys() {
            writeln!(self.out, "\"{path}\"")?;
        }
        self.out.unindent();
        writeln!(self.out, ")\n")?;
        Ok(())
    }

    fn has_int128(registry: &Registry) -> bool {
        for format in registry.values() {
            if format
                .visit(&mut |f| match f {
                    Format::I128 | Format::U128 => {
                        // Interrupt the visit if we find a (u)int128
                        Err(serde_reflection::Error::Custom(String::new()))
                    }
                    _ => Ok(()),
                })
                .is_err()
            {
                return true;
            }
        }
        false
    }

    fn has_enum(registry: &Registry) -> bool {
        for format in registry.values() {
            if let ContainerFormat::Enum(_) = format {
                return true;
            }
        }
        false
    }

    /// Compute a reference to the registry type `name`.
    fn quote_qualified_name(&self, name: &str) -> String {
        self.generator
            .external_qualified_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    fn output_comment(&mut self, name: &str) -> std::io::Result<()> {
        let mut path = self.current_namespace.clone();
        path.push(name.to_string());
        if let Some(doc) = self.generator.config.comments.get(&path) {
            let text = textwrap::indent(doc, "// ").replace("\n\n", "\n//\n");
            write!(self.out, "{text}")?;
        }
        Ok(())
    }

    fn output_custom_code(&mut self, name: &str) -> std::io::Result<()> {
        let mut path = self.current_namespace.clone();
        path.push(name.to_string());
        if let Some(code) = self.generator.config.custom_code.get(&path) {
            write!(self.out, "\n{code}")?;
        }
        Ok(())
    }

    fn quote_type(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(x) => self.quote_qualified_name(x),
            Unit => "struct {}".into(),
            Bool => "bool".into(),
            I8 => "int8".into(),
            I16 => "int16".into(),
            I32 => "int32".into(),
            I64 => "int64".into(),
            I128 => "serde.Int128".into(),
            U8 => "uint8".into(),
            U16 => "uint16".into(),
            U32 => "uint32".into(),
            U64 => "uint64".into(),
            U128 => "serde.Uint128".into(),
            F32 => "float32".into(),
            F64 => "float64".into(),
            Char => "rune".into(),
            Str => "string".into(),
            Bytes => "[]byte".into(),

            Option(format) => format!("*{}", self.quote_type(format)),
            Seq(format) => format!("[]{}", self.quote_type(format)),
            Map { key, value } => {
                format!("map[{}]{}", self.quote_type(key), self.quote_type(value))
            }
            Tuple(formats) => format!(
                "struct {{{}}}",
                formats
                    .iter()
                    .enumerate()
                    .map(|(index, format)| format!("Field{} {}", index, self.quote_type(format)))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            TupleArray { content, size } => format!("[{}]{}", size, self.quote_type(content)),

            Variable(_) => panic!("unexpected value"),
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
        for (mangled_name, subtype) in &subtypes {
            self.output_serialization_helper(mangled_name, subtype)?;
            self.output_deserialization_helper(mangled_name, subtype)?;
        }
        Ok(())
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
        let expr = match format {
            TypeName(_) => format!("{value}.Serialize(serializer)"),
            Unit => format!("serializer.SerializeUnit({value})"),
            Bool => format!("serializer.SerializeBool({value})"),
            I8 => format!("serializer.SerializeI8({value})"),
            I16 => format!("serializer.SerializeI16({value})"),
            I32 => format!("serializer.SerializeI32({value})"),
            I64 => format!("serializer.SerializeI64({value})"),
            I128 => format!("serializer.SerializeI128({value})"),
            U8 => format!("serializer.SerializeU8({value})"),
            U16 => format!("serializer.SerializeU16({value})"),
            U32 => format!("serializer.SerializeU32({value})"),
            U64 => format!("serializer.SerializeU64({value})"),
            U128 => format!("serializer.SerializeU128({value})"),
            F32 => format!("serializer.SerializeF32({value})"),
            F64 => format!("serializer.SerializeF64({value})"),
            Char => format!("serializer.SerializeChar({value})"),
            Str => format!("serializer.SerializeStr({value})"),
            Bytes => format!("serializer.SerializeBytes({value})"),
            _ => format!(
                "serialize_{}({}, serializer)",
                common::mangle_type(format),
                value
            ),
        };
        format!("if err := {expr}; err != nil {{ return err }}")
    }

    fn quote_deserialize(&self, format: &Format, dest: &str, fail: &str) -> String {
        use Format::*;
        let expr = match format {
            TypeName(name) => format!(
                "Deserialize{}(deserializer)",
                self.quote_qualified_name(name)
            ),
            Unit => "deserializer.DeserializeUnit()".to_string(),
            Bool => "deserializer.DeserializeBool()".to_string(),
            I8 => "deserializer.DeserializeI8()".to_string(),
            I16 => "deserializer.DeserializeI16()".to_string(),
            I32 => "deserializer.DeserializeI32()".to_string(),
            I64 => "deserializer.DeserializeI64()".to_string(),
            I128 => "deserializer.DeserializeI128()".to_string(),
            U8 => "deserializer.DeserializeU8()".to_string(),
            U16 => "deserializer.DeserializeU16()".to_string(),
            U32 => "deserializer.DeserializeU32()".to_string(),
            U64 => "deserializer.DeserializeU64()".to_string(),
            U128 => "deserializer.DeserializeU128()".to_string(),
            F32 => "deserializer.DeserializeF32()".to_string(),
            F64 => "deserializer.DeserializeF64()".to_string(),
            Char => "deserializer.DeserializeChar()".to_string(),
            Str => "deserializer.DeserializeStr()".to_string(),
            Bytes => "deserializer.DeserializeBytes()".to_string(),
            _ => format!("deserialize_{}(deserializer)", common::mangle_type(format)),
        };
        format!(
            "if val, err := {expr}; err == nil {{ {dest} = val }} else {{ return {fail}, err }}"
        )
    }

    fn output_serialization_helper(&mut self, name: &str, format0: &Format) -> Result<()> {
        use Format::*;

        write!(
            self.out,
            "func serialize_{}(value {}, serializer serde.Serializer) error {{",
            name,
            self.quote_type(format0)
        )?;
        self.out.indent();
        match format0 {
            Option(format) => {
                write!(
                    self.out,
                    r#"
if value != nil {{
	if err := serializer.SerializeOptionTag(true); err != nil {{ return err }}
	{}
}} else {{
	if err := serializer.SerializeOptionTag(false); err != nil {{ return err }}
}}
"#,
                    self.quote_serialize_value("(*value)", format)
                )?;
            }

            Seq(format) => {
                write!(
                    self.out,
                    r#"
if err := serializer.SerializeLen(uint64(len(value))); err != nil {{ return err }}
for _, item := range(value) {{
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
if err := serializer.SerializeLen(uint64(len(value))); err != nil {{ return err }}
offsets := make([]uint64, len(value))
count := 0
for k, v := range(value) {{
	offsets[count] = serializer.GetBufferOffset()
	count += 1
	{}
	{}
}}
serializer.SortMapEntries(offsets);
"#,
                    self.quote_serialize_value("k", key),
                    self.quote_serialize_value("v", value)
                )?;
            }

            Tuple(formats) => {
                writeln!(self.out)?;
                for (index, format) in formats.iter().enumerate() {
                    let expr = format!("value.Field{index}");
                    writeln!(self.out, "{}", self.quote_serialize_value(&expr, format))?;
                }
            }

            TupleArray { content, size: _ } => {
                write!(
                    self.out,
                    r#"
for _, item := range(value) {{
	{}
}}
"#,
                    self.quote_serialize_value("item", content),
                )?;
            }

            _ => panic!("unexpected case"),
        }
        writeln!(self.out, "return nil")?;
        self.out.unindent();
        writeln!(self.out, "}}\n")
    }

    fn output_deserialization_helper(&mut self, name: &str, format0: &Format) -> Result<()> {
        use Format::*;

        write!(
            self.out,
            "func deserialize_{}(deserializer serde.Deserializer) ({}, error) {{",
            name,
            self.quote_type(format0),
        )?;
        self.out.indent();
        match format0 {
            Option(format) => {
                write!(
                    self.out,
                    r#"
tag, err := deserializer.DeserializeOptionTag()
if err != nil {{ return nil, err }}
if tag {{
	value := new({})
	{}
        return value, nil
}} else {{
	return nil, nil
}}
"#,
                    self.quote_type(format),
                    self.quote_deserialize(format, "*value", "nil"),
                )?;
            }

            Seq(format) => {
                write!(
                    self.out,
                    r#"
length, err := deserializer.DeserializeLen()
if err != nil {{ return nil, err }}
obj := make([]{}, length)
for i := range(obj) {{
	{}
}}
return obj, nil
"#,
                    self.quote_type(format),
                    self.quote_deserialize(format, "obj[i]", "nil")
                )?;
            }

            Map { key, value } => {
                write!(
                    self.out,
                    r#"
length, err := deserializer.DeserializeLen()
if err != nil {{ return nil, err }}
obj := make(map[{0}]{1})
previous_slice := serde.Slice {{ 0, 0 }}
for i := 0; i < int(length); i++ {{
	var slice serde.Slice
	slice.Start = deserializer.GetBufferOffset()
	var key {0}
	{2}
	slice.End = deserializer.GetBufferOffset()
	if i > 0 {{
		err := deserializer.CheckThatKeySlicesAreIncreasing(previous_slice, slice)
		if err != nil {{ return nil, err }}
	}}
	previous_slice = slice
	{3}
}}
return obj, nil
"#,
                    self.quote_type(key),
                    self.quote_type(value),
                    self.quote_deserialize(key, "key", "nil"),
                    self.quote_deserialize(value, "obj[key]", "nil"),
                )?;
            }

            Tuple(formats) => {
                write!(
                    self.out,
                    r#"
var obj {}
{}
return obj, nil
"#,
                    self.quote_type(format0),
                    formats
                        .iter()
                        .enumerate()
                        .map(|(i, f)| self.quote_deserialize(f, &format!("obj.Field{i}"), "obj"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )?;
            }

            TupleArray { content, size } => {
                write!(
                    self.out,
                    r#"
var obj [{1}]{0}
for i := range(obj) {{
	{2}
}}
return obj, nil
"#,
                    self.quote_type(content),
                    size,
                    self.quote_deserialize(content, "obj[i]", "obj")
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
            NewType(format) => match format.as_ref() {
                // We cannot define a "new type" (e.g. `type Foo Bar`) out of a typename `Bar` because `Bar`
                // could point to a Go interface. This would make `Foo` an interface as well. Interfaces can't be used
                // as structs (e.g. they cannot have methods).
                //
                // Similarly, option types are compiled as pointers but `type Foo *Bar` would prevent `Foo` from being a
                // valid pointer receiver.
                Format::TypeName(_) | Format::Option(_) => vec![Named {
                    name: "Value".to_string(),
                    value: format.as_ref().clone(),
                }],
                // Other cases are fine.
                _ => {
                    self.output_struct_or_variant_new_type_container(
                        Some(base),
                        Some(index),
                        name,
                        format,
                    )?;
                    return Ok(());
                }
            },
            Tuple(formats) => formats
                .iter()
                .enumerate()
                .map(|(i, f)| Named {
                    name: format!("Field{i}"),
                    value: f.clone(),
                })
                .collect(),
            Struct(fields) => fields
                .iter()
                .map(|f| Named {
                    name: f.name.to_camel_case(),
                    value: f.value.clone(),
                })
                .collect(),
            Variable(_) => panic!("incorrect value"),
        };
        self.output_struct_or_variant_container(Some(base), Some(index), name, &fields)
    }

    fn output_struct_or_variant_container(
        &mut self,
        variant_base: Option<&str>,
        variant_index: Option<u32>,
        name: &str,
        fields: &[Named<Format>],
    ) -> Result<()> {
        let full_name = match variant_base {
            None => name.to_string(),
            Some(base) => format!("{base}__{name}"),
        };
        // Struct
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "type {full_name} struct {{")?;
        self.enter_class(name);
        for field in fields {
            self.output_comment(&field.name)?;
            writeln!(self.out, "{} {}", field.name, self.quote_type(&field.value))?;
        }
        self.leave_class();
        writeln!(self.out, "}}")?;

        // Link to base interface.
        if let Some(base) = variant_base {
            writeln!(self.out, "\nfunc (*{full_name}) is{base}() {{}}")?;
        }

        // Serialize
        if self.generator.config.serialization {
            writeln!(
                self.out,
                "\nfunc (obj *{full_name}) Serialize(serializer serde.Serializer) error {{"
            )?;
            self.out.indent();
            writeln!(
                self.out,
                "if err := serializer.IncreaseContainerDepth(); err != nil {{ return err }}"
            )?;
            if let Some(index) = variant_index {
                writeln!(self.out, "serializer.SerializeVariantIndex({index})")?;
            }
            for field in fields {
                writeln!(
                    self.out,
                    "{}",
                    self.quote_serialize_value(&format!("obj.{}", &field.name), &field.value)
                )?;
            }
            writeln!(self.out, "serializer.DecreaseContainerDepth()")?;
            writeln!(self.out, "return nil")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            for encoding in &self.generator.config.encodings {
                self.output_struct_serialize_for_encoding(&full_name, *encoding)?;
            }
        }
        // Deserialize (struct) or Load (variant)
        if self.generator.config.serialization {
            writeln!(
                self.out,
                "\nfunc {0}{1}(deserializer serde.Deserializer) ({1}, error) {{",
                if variant_base.is_none() {
                    "Deserialize"
                } else {
                    "load_"
                },
                full_name,
            )?;
            self.out.indent();
            writeln!(self.out, "var obj {full_name}")?;
            writeln!(
                self.out,
                "if err := deserializer.IncreaseContainerDepth(); err != nil {{ return obj, err }}"
            )?;
            for field in fields {
                writeln!(
                    self.out,
                    "{}",
                    self.quote_deserialize(&field.value, &format!("obj.{}", field.name), "obj")
                )?;
            }
            writeln!(self.out, "deserializer.DecreaseContainerDepth()")?;
            writeln!(self.out, "return obj, nil")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            if variant_base.is_none() {
                for encoding in &self.generator.config.encodings {
                    self.output_struct_deserialize_for_encoding(&full_name, *encoding)?;
                }
            }
        }
        // Custom code
        self.output_custom_code(name)?;
        Ok(())
    }

    // Same as output_struct_or_variant_container but we map the container with a single anonymous field
    // to a new type in Go.
    fn output_struct_or_variant_new_type_container(
        &mut self,
        variant_base: Option<&str>,
        variant_index: Option<u32>,
        name: &str,
        format: &Format,
    ) -> Result<()> {
        let full_name = match variant_base {
            None => name.to_string(),
            Some(base) => format!("{base}__{name}"),
        };
        // Struct
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "type {} {}", full_name, self.quote_type(format))?;

        // Link to base interface.
        if let Some(base) = variant_base {
            writeln!(self.out, "\nfunc (*{full_name}) is{base}() {{}}")?;
        }

        // Serialize
        if self.generator.config.serialization {
            writeln!(
                self.out,
                "\nfunc (obj *{full_name}) Serialize(serializer serde.Serializer) error {{"
            )?;
            self.out.indent();
            writeln!(
                self.out,
                "if err := serializer.IncreaseContainerDepth(); err != nil {{ return err }}"
            )?;
            if let Some(index) = variant_index {
                writeln!(self.out, "serializer.SerializeVariantIndex({index})")?;
            }
            writeln!(
                self.out,
                "{}",
                self.quote_serialize_value(
                    &format!("(({})(*obj))", self.quote_type(format)),
                    format
                )
            )?;
            writeln!(self.out, "serializer.DecreaseContainerDepth()")?;
            writeln!(self.out, "return nil")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            for encoding in &self.generator.config.encodings {
                self.output_struct_serialize_for_encoding(&full_name, *encoding)?;
            }
        }
        // Deserialize (struct) or Load (variant)
        if self.generator.config.serialization {
            writeln!(
                self.out,
                "\nfunc {0}{1}(deserializer serde.Deserializer) ({1}, error) {{",
                if variant_base.is_none() {
                    "Deserialize"
                } else {
                    "load_"
                },
                full_name,
            )?;
            self.out.indent();
            writeln!(self.out, "var obj {}", self.quote_type(format))?;
            writeln!(self.out, "if err := deserializer.IncreaseContainerDepth(); err != nil {{ return ({full_name})(obj), err }}")?;
            writeln!(
                self.out,
                "{}",
                self.quote_deserialize(format, "obj", &format!("(({full_name})(obj))"))
            )?;
            writeln!(self.out, "deserializer.DecreaseContainerDepth()")?;
            writeln!(self.out, "return ({full_name})(obj), nil")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            if variant_base.is_none() {
                for encoding in &self.generator.config.encodings {
                    self.output_struct_deserialize_for_encoding(&full_name, *encoding)?;
                }
            }
        }
        // Custom code
        self.output_custom_code(name)?;
        Ok(())
    }

    fn output_struct_serialize_for_encoding(
        &mut self,
        name: &str,
        encoding: Encoding,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
func (obj *{0}) {2}Serialize() ([]byte, error) {{
	if obj == nil {{
		return nil, fmt.Errorf("Cannot serialize null object")
	}}
	serializer := {1}.NewSerializer();
	if err := obj.Serialize(serializer); err != nil {{ return nil, err }}
	return serializer.GetBytes(), nil
}}"#,
            name,
            encoding.name(),
            encoding.name().to_camel_case()
        )
    }

    fn output_struct_deserialize_for_encoding(
        &mut self,
        name: &str,
        encoding: Encoding,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
func {2}Deserialize{0}(input []byte) ({0}, error) {{
	if input == nil {{
		var obj {0}
		return obj, fmt.Errorf("Cannot deserialize null array")
	}}
	deserializer := {1}.NewDeserializer(input);
	obj, err := Deserialize{0}(deserializer)
	if err == nil && deserializer.GetBufferOffset() < uint64(len(input)) {{
		return obj, fmt.Errorf("Some input bytes were not read")
	}}
	return obj, err
}}"#,
            name,
            encoding.name(),
            encoding.name().to_camel_case(),
        )
    }

    fn output_enum_container(
        &mut self,
        name: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "type {name} interface {{")?;
        self.current_namespace.push(name.to_string());
        self.out.indent();
        writeln!(self.out, "is{name}()")?;
        if self.generator.config.serialization {
            writeln!(self.out, "Serialize(serializer serde.Serializer) error")?;
            for encoding in &self.generator.config.encodings {
                writeln!(
                    self.out,
                    "{}Serialize() ([]byte, error)",
                    encoding.name().to_camel_case()
                )?;
            }
        }
        self.out.unindent();
        writeln!(self.out, "}}")?;

        if self.generator.config.serialization {
            write!(
                self.out,
                "\nfunc Deserialize{name}(deserializer serde.Deserializer) ({name}, error) {{"
            )?;
            self.out.indent();
            writeln!(
                self.out,
                r#"
index, err := deserializer.DeserializeVariantIndex()
if err != nil {{ return nil, err }}

switch index {{"#,
            )?;
            for (index, variant) in variants {
                writeln!(
                    self.out,
                    r#"case {}:
	if val, err := load_{}__{}(deserializer); err == nil {{
		return &val, nil
	}} else {{
		return nil, err
	}}
"#,
                    index, name, variant.name
                )?;
            }
            writeln!(
                self.out,
                "default:
	return nil, fmt.Errorf(\"Unknown variant index for {name}: %d\", index)",
            )?;
            writeln!(self.out, "}}")?;
            self.out.unindent();
            writeln!(self.out, "}}")?;

            for encoding in &self.generator.config.encodings {
                self.output_struct_deserialize_for_encoding(name, *encoding)?;
            }
        }

        for (index, variant) in variants {
            self.output_variant(name, *index, &variant.name, &variant.value)?;
        }
        self.current_namespace.pop();
        // Custom code
        self.output_custom_code(name)?;
        Ok(())
    }

    fn output_container(&mut self, name: &str, format: &ContainerFormat) -> Result<()> {
        use ContainerFormat::*;
        let fields = match format {
            UnitStruct => Vec::new(),
            NewTypeStruct(format) => match format.as_ref() {
                // See comment in `output_variant`.
                Format::TypeName(_) | Format::Option(_) => vec![Named {
                    name: "Value".to_string(),
                    value: format.as_ref().clone(),
                }],
                _ => {
                    self.output_struct_or_variant_new_type_container(None, None, name, format)?;
                    return Ok(());
                }
            },
            TupleStruct(formats) => formats
                .iter()
                .enumerate()
                .map(|(i, f)| Named {
                    name: format!("Field{i}"),
                    value: f.clone(),
                })
                .collect(),
            Struct(fields) => fields
                .iter()
                .map(|f| Named {
                    name: f.name.to_camel_case(),
                    value: f.value.clone(),
                })
                .collect(),
            Enum(variants) => {
                let variants = variants
                    .iter()
                    .map(|(i, f)| {
                        (
                            *i,
                            Named {
                                name: f.name.to_camel_case(),
                                value: f.value.clone(),
                            },
                        )
                    })
                    .collect();
                self.output_enum_container(name, &variants)?;
                return Ok(());
            }
        };
        self.output_struct_or_variant_container(None, None, name, &fields)
    }
}

/// Installer for generated source files in Go.
pub struct Installer {
    install_dir: PathBuf,
    serde_module_path: Option<String>,
}

impl Installer {
    pub fn new(install_dir: PathBuf, serde_module_path: Option<String>) -> Self {
        Installer {
            install_dir,
            serde_module_path,
        }
    }

    fn runtime_installation_message(&self, name: &str) {
        eprintln!(
            "Not installing sources for published package {}{}",
            match &self.serde_module_path {
                None => String::new(),
                Some(path) => format!("{path}/"),
            },
            name
        );
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_module(
        &self,
        config: &CodeGeneratorConfig,
        registry: &Registry,
    ) -> std::result::Result<(), Self::Error> {
        let dir_path = self.install_dir.join(&config.module_name);
        std::fs::create_dir_all(&dir_path)?;
        let source_path = dir_path.join("lib.go");
        let mut file = std::fs::File::create(source_path)?;

        let mut generator = CodeGenerator::new(config);
        if let Some(path) = &self.serde_module_path {
            generator = generator.with_serde_module_path(path.clone());
        }
        generator.output(&mut file, registry)?;
        Ok(())
    }

    fn install_serde_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.runtime_installation_message("serde");
        Ok(())
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.runtime_installation_message("bincode");
        Ok(())
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.runtime_installation_message("bcs");
        Ok(())
    }
}

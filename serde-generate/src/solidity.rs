// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    analyzer,
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig, Encoding,
};
use heck::CamelCase;
use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Result, Write},
    path::PathBuf,
};

/// Main configuration object for code-generation in C++.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
    /// Mapping from external type names to suitably qualified names (e.g. "MyClass" -> "name::MyClass").
    /// Derived from `config.external_definitions`.
    external_qualified_names: HashMap<String, String>,
}

/// Shared state for the code generation of a C++ source file.
struct SolEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
    /// Track which type names have been declared so far. (Used to add forward declarations.)
    known_names: HashSet<&'a str>,
    /// Track which definitions have a known size. (Used to add shared pointers.)
    known_sizes: HashSet<&'a str>,
    /// Current namespace (e.g. vec!["name", "MyClass"])
    current_namespace: Vec<String>,
}

impl<'a> CodeGenerator<'a> {
    /// Create a C++ code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.c_style_enums {
            panic!("C++ does not support generating c-style enums");
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

    pub fn output(
        &self,
        out: &mut dyn Write,
        registry: &Registry,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let current_namespace = self
            .config
            .module_name
            .split("::")
            .map(String::from)
            .collect();
        let mut emitter = SolEmitter {
            out: IndentedWriter::new(out, IndentConfig::Space(4)),
            generator: self,
            known_names: HashSet::new(),
            known_sizes: HashSet::new(),
            current_namespace,
        };

        emitter.output_preamble()?;
        emitter.output_open_contract()?;

        let dependencies = analyzer::get_dependency_map(registry)?;
        let entries = analyzer::best_effort_topological_sort(&dependencies);


        for name in entries {
            for dependency in &dependencies[name] {
                if !emitter.known_names.contains(dependency) {
                    panic!("Type circular dependencies are not allowed in solidity");
                }
            }
            let format = &registry[name];
            emitter.output_container(name, format)?;
            emitter.known_sizes.insert(name);
            emitter.known_names.insert(name);
        }

        emitter.output_close_contract()?;
        writeln!(emitter.out)?;
        for (name, format) in registry {
            emitter.output_container_traits(name, format)?;
        }
        Ok(())
    }
}

impl<'a, T> SolEmitter<'a, T>
where
    T: std::io::Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        writeln!(
            self.out,
            r#"pragma solidity ^0.8.0;"#
        )?;
        Ok(())
    }

    fn output_open_contract(&mut self) -> Result<()> {
        writeln!(
            self.out,
            "\nnamespace {} {{",
            self.generator.config.module_name
        )?;
        self.out.indent();
        Ok(())
    }

    fn output_close_contract(&mut self) -> Result<()> {
        self.out.unindent();
        writeln!(
            self.out,
            "\n}} // end of contract {}",
            self.generator.config.module_name
        )?;
        Ok(())
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

    fn output_custom_code(&mut self) -> std::io::Result<()> {
        if let Some(code) = self
            .generator
            .config
            .custom_code
            .get(&self.current_namespace)
        {
            write!(self.out, "\n{}", code)?;
        }
        Ok(())
    }

    /// Compute a fully qualified reference to the container type `name`.
    fn quote_qualified_name(&self, name: &str) -> String {
        self.generator
            .external_qualified_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("{}.{}", self.generator.config.module_name, name))
    }

    fn quote_type(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(x) => self.quote_qualified_name(x),
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
            F32 => panic!("no support for floating point in solidity"),
            F64 => panic!("no support for floating point in solidity"),
            Char => "bytes1".into(),
            Str => "string".into(),
            Bytes => "bytes".into(),

            Option(format) => format!("opt_{}", self.quote_type(format)),
            Seq(format) => format!("{}[]", self.quote_type(format)),
            Map { key, value } => format!("map_{}_{}",
                self.quote_type(key, false),
                self.quote_type(value, false)
            ),
            Tuple(formats) => format!("tuple_{}",
                                      formats
                                      .iter()
                                      .map(|x| self.quote_type(x, require_known_size))
                                      .collect::<Vec<_>>()
                                      .join("_")
            ),
            TupleArray { content, size } => format!("array{}_{}[]", *size, self.quote_type(content)),

            Variable(_) => panic!("unexpected value"),
        }
    }

    fn output_struct_or_variant_container(
        &mut self,
        name: &str,
        fields: &[Named<Format>],
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "struct {} {{", name)?;
        for field in fields {
            self.output_comment(&field.name)?;
            writeln!(
                self.out,
                "{} {};",
                self.quote_type(&field.value, true),
                field.name
            )?;
        }
        if !fields.is_empty() {
            writeln!(self.out)?;
        }
        self.output_class_method_declarations(name)?;
        self.output_custom_code()?;
        writeln!(self.out, "}};")
    }

    fn output_variant(&mut self, name: &str, variant: &VariantFormat) -> Result<()> {
        use VariantFormat::*;
        let fields = match variant {
            Unit => Vec::new(),
            NewType(format) => vec![Named {
                name: "value".to_string(),
                value: format.as_ref().clone(),
            }],
            Tuple(formats) => vec![Named {
                name: "value".to_string(),
                value: Format::Tuple(formats.clone()),
            }],
            Struct(fields) => fields.clone(),
            Variable(_) => panic!("incorrect value"),
        };
        self.output_struct_or_variant_container(name, &fields)
    }

    fn output_enum_container(
        &mut self,
        name: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        writeln!(self.out)?;
        self.output_comment(name)?;
        writeln!(self.out, "struct {} {{", name)?;
        for (expected_index, (index, variant)) in variants.iter().enumerate() {
            assert_eq!(*index, expected_index as u32);
            self.output_variant(&variant.name, &variant.value)?;
        }
        writeln!(
            self.out,
            "\nstd::variant<{}> value;",
            variants
                .iter()
                .map(|(_, v)| v.name.clone())
                .collect::<Vec<_>>()
                .join(", "),
        )?;
        writeln!(self.out)?;
        self.output_class_method_declarations(name)?;
        self.output_custom_code()?;
        writeln!(self.out, "}};")
    }

    fn output_class_method_declarations(&mut self, name: &str) -> Result<()> {
        writeln!(
            self.out,
            "friend bool operator==(const {}&, const {}&);",
            name, name
        )?;
        if self.generator.config.serialization {
            for encoding in &self.generator.config.encodings {
                writeln!(
                    self.out,
                    "std::vector<uint8_t> {}Serialize() const;",
                    encoding.name()
                )?;
                writeln!(
                    self.out,
                    "static {} {}Deserialize(std::vector<uint8_t>);",
                    name,
                    encoding.name()
                )?;
            }
        }
        Ok(())
    }

    fn output_container(&mut self, name: &str, format: &ContainerFormat) -> Result<()> {
        use ContainerFormat::*;
        let fields = match format {
            UnitStruct => Vec::new(),
            NewTypeStruct(format) => vec![Named {
                name: "value".to_string(),
                value: format.as_ref().clone(),
            }],
            TupleStruct(formats) => vec![Named {
                name: "value".to_string(),
                value: Format::Tuple(formats.clone()),
            }],
            Struct(fields) => fields.clone(),
            Enum(variants) => {
                if Self::is_trivial(variant) {
                    Self::output_trivial_enum(name, variants)?;
                    return Ok(())
                } else {
                    let name = self.get_composed_name(
                }

                
                self.output_enum_container(name, variants)?;
                return Ok(());
            }
        };
        self.output_struct_or_variant_container(name, &fields)
    }

    fn output_struct_equality_test(&mut self, name: &str, fields: &[&str]) -> Result<()> {
        writeln!(
            self.out,
            "\ninline bool operator==(const {0} &lhs, const {0} &rhs) {{",
            name,
        )?;
        self.out.indent();
        for field in fields {
            writeln!(
                self.out,
                "if (!(lhs.{0} == rhs.{0})) {{ return false; }}",
                field,
            )?;
        }
        writeln!(self.out, "return true;")?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_struct_serialize_for_encoding(
        &mut self,
        name: &str,
        encoding: Encoding,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
inline std::vector<uint8_t> {}::{}Serialize() const {{
    auto serializer = serde::{}Serializer();
    serde::Serializable<{}>::serialize(*this, serializer);
    return std::move(serializer).bytes();
}}"#,
            name,
            encoding.name(),
            encoding.name().to_camel_case(),
            name
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
inline {} {}::{}Deserialize(std::vector<uint8_t> input) {{
    auto deserializer = serde::{}Deserializer(input);
    auto value = serde::Deserializable<{}>::deserialize(deserializer);
    if (deserializer.get_buffer_offset() < input.size()) {{
        throw serde::deserialization_error("Some input bytes were not read");
    }}
    return value;
}}"#,
            name,
            name,
            encoding.name(),
            encoding.name().to_camel_case(),
            name,
        )
    }

    fn output_struct_serializable(
        &mut self,
        name: &str,
        fields: &[&str],
        is_container: bool,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
template <>
template <typename Serializer>
void serde::Serializable<{0}>::serialize(const {0} &obj, Serializer &serializer) {{"#,
            name,
        )?;
        self.out.indent();
        if is_container {
            writeln!(self.out, "serializer.increase_container_depth();")?;
        }
        for field in fields {
            writeln!(
                self.out,
                "serde::Serializable<decltype(obj.{0})>::serialize(obj.{0}, serializer);",
                field,
            )?;
        }
        if is_container {
            writeln!(self.out, "serializer.decrease_container_depth();")?;
        }
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_struct_deserializable(
        &mut self,
        name: &str,
        fields: &[&str],
        is_container: bool,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
template <>
template <typename Deserializer>
{0} serde::Deserializable<{0}>::deserialize(Deserializer &deserializer) {{"#,
            name,
        )?;
        self.out.indent();
        if is_container {
            writeln!(self.out, "deserializer.increase_container_depth();")?;
        }
        writeln!(self.out, "{} obj;", name)?;
        for field in fields {
            writeln!(
                self.out,
                "obj.{0} = serde::Deserializable<decltype(obj.{0})>::deserialize(deserializer);",
                field,
            )?;
        }
        if is_container {
            writeln!(self.out, "deserializer.decrease_container_depth();")?;
        }
        writeln!(self.out, "return obj;")?;
        self.out.unindent();
        writeln!(self.out, "}}")
    }

    fn output_struct_traits(
        &mut self,
        name: &str,
        fields: &[&str],
        is_container: bool,
    ) -> Result<()> {
        self.output_open_namespace()?;
        self.output_struct_equality_test(name, fields)?;
        if self.generator.config.serialization {
            for encoding in &self.generator.config.encodings {
                self.output_struct_serialize_for_encoding(name, *encoding)?;
                self.output_struct_deserialize_for_encoding(name, *encoding)?;
            }
        }
        self.output_close_namespace()?;
        let namespaced_name = self.quote_qualified_name(name);
        if self.generator.config.serialization {
            self.output_struct_serializable(&namespaced_name, fields, is_container)?;
            self.output_struct_deserializable(&namespaced_name, fields, is_container)?;
        }
        Ok(())
    }

    fn get_variant_fields(format: &VariantFormat) -> Vec<&str> {
        use VariantFormat::*;
        match format {
            Unit => Vec::new(),
            NewType(_format) => vec!["value"],
            Tuple(_formats) => vec!["value"],
            Struct(fields) => fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            Variable(_) => panic!("incorrect value"),
        }
    }

    fn output_container_traits(&mut self, name: &str, format: &ContainerFormat) -> Result<()> {
        use ContainerFormat::*;
        match format {
            UnitStruct => self.output_struct_traits(name, &[], true),
            NewTypeStruct(_format) => self.output_struct_traits(name, &["value"], true),
            TupleStruct(_formats) => self.output_struct_traits(name, &["value"], true),
            Struct(fields) => self.output_struct_traits(
                name,
                &fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>(),
                true,
            ),
            Enum(variants) => {
                self.output_struct_traits(name, &["value"], true)?;
                for variant in variants.values() {
                    self.output_struct_traits(
                        &format!("{}::{}", name, variant.name),
                        &Self::get_variant_fields(&variant.value),
                        false,
                    )?;
                }
                Ok(())
            }
        }
    }
}

/// Installer for generated source files in C++.
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
        let mut file = self.create_header_file("binary")?;
        write!(file, "{}", include_str!("../runtime/solidity/binary.sol"))?;
        Ok(())
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_header_file("bincode")?;
        write!(file, "{}", include_str!("../runtime/solidity/bincode.sol"))?;
        Ok(())
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_header_file("bcs")?;
        write!(file, "{}", include_str!("../runtime/solidity/bcs.sol"))?;
        Ok(())
    }
}

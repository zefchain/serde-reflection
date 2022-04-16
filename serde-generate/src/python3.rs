// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig, Encoding,
};
use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap, HashMap},
    io::{Result, Write},
    path::PathBuf,
};

/// Main configuration object for code-generation in Python.
pub struct CodeGenerator<'a> {
    /// Language-independent configuration.
    config: &'a CodeGeneratorConfig,
    /// Whether the module providing Serde definitions is located within package.
    serde_package_name: Option<String>,
    /// Mapping from external type names to suitably qualified names (e.g. "MyClass" -> "my_module.MyClass").
    /// Assumes suitable imports (e.g. "from my_package import my_module").
    /// Derived from `config.external_definitions`.
    external_qualified_names: HashMap<String, String>,
}

/// Shared state for the code generation of a Python source file.
struct PythonEmitter<'a, T> {
    /// Writer.
    out: IndentedWriter<T>,
    /// Generator.
    generator: &'a CodeGenerator<'a>,
    /// Current namespace (e.g. vec!["my_package", "my_module", "MyClass"])
    current_namespace: Vec<String>,
}

impl<'a> CodeGenerator<'a> {
    /// Create a Python code generator for the given config.
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.c_style_enums {
            panic!("Python 3 does not support generating c-style enums");
        }
        let mut external_qualified_names = HashMap::new();
        for (module_path, names) in &config.external_definitions {
            let module = {
                let mut path = module_path.split('.').collect::<Vec<_>>();
                if path.len() < 2 {
                    module_path
                } else {
                    path.pop().unwrap()
                }
            };
            for name in names {
                external_qualified_names.insert(name.to_string(), format!("{}.{}", module, name));
            }
        }
        Self {
            config,
            serde_package_name: None,
            external_qualified_names,
        }
    }

    /// Whether the module providing Serde definitions is located within a package.
    pub fn with_serde_package_name(mut self, serde_package_name: Option<String>) -> Self {
        self.serde_package_name = serde_package_name;
        self
    }

    /// Write container definitions in Python.
    pub fn output(&self, out: &mut dyn Write, registry: &Registry) -> Result<()> {
        let current_namespace = self
            .config
            .module_name
            .split('.')
            .map(String::from)
            .collect();
        let mut emitter = PythonEmitter {
            out: IndentedWriter::new(out, IndentConfig::Space(4)),
            generator: self,
            current_namespace,
        };
        emitter.output_preamble()?;
        for (name, format) in registry {
            emitter.output_container(name, format)?;
        }
        Ok(())
    }
}

impl<'a, T> PythonEmitter<'a, T>
where
    T: Write,
{
    fn quote_import(&self, module: &str) -> String {
        let mut parts = module.split('.').collect::<Vec<_>>();
        if parts.len() <= 1 {
            format!("import {}", module)
        } else {
            let module_name = parts.pop().unwrap();
            format!("from {} import {}", parts.join("."), module_name)
        }
    }

    fn output_preamble(&mut self) -> Result<()> {
        let from_serde_package = match &self.generator.serde_package_name {
            None => "".to_string(),
            Some(name) => format!("from {} ", name),
        };
        writeln!(
            self.out,
            r#"# pyre-strict
from dataclasses import dataclass
import typing
{}import serde_types as st"#,
            from_serde_package,
        )?;
        for encoding in &self.generator.config.encodings {
            writeln!(self.out, "{}import {}", from_serde_package, encoding.name())?;
        }
        for module in self.generator.config.external_definitions.keys() {
            writeln!(self.out, "{}\n", self.quote_import(module))?;
        }
        Ok(())
    }

    /// Compute a reference to the registry type `name`.
    /// Use a qualified name in case of external definitions.
    fn quote_qualified_name(&self, name: &str) -> String {
        self.generator
            .external_qualified_names
            .get(name)
            .cloned()
            .unwrap_or_else(|| {
                // Need quotes because of circular dependencies.
                format!("\"{}\"", name)
            })
    }

    fn quote_type(&self, format: &Format) -> String {
        use Format::*;
        match format {
            TypeName(x) => self.quote_qualified_name(x),
            Unit => "st.unit".into(),
            Bool => "bool".into(),
            I8 => "st.int8".into(),
            I16 => "st.int16".into(),
            I32 => "st.int32".into(),
            I64 => "st.int64".into(),
            I128 => "st.int128".into(),
            U8 => "st.uint8".into(),
            U16 => "st.uint16".into(),
            U32 => "st.uint32".into(),
            U64 => "st.uint64".into(),
            U128 => "st.uint128".into(),
            F32 => "st.float32".into(),
            F64 => "st.float64".into(),
            Char => "st.char".into(),
            Str => "str".into(),
            Bytes => "bytes".into(),

            Option(format) => format!("typing.Optional[{}]", self.quote_type(format)),
            Seq(format) => format!("typing.Sequence[{}]", self.quote_type(format)),
            Map { key, value } => format!(
                "typing.Dict[{}, {}]",
                self.quote_type(key),
                self.quote_type(value)
            ),
            Tuple(formats) => {
                if formats.is_empty() {
                    "typing.Tuple[()]".into()
                } else {
                    format!("typing.Tuple[{}]", self.quote_types(formats))
                }
            }
            TupleArray { content, size } => format!(
                "typing.Tuple[{}]",
                self.quote_types(&vec![content.as_ref().clone(); *size])
            ), // Sadly, there are no fixed-size arrays in python.

            Variable(_) => panic!("unexpected value"),
        }
    }

    fn quote_types(&self, formats: &[Format]) -> String {
        formats
            .iter()
            .map(|x| self.quote_type(x))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn output_comment(&mut self, name: &str) -> std::io::Result<()> {
        let mut path = self.current_namespace.clone();
        path.push(name.to_string());
        if let Some(doc) = self.generator.config.comments.get(&path) {
            writeln!(self.out, "\"\"\"{}\"\"\"", doc)?;
        }
        Ok(())
    }

    fn output_custom_code(&mut self) -> std::io::Result<bool> {
        match self
            .generator
            .config
            .custom_code
            .get(&self.current_namespace)
        {
            Some(code) => {
                writeln!(self.out, "\n{}", code)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn output_fields(&mut self, fields: &[Named<Format>]) -> Result<()> {
        if fields.is_empty() {
            writeln!(self.out, "pass")?;
            return Ok(());
        }
        for field in fields {
            writeln!(
                self.out,
                "{}: {}",
                field.name,
                self.quote_type(&field.value)
            )?;
        }
        Ok(())
    }

    fn output_variant(
        &mut self,
        base: &str,
        name: &str,
        index: u32,
        variant: &VariantFormat,
    ) -> Result<()> {
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

        // Regarding comments, we pretend the namespace is `[module, base, name]`.
        writeln!(
            self.out,
            "\n@dataclass(frozen=True)\nclass {0}__{1}({0}):",
            base, name
        )?;
        self.out.indent();
        self.output_comment(name)?;
        if self.generator.config.serialization {
            writeln!(self.out, "INDEX = {}  # type: int", index)?;
        }
        self.current_namespace.push(name.to_string());
        self.output_fields(&fields)?;
        self.output_custom_code()?;
        self.current_namespace.pop();
        self.out.unindent();
        writeln!(self.out)
    }

    fn output_enum_container(
        &mut self,
        name: &str,
        variants: &BTreeMap<u32, Named<VariantFormat>>,
    ) -> Result<()> {
        writeln!(self.out, "\nclass {}:", name)?;
        self.out.indent();
        self.output_comment(name)?;
        self.current_namespace.push(name.to_string());
        if self.generator.config.serialization {
            writeln!(
                self.out,
                "VARIANTS = []  # type: typing.Sequence[typing.Type[{}]]",
                name
            )?;
            for encoding in &self.generator.config.encodings {
                self.output_serialize_method_for_encoding(name, *encoding)?;
                self.output_deserialize_method_for_encoding(name, *encoding)?;
            }
        }
        let wrote_custom_code = self.output_custom_code()?;
        if !self.generator.config.serialization && !wrote_custom_code {
            writeln!(self.out, "pass")?;
        }
        writeln!(self.out)?;
        self.out.unindent();

        for (index, variant) in variants {
            self.output_variant(name, &variant.name, *index, &variant.value)?;
        }
        self.current_namespace.pop();

        if self.generator.config.serialization {
            writeln!(
                self.out,
                "{}.VARIANTS = [\n{}]\n",
                name,
                variants
                    .iter()
                    .map(|(_, v)| format!("    {}__{},\n", name, v.name))
                    .collect::<Vec<_>>()
                    .join("")
            )?;
        }
        Ok(())
    }

    fn output_serialize_method_for_encoding(
        &mut self,
        name: &str,
        encoding: Encoding,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
def {0}_serialize(self) -> bytes:
    return {0}.serialize(self, {1})"#,
            encoding.name(),
            name
        )
    }

    fn output_deserialize_method_for_encoding(
        &mut self,
        name: &str,
        encoding: Encoding,
    ) -> Result<()> {
        writeln!(
            self.out,
            r#"
@staticmethod
def {0}_deserialize(input: bytes) -> '{1}':
    v, buffer = {0}.deserialize(input, {1})
    if buffer:
        raise st.DeserializationError("Some input bytes were not read");
    return v"#,
            encoding.name(),
            name
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
            TupleStruct(formats) => vec![Named {
                name: "value".to_string(),
                value: Format::Tuple(formats.clone()),
            }],
            Struct(fields) => fields.clone(),
            Enum(variants) => {
                // Enum case.
                self.output_enum_container(name, variants)?;
                return Ok(());
            }
        };
        // Struct case.
        writeln!(self.out, "\n@dataclass(frozen=True)\nclass {}:", name)?;
        self.out.indent();
        self.output_comment(name)?;
        self.current_namespace.push(name.to_string());
        self.output_fields(&fields)?;
        for encoding in &self.generator.config.encodings {
            self.output_serialize_method_for_encoding(name, *encoding)?;
            self.output_deserialize_method_for_encoding(name, *encoding)?;
        }
        self.output_custom_code()?;
        self.current_namespace.pop();
        self.out.unindent();
        writeln!(self.out)
    }
}

/// Installer for generated source files in Python.
pub struct Installer {
    install_dir: PathBuf,
    serde_package_name: Option<String>,
}

impl Installer {
    pub fn new(install_dir: PathBuf, serde_package_name: Option<String>) -> Self {
        Installer {
            install_dir,
            serde_package_name,
        }
    }

    fn create_module_init_file(&self, name: &str) -> Result<std::fs::File> {
        let dir_path = self.install_dir.join(name);
        std::fs::create_dir_all(&dir_path)?;
        std::fs::File::create(dir_path.join("__init__.py"))
    }

    fn fix_serde_package(&self, content: &str) -> String {
        match &self.serde_package_name {
            None => content.into(),
            Some(name) => content
                .replace(
                    "import serde_types",
                    &format!("from {} import serde_types", name),
                )
                .replace(
                    "import serde_binary",
                    &format!("from {} import serde_binary", name),
                ),
        }
    }
}

impl crate::SourceInstaller for Installer {
    type Error = Box<dyn std::error::Error>;

    fn install_module(
        &self,
        config: &crate::CodeGeneratorConfig,
        registry: &Registry,
    ) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_module_init_file(&config.module_name)?;
        let generator =
            CodeGenerator::new(config).with_serde_package_name(self.serde_package_name.clone());
        generator.output(&mut file, registry)?;
        Ok(())
    }

    fn install_serde_runtime(&self) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_module_init_file("serde_types")?;
        write!(
            file,
            "{}",
            self.fix_serde_package(include_str!("../runtime/python/serde_types/__init__.py"))
        )?;
        let mut file = self.create_module_init_file("serde_binary")?;
        write!(
            file,
            "{}",
            self.fix_serde_package(include_str!("../runtime/python/serde_binary/__init__.py"))
        )?;
        Ok(())
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_module_init_file("bincode")?;
        write!(
            file,
            "{}",
            self.fix_serde_package(include_str!("../runtime/python/bincode/__init__.py"))
        )?;
        Ok(())
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        let mut file = self.create_module_init_file("bcs")?;
        write!(
            file,
            "{}",
            self.fix_serde_package(include_str!("../runtime/python/bcs/__init__.py"))
        )?;
        Ok(())
    }
}

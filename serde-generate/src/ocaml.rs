use crate::{
    common::capitalize,
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig,
};
use heck::CamelCase;
use heck::SnakeCase;
use include_dir::include_dir as include_directory;
use phf::phf_set;
use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
use std::{
    collections::BTreeMap,
    io::{Result, Write},
    path::PathBuf,
};

pub struct CodeGenerator<'a> {
    config: &'a CodeGeneratorConfig,
    libraries: Vec<String>,
}

struct OCamlEmitter<'a, T> {
    out: IndentedWriter<T>,
    generator: &'a CodeGenerator<'a>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(config: &'a CodeGeneratorConfig) -> Self {
        if config.c_style_enums {
            panic!("OCaml does not support generating c-style enums");
        }
        Self {
            config,
            libraries: config
                .external_definitions
                .keys()
                .map(|k| k.to_string())
                .collect::<Vec<_>>(),
        }
    }

    pub fn output(&self, out: &mut dyn Write, registry: &Registry) -> Result<()> {
        let mut emitter = OCamlEmitter {
            out: IndentedWriter::new(out, IndentConfig::Space(2)),
            generator: self,
        };
        emitter.output_preamble()?;
        let n = registry.len();
        for (i, (name, format)) in registry.iter().enumerate() {
            let first = i == 0;
            let last = i == n - 1;
            emitter.output_container(name, format, first, last)?;
        }
        Ok(())
    }
}

static KEYWORDS: phf::Set<&str> = phf_set! {
    "and", "as", "assert", "asr",
    "begin", "class", "constraint",
    "do", "done", "downto", "else",
    "end", "exception", "external",
    "false", "for", "fun", "function",
    "functor", "if", "in", "include",
    "inherit", "initializer", "land",
    "lazy", "let", "lor", "lsl",
    "lsr", "lxor", "match", "method",
    "mod", "module", "mutable", "new",
    "nonrec", "object", "of", "open",
    "or", "private", "rec", "sig",
    "struct", "then", "to", "true",
    "try", "type", "val", "virtual",
    "when", "while", "with", "bool",
    "string", "bytes", "char", "unit",
    "option", "float", "list",
    "int32", "int64"
};

impl<'a, T> OCamlEmitter<'a, T>
where
    T: Write,
{
    fn output_preamble(&mut self) -> Result<()> {
        for namespace in self.generator.libraries.iter() {
            writeln!(self.out, "open {}", capitalize(namespace))?;
        }
        Ok(())
    }

    fn type_name(&self, s: &str) -> String {
        let s = s.to_snake_case();
        if KEYWORDS.contains(&*s) {
            s + "_"
        } else {
            s
        }
    }

    fn output_format(&mut self, format: &Format) -> Result<()> {
        use Format::*;
        match format {
            Variable(_) => panic!("incorrect value"),
            TypeName(s) => write!(self.out, "{}", self.type_name(s)),
            Unit => write!(self.out, "unit"),
            Bool => write!(self.out, "bool"),
            I8 => write!(self.out, "Stdint.int8"),
            I16 => write!(self.out, "Stdint.int16"),
            I32 => write!(self.out, "int32"),
            I64 => write!(self.out, "int64"),
            I128 => write!(self.out, "Stdint.int128"),
            U8 => write!(self.out, "Stdint.uint8"),
            U16 => write!(self.out, "Stdint.uint16"),
            U32 => write!(self.out, "Stdint.uint32"),
            U64 => write!(self.out, "Stdint.uint64"),
            U128 => write!(self.out, "Stdint.uint128"),
            F32 => write!(self.out, "float"),
            F64 => write!(self.out, "float"),
            Char => write!(self.out, "char"),
            Str => write!(self.out, "string"),
            Bytes => write!(self.out, "bytes"),
            Option(f) => {
                self.output_format(f)?;
                write!(self.out, " option")
            }
            Seq(f) => {
                self.output_format(f)?;
                write!(self.out, " list")
            }
            Map { key, value } => self.output_map(key, value),
            Tuple(fs) => self.output_tuple(fs),
            TupleArray { content, size } => {
                self.output_tuple(&vec![content.as_ref().clone(); *size])
            }
        }
    }

    fn output_map(&mut self, key: &Format, value: &Format) -> Result<()> {
        write!(self.out, "(")?;
        self.output_format(key)?;
        write!(self.out, ", ")?;
        self.output_format(value)?;
        write!(self.out, ") Serde.map")
    }

    fn output_tuple(&mut self, formats: &[Format]) -> Result<()> {
        write!(self.out, "(")?;
        let n = formats.len();
        formats
            .iter()
            .enumerate()
            .map(|(i, f)| {
                self.output_format(f)?;
                if i != n - 1 {
                    write!(self.out, " * ")
                } else {
                    Ok(())
                }
            })
            .collect::<Result<Vec<_>>>()?;
        write!(self.out, ")")
    }

    fn output_record(&mut self, formats: &[Named<Format>]) -> Result<()> {
        writeln!(self.out, "{{")?;
        self.out.indent();
        formats
            .iter()
            .map(|f| {
                write!(self.out, "{}: ", f.name)?;
                self.output_format(&f.value)?;
                writeln!(self.out, ";")
            })
            .collect::<Result<Vec<_>>>()?;
        self.out.unindent();
        write!(self.out, "\n}}")
    }

    fn output_variant(&mut self, format: &VariantFormat) -> Result<()> {
        use VariantFormat::*;
        match format {
            Variable(_) => panic!("incorrect value"),
            Unit => Ok(()),
            NewType(f) => {
                write!(self.out, " of ")?;
                self.output_format(f)
            }
            Tuple(fields) if fields.is_empty() => Ok(()),
            Tuple(fields) => {
                write!(self.out, " of ")?;
                self.output_tuple(fields)
            }
            Struct(fields) if fields.is_empty() => Ok(()),
            Struct(fields) => {
                write!(self.out, " of ")?;
                self.output_record(fields)
            }
        }
    }

    fn output_enum(&mut self, formats: &BTreeMap<u32, Named<VariantFormat>>) -> Result<()> {
        writeln!(self.out)?;
        self.out.indent();
        formats
            .iter()
            .map(|(_, f)| {
                write!(self.out, "| {}", f.name)?;
                self.output_variant(&f.value)?;
                writeln!(self.out)
            })
            .collect::<Result<Vec<_>>>()?;
        self.out.unindent();
        Ok(())
    }

    fn is_cyclic(&self, name: &str, format: &Format) -> bool {
        use Format::*;
        match format {
            TypeName(s) => name == s,
            Option(f) => self.is_cyclic(name, f),
            Seq(f) => self.is_cyclic(name, f),
            Map { key, value } => self.is_cyclic(name, key) || self.is_cyclic(name, value),
            Tuple(fs) => fs.iter().any(|f| self.is_cyclic(name, f)),
            TupleArray { content, size: _ } => self.is_cyclic(name, content),
            _ => false,
        }
    }

    fn output_container(
        &mut self,
        name: &str,
        format: &ContainerFormat,
        first: bool,
        last: bool,
    ) -> Result<()> {
        use ContainerFormat::*;
        write!(
            self.out,
            "{} {} = ",
            if first { "type" } else { "and" },
            self.type_name(name)
        )?;
        match format {
            UnitStruct => write!(self.out, "unit"),
            NewTypeStruct(format) if self.is_cyclic(name, format.as_ref()) => {
                let mut map = BTreeMap::new();
                map.insert(
                    0,
                    Named {
                        name: format!("{}_", name.to_camel_case()),
                        value: VariantFormat::NewType(format.clone()),
                    },
                );
                self.output_enum(&map)
            }
            NewTypeStruct(format) => self.output_format(format.as_ref()),
            TupleStruct(formats) => self.output_tuple(formats),
            Struct(fields) => self.output_record(fields),
            Enum(variants) => self.output_enum(variants),
        }?;
        writeln!(
            self.out,
            "{}",
            if last && self.generator.config.serialization {
                "[@@deriving serde]"
            } else {
                ""
            }
        )
    }
}

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
        let dir_path = self.install_dir.join(&config.module_name);
        std::fs::create_dir_all(&dir_path)?;
        let dune_project_source_path = self.install_dir.join("dune-project");
        let mut dune_project_file = std::fs::File::create(dune_project_source_path)?;
        writeln!(dune_project_file, "(lang dune 3.0)")?;
        let dune_source_path = dir_path.join("dune");
        let mut dune_file = std::fs::File::create(dune_source_path)?;
        let name = config.module_name.to_snake_case();
        writeln!(
            dune_file,
            "(env (_ (flags (:standard -w -30-42))))\n\n(library\n (name {0})\n (modules {0})\n (preprocess (pps ppx)))",
            name
        )?;
        let source_path = dir_path.join(format!("{}.ml", name));
        let mut file = std::fs::File::create(source_path)?;
        let generator = CodeGenerator::new(config);
        generator.output(&mut file, registry)?;
        Ok(())
    }

    fn install_serde_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(include_directory!("runtime/ocaml/common"), "common")?;
        self.install_runtime(include_directory!("runtime/ocaml/virtual"), "virtual")?;
        self.install_runtime(include_directory!("runtime/ocaml/ppx"), "ppx")?;
        self.install_runtime(include_directory!("runtime/ocaml/serde"), "serde")
    }

    fn install_bincode_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(include_directory!("runtime/ocaml/common"), "common")?;
        self.install_runtime(include_directory!("runtime/ocaml/virtual"), "virtual")?;
        self.install_runtime(include_directory!("runtime/ocaml/ppx"), "ppx")?;
        self.install_runtime(include_directory!("runtime/ocaml/serde"), "serde")?;
        self.install_runtime(include_directory!("runtime/ocaml/bincode"), "bincode")
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(include_directory!("runtime/ocaml/common"), "common")?;
        self.install_runtime(include_directory!("runtime/ocaml/virtual"), "virtual")?;
        self.install_runtime(include_directory!("runtime/ocaml/ppx"), "ppx")?;
        self.install_runtime(include_directory!("runtime/ocaml/serde"), "serde")?;
        self.install_runtime(include_directory!("runtime/ocaml/bcs"), "bcs")
    }
}

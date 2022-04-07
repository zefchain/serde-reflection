use crate::{
    indent::{IndentConfig, IndentedWriter},
    CodeGeneratorConfig,
};
use heck::SnakeCase;
use include_dir::include_dir as include_directory;
use serde_reflection::{ContainerFormat, Format, Named, Registry, VariantFormat};
use std::{
    collections::{BTreeMap},
    io::{Result, Write},
    path::PathBuf,
};

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

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
            libraries: config.external_definitions.keys().map(|k| k.to_string()).collect::<Vec<_>>(),
        }
    }

    pub fn output(&self, out: &mut dyn Write, registry: &Registry) -> Result<()> {
        let mut emitter = OCamlEmitter {
            out: IndentedWriter::new(out, IndentConfig::Space(2)),
            generator: self,
        };
        emitter.output_preamble()?;
        for (name, format) in registry {
            emitter.output_container(name, format)?;
        }
        Ok(())
    }
}

impl<'a, T> OCamlEmitter<'a, T>
where T: Write, {

    fn output_preamble(&mut self) -> Result<()> {
        for namespace in self.generator.libraries.iter() {
            writeln!(self.out, "open {}\n", capitalize(namespace))?;
        }
        Ok(())
    }

    fn format(&self, format: &Format) -> String {
        use Format::*;
        match format {
            Variable(_) => panic!("incorrect value"),
            TypeName(s) => s.to_string(),
            Unit => "unit".into(),
            Bool => "bool".into(),
            I8 => "Stdint.int8".into(),
            I16 => "Stdint.int16".into(),
            I32 => "int32".into(),
            I64 => "int64".into(),
            I128 => "Stdint.int128".into(),
            U8 => "Stdint.uint8".into(),
            U16 => "Stdint.uint16".into(),
            U32 => "Stdint.uint32".into(),
            U64 => "Stdint.uint64".into(),
            U128 => "Stdint.uint128".into(),
            F32 => panic!("float32 not implemented in ocaml"),
            F64 => "float".into(),
            Char => "char".into(),
            Str => "string".into(),
            Bytes => "bytes".into(),
            Option(f) => format!("{} option", self.format(f)),
            Seq(f) => format!("{} list", self.format(f)),
            Map{key, value} => self.map_format(key, value),
            Tuple(fs) => self.tuple_format(fs),
            TupleArray{content, size} => self.tuple_format(&vec![content.as_ref().clone(); *size]),
        }
    }

    fn map_format(&self, key: &Format, value: &Format) -> String {
        format!("({}, {}) Serde.map", self.format(key), self.format(value))
    }

    fn tuple_format(&self, formats: &Vec<Format>) -> String {
        format!("({})", formats.iter().map(|f| self.format(f)).collect::<Vec<_>>().join(" * "))
    }

    fn record_format(&self, formats: &Vec<Named<Format>>) -> String {
        format!("{{\n{}\n}}", formats.iter().map(|f| format!("  {}: {}\n", f.name, self.format(&f.value))).collect::<Vec<_>>().join("; "))
    }

    fn variant_format(&self, format: &VariantFormat) -> String {
        use VariantFormat::*;
        match format {
            Variable(_) => panic!("incorrect value"),
            Unit => "".to_string(),
            NewType(f) => format!(" of {}", self.format(f)),
            Tuple(fields) => format!(" of {}", self.tuple_format(fields)),
            Struct(fields) => format!(" of {}",self.record_format(fields))
        }
    }

    fn enum_format(&self, formats: &BTreeMap<u32, Named<VariantFormat>>) -> String {
        formats.iter().map(|(_, f)| format!("  | {}{}", f.name, self.variant_format(&f.value))).collect::<Vec<_>>().join("\n")
    }

    fn serialize(&self) -> String {
        if self.generator.config.serialization { " [@@deriving serde]".to_string() }
        else { "".to_string() }
    }

    fn output_type(&mut self, name: &str, s: String, variant: bool) -> Result<()> {
        let sep = if variant { "\n".to_string() } else { "".to_string() };
        writeln!(self.out, "type {} = {}{}{}{}\n", name.to_snake_case(), sep, s, sep, self.serialize())
    }

    fn output_container(&mut self, name: &str, format: &ContainerFormat) -> Result<()> {
        use ContainerFormat::*;
        match format {
            UnitStruct => self.output_type(name, "unit".to_string(), false),
            NewTypeStruct(format) => self.output_type(name, self.format(format.as_ref()), false),
            TupleStruct(formats) => self.output_type(name, self.tuple_format(formats), false),
            Struct(fields) => self.output_type(name, self.record_format(fields), false),
            Enum(variants) => self.output_type(name, self.enum_format(variants), true),
        }
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
        let dune_source_path = dir_path.join("dune");
        let mut dune_file = std::fs::File::create(dune_source_path)?;
        let name = config.module_name.to_snake_case();
        writeln!(dune_file, "(library\n (name {0})\n (modules {0})\n (preprocess (pps ppx)))", name)?;
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
        self.install_runtime(include_directory!("runtime/ocaml/bincode"), "bincode")
    }

    fn install_bcs_runtime(&self) -> std::result::Result<(), Self::Error> {
        self.install_runtime(include_directory!("runtime/ocaml/common"), "common")?;
        self.install_runtime(include_directory!("runtime/ocaml/virtual"), "virtual")?;
        self.install_runtime(include_directory!("runtime/ocaml/ppx"), "ppx")?;
        self.install_runtime(include_directory!("runtime/ocaml/bcs"), "bcs")
    }
}

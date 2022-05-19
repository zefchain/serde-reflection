// Copyright (c) Facebook, Inc. and its affiliates
// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # Serde code generator
//!
//! '''bash
//! cargo run --bin serdegen -- --help
//! '''

use serde_generate::{
    cpp, csharp, dart, golang, java, ocaml, python3, rust, swift, typescript, CodeGeneratorConfig,
    Encoding, SourceInstaller,
};
use serde_reflection::Registry;
use std::path::PathBuf;
use structopt::{clap::arg_enum, StructOpt};

arg_enum! {
#[derive(Debug, StructOpt)]
enum Language {
    Python3,
    Cpp,
    Rust,
    Java,
    Go,
    Dart,
    TypeScript,
    CSharp,
    Swift,
    OCaml,
}
}

arg_enum! {
#[derive(Debug, StructOpt, PartialEq, Eq, PartialOrd, Ord)]
enum Runtime {
    Serde,
    Bincode,
    Bcs,
}
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Serde code generator",
    about = "Generate code for Serde containers"
)]
struct Options {
    /// Path to the YAML-encoded Serde formats.
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    /// Language for code generation.
    #[structopt(long, possible_values = &Language::variants(), case_insensitive = true, default_value = "Python3")]
    language: Language,

    /// Directory where to write generated modules (otherwise print code on stdout).
    #[structopt(long)]
    target_source_dir: Option<PathBuf>,

    /// Optional runtimes to install in the `target_source_dir` (if applicable).
    /// Also triggers the generation of specialized methods for each runtime.
    #[structopt(long, possible_values = &Runtime::variants(), case_insensitive = true)]
    with_runtimes: Vec<Runtime>,

    /// Module name for the Serde formats installed in the `target_source_dir`.
    /// Rust crates may contain a version number separated with a colon, e.g. "test:1.2.0".
    /// (By default, the installer will use version "0.1.0".)
    #[structopt(long)]
    module_name: Option<String>,

    /// Optional package name (Python) or module path (Go) where to find Serde runtime dependencies.
    #[structopt(long)]
    serde_package_name: Option<String>,

    /// Translate enums without variant data (c-style enums) into their equivalent in the target language,
    /// if the target language and the generator code support them.
    #[structopt(long)]
    use_c_style_enums: bool,
}

fn get_codegen_config<'a, I>(name: String, runtimes: I, c_style_enums: bool) -> CodeGeneratorConfig
where
    I: IntoIterator<Item = &'a Runtime>,
{
    let mut encodings = Vec::new();
    for runtime in runtimes {
        match runtime {
            Runtime::Bincode => {
                encodings.push(Encoding::Bincode);
            }
            Runtime::Bcs => {
                encodings.push(Encoding::Bcs);
            }
            Runtime::Serde => (),
        }
    }
    CodeGeneratorConfig::new(name)
        .with_encodings(encodings)
        .with_c_style_enums(c_style_enums)
}

fn main() {
    let options = Options::from_args();
    let serde_package_name_opt = options.serde_package_name.clone();
    let named_registry_opt = match &options.input {
        None => None,
        Some(input) => {
            let name = options.module_name.clone().unwrap_or_else(|| {
                input
                    .file_stem()
                    .expect("failed to deduce module name from input path")
                    .to_string_lossy()
                    .into_owned()
            });
            let content = std::fs::read_to_string(input).expect("input file must be readable");
            let registry = serde_yaml::from_str::<Registry>(content.as_str()).unwrap();
            Some((registry, name))
        }
    };
    let runtimes: std::collections::BTreeSet<_> = options.with_runtimes.into_iter().collect();

    match options.target_source_dir {
        None => {
            if let Some((registry, name)) = named_registry_opt {
                let config = get_codegen_config(name, &runtimes, options.use_c_style_enums);

                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                match options.language {
                    Language::Python3 => python3::CodeGenerator::new(&config)
                        .with_serde_package_name(serde_package_name_opt)
                        .output(&mut out, &registry)
                        .unwrap(),
                    Language::Rust => rust::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    Language::Cpp => cpp::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    Language::Go => golang::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    Language::Java => {
                        panic!("Code generation in Java requires `--target-source-dir`")
                    }
                    Language::Dart => {
                        panic!("Code generation in Dart requires `--target-source-dir`")
                    }
                    Language::TypeScript => typescript::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    Language::CSharp => {
                        panic!("Code generation in C# requires `--target-source-dir`")
                    }
                    Language::Swift => swift::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    Language::OCaml => ocaml::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                }
            }
        }

        Some(install_dir) => {
            let installer: Box<dyn SourceInstaller<Error = Box<dyn std::error::Error>>> =
                match options.language {
                    Language::Python3 => {
                        Box::new(python3::Installer::new(install_dir, serde_package_name_opt))
                    }
                    Language::Rust => Box::new(rust::Installer::new(install_dir)),
                    Language::Cpp => Box::new(cpp::Installer::new(install_dir)),
                    Language::Java => Box::new(java::Installer::new(install_dir)),
                    Language::Go => {
                        Box::new(golang::Installer::new(install_dir, serde_package_name_opt))
                    }
                    Language::Dart => Box::new(dart::Installer::new(install_dir)),
                    Language::TypeScript => Box::new(typescript::Installer::new(install_dir)),
                    Language::CSharp => Box::new(csharp::Installer::new(install_dir)),
                    Language::Swift => Box::new(swift::Installer::new(install_dir)),
                    Language::OCaml => Box::new(ocaml::Installer::new(install_dir)),
                };

            if let Some((registry, name)) = named_registry_opt {
                let config = get_codegen_config(name, &runtimes, options.use_c_style_enums);
                installer.install_module(&config, &registry).unwrap();
            }

            for runtime in runtimes {
                match runtime {
                    Runtime::Serde => installer.install_serde_runtime().unwrap(),
                    Runtime::Bincode => installer.install_bincode_runtime().unwrap(),
                    Runtime::Bcs => installer.install_bcs_runtime().unwrap(),
                }
            }
        }
    }
}

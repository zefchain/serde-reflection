// Copyright (c) Facebook, Inc. and its affiliates
// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # Serde code generator
//!
//! '''bash
//! cargo run -- --help
//! '''

use clap::{Parser, ValueEnum};
#[cfg(feature = "cpp")]
use serde_generate::cpp;
#[cfg(feature = "csharp")]
use serde_generate::csharp;
#[cfg(feature = "dart")]
use serde_generate::dart;
#[cfg(feature = "golang")]
use serde_generate::golang;
#[cfg(feature = "java")]
use serde_generate::java;
#[cfg(feature = "kotlin")]
use serde_generate::kotlin;
#[cfg(feature = "ocaml")]
use serde_generate::ocaml;
#[cfg(feature = "python3")]
use serde_generate::python3;
#[cfg(feature = "rust")]
use serde_generate::rust;
#[cfg(feature = "solidity")]
use serde_generate::solidity;
#[cfg(feature = "swift")]
use serde_generate::swift;
#[cfg(feature = "typescript")]
use serde_generate::typescript;
use serde_generate::{CodeGeneratorConfig, Encoding, SourceInstaller};
use serde_reflection::Registry;
use std::path::PathBuf;

#[derive(Clone, Debug, ValueEnum)]
enum Language {
    Python3,
    Cpp,
    Solidity,
    Rust,
    Java,
    Go,
    Dart,
    #[value(name = "typescript")]
    TypeScript,
    #[value(name = "csharp")]
    CSharp,
    Swift,
    #[value(name = "ocaml")]
    OCaml,
    Kotlin,
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
enum Runtime {
    Serde,
    Bincode,
    Bcs,
}

#[derive(Debug, Parser)]
#[command(
    name = "Serde code generator",
    about = "Generate code for Serde containers"
)]
struct Options {
    /// Path to the YAML-encoded Serde formats.
    input: Option<PathBuf>,

    /// Language for code generation.
    #[arg(long, value_enum, ignore_case = true, default_value = "python3")]
    language: Language,

    /// Directory where to write generated modules (otherwise print code on stdout).
    #[arg(long)]
    target_source_dir: Option<PathBuf>,

    /// Optional runtimes to install in the `target_source_dir` (if applicable).
    /// Also triggers the generation of specialized methods for each runtime.
    #[arg(long, value_enum, ignore_case = true, num_args = 1..)]
    with_runtimes: Vec<Runtime>,

    /// Module name for the Serde formats installed in the `target_source_dir`.
    /// Rust crates may contain a version number separated with a colon, e.g. "test:1.2.0".
    /// (By default, the installer will use version "0.1.0".)
    #[arg(long)]
    module_name: Option<String>,

    /// Optional package name (Python) or module path (Go) where to find Serde runtime dependencies.
    #[arg(long)]
    #[cfg_attr(not(any(feature = "python3", feature = "golang")), allow(dead_code))]
    serde_package_name: Option<String>,

    /// Translate enums without variant data (c-style enums) into their equivalent in the target language,
    /// if the target language and the generator code support them.
    #[arg(long)]
    use_c_style_enums: bool,

    /// Avoid creating a package spec file defining dependencies for the chosen language.
    /// Takes effect only for languages that have a package manifest format.
    #[arg(long)]
    skip_package_manifest: bool,
}

fn get_codegen_config<'a, I>(
    name: String,
    runtimes: I,
    c_style_enums: bool,
    package_manifest: bool,
) -> CodeGeneratorConfig
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
        .with_package_manifest(package_manifest)
}

#[allow(unused_macros)]
macro_rules! require_feature {
    ($feature:expr, $language:expr) => {
        panic!(
            "Language {} requires the `{}` feature to be enabled",
            $language, $feature
        )
    };
}

fn main() {
    let options = Options::parse();
    #[cfg(any(feature = "python3", feature = "golang"))]
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
        None =>
        {
            #[allow(unused_variables, unused_mut)]
            if let Some((registry, name)) = named_registry_opt {
                let config = get_codegen_config(
                    name,
                    &runtimes,
                    options.use_c_style_enums,
                    !options.skip_package_manifest,
                );

                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                match options.language {
                    #[cfg(feature = "python3")]
                    Language::Python3 => python3::CodeGenerator::new(&config)
                        .with_serde_package_name(serde_package_name_opt)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "python3"))]
                    Language::Python3 => require_feature!("python3", "Python3"),
                    #[cfg(feature = "rust")]
                    Language::Rust => rust::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "rust"))]
                    Language::Rust => require_feature!("rust", "Rust"),
                    #[cfg(feature = "cpp")]
                    Language::Cpp => cpp::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "cpp"))]
                    Language::Cpp => require_feature!("cpp", "Cpp"),
                    #[cfg(feature = "solidity")]
                    Language::Solidity => solidity::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "solidity"))]
                    Language::Solidity => require_feature!("solidity", "Solidity"),
                    #[cfg(feature = "golang")]
                    Language::Go => golang::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "golang"))]
                    Language::Go => require_feature!("golang", "Go"),
                    Language::Java => {
                        panic!("Code generation in Java requires `--target-source-dir`")
                    }
                    Language::Dart => {
                        panic!("Code generation in Dart requires `--target-source-dir`")
                    }
                    #[cfg(feature = "typescript")]
                    Language::TypeScript => typescript::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "typescript"))]
                    Language::TypeScript => require_feature!("typescript", "TypeScript"),
                    Language::CSharp => {
                        panic!("Code generation in C# requires `--target-source-dir`")
                    }
                    #[cfg(feature = "swift")]
                    Language::Swift => swift::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "swift"))]
                    Language::Swift => require_feature!("swift", "Swift"),
                    #[cfg(feature = "ocaml")]
                    Language::OCaml => ocaml::CodeGenerator::new(&config)
                        .output(&mut out, &registry)
                        .unwrap(),
                    #[cfg(not(feature = "ocaml"))]
                    Language::OCaml => require_feature!("ocaml", "OCaml"),
                    Language::Kotlin => {
                        panic!("Code generation in Kotlin requires `--target-source-dir`")
                    }
                }
            }
        }

        #[allow(unused_variables, unreachable_code)]
        Some(install_dir) => {
            let installer: Box<dyn SourceInstaller<Error = Box<dyn std::error::Error>>> =
                match options.language {
                    #[cfg(feature = "python3")]
                    Language::Python3 => {
                        Box::new(python3::Installer::new(install_dir, serde_package_name_opt))
                    }
                    #[cfg(not(feature = "python3"))]
                    Language::Python3 => require_feature!("python3", "Python3"),
                    #[cfg(feature = "rust")]
                    Language::Rust => Box::new(rust::Installer::new(install_dir)),
                    #[cfg(not(feature = "rust"))]
                    Language::Rust => require_feature!("rust", "Rust"),
                    #[cfg(feature = "cpp")]
                    Language::Cpp => Box::new(cpp::Installer::new(install_dir)),
                    #[cfg(not(feature = "cpp"))]
                    Language::Cpp => require_feature!("cpp", "Cpp"),
                    #[cfg(feature = "solidity")]
                    Language::Solidity => Box::new(solidity::Installer::new(install_dir)),
                    #[cfg(not(feature = "solidity"))]
                    Language::Solidity => require_feature!("solidity", "Solidity"),
                    #[cfg(feature = "java")]
                    Language::Java => Box::new(java::Installer::new(install_dir)),
                    #[cfg(not(feature = "java"))]
                    Language::Java => require_feature!("java", "Java"),
                    #[cfg(feature = "golang")]
                    Language::Go => {
                        Box::new(golang::Installer::new(install_dir, serde_package_name_opt))
                    }
                    #[cfg(not(feature = "golang"))]
                    Language::Go => require_feature!("golang", "Go"),
                    #[cfg(feature = "dart")]
                    Language::Dart => Box::new(dart::Installer::new(install_dir)),
                    #[cfg(not(feature = "dart"))]
                    Language::Dart => require_feature!("dart", "Dart"),
                    #[cfg(feature = "typescript")]
                    Language::TypeScript => Box::new(typescript::Installer::new(install_dir)),
                    #[cfg(not(feature = "typescript"))]
                    Language::TypeScript => require_feature!("typescript", "TypeScript"),
                    #[cfg(feature = "csharp")]
                    Language::CSharp => Box::new(csharp::Installer::new(install_dir)),
                    #[cfg(not(feature = "csharp"))]
                    Language::CSharp => require_feature!("csharp", "CSharp"),
                    #[cfg(feature = "swift")]
                    Language::Swift => Box::new(swift::Installer::new(install_dir)),
                    #[cfg(not(feature = "swift"))]
                    Language::Swift => require_feature!("swift", "Swift"),
                    #[cfg(feature = "ocaml")]
                    Language::OCaml => Box::new(ocaml::Installer::new(install_dir)),
                    #[cfg(not(feature = "ocaml"))]
                    Language::OCaml => require_feature!("ocaml", "OCaml"),
                    #[cfg(feature = "kotlin")]
                    Language::Kotlin => Box::new(kotlin::Installer::new(install_dir)),
                    #[cfg(not(feature = "kotlin"))]
                    Language::Kotlin => require_feature!("kotlin", "Kotlin"),
                };

            if let Some((registry, name)) = named_registry_opt {
                let config = get_codegen_config(
                    name,
                    &runtimes,
                    options.use_c_style_enums,
                    !options.skip_package_manifest,
                );
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

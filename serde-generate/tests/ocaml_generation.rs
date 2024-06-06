// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{ocaml, CodeGeneratorConfig, Encoding, SourceInstaller};
use std::{fs::File, io::Write, process::Command};
use tempfile::{tempdir, TempDir};

fn test_that_ocaml_code_compiles_with_config(
    config: &CodeGeneratorConfig,
    must_fail: bool,
    more: Option<&str>,
    encoding: Option<Encoding>,
) -> (TempDir, std::path::PathBuf) {
    let registry = test_utils::get_registry().unwrap();
    let dir0 = tempdir().unwrap();
    let dir = dir0.path();
    std::fs::create_dir_all(dir).unwrap();

    let source_path = dir.join("test.ml");
    let mut source = File::create(&source_path).unwrap();

    if let Some(s) = more {
        writeln!(source, "{}", s).unwrap()
    };

    let generator = ocaml::CodeGenerator::new(config);
    generator.output(&mut source, &registry).unwrap();

    let installer = ocaml::Installer::new(dir.to_path_buf());
    let runtime_str = match encoding {
        Some(Encoding::Bcs) => {
            installer.install_bcs_runtime().unwrap();
            "\n(libraries bcs_runtime)"
        }
        Some(Encoding::Bincode) => {
            installer.install_bincode_runtime().unwrap();
            "\n(libraries bincode_runtime)"
        }
        None => {
            installer.install_serde_runtime().unwrap();
            ""
        }
    };

    let dune_project_source_path = dir.join("dune-project");
    let mut dune_project_file = File::create(dune_project_source_path).unwrap();
    write!(dune_project_file, "(lang dune 3.0)").unwrap();

    let dune_source_path = dir.join("dune");
    let mut dune_file = File::create(dune_source_path).unwrap();

    write!(
        dune_file,
        r#"
(env (_ (flags (:standard -w -30-42 -warn-error -a))))

(library
 (name test)
 (modules test)
 (preprocess (pps ppx)){})"#,
        runtime_str
    )
    .unwrap();

    let status = Command::new("dune")
        .arg("build")
        .arg("--root")
        .arg(dir)
        .status()
        .unwrap();
    if must_fail {
        assert!(!status.success())
    } else {
        assert!(status.success())
    }

    (dir0, source_path)
}

#[test]
fn test_that_ocaml_code_compiles() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    test_that_ocaml_code_compiles_with_config(&config, false, None, None);
}

#[test]
fn test_that_ocaml_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_that_ocaml_code_compiles_with_config(&config, false, None, Some(Encoding::Bcs));
}

#[test]
fn test_that_ocaml_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    test_that_ocaml_code_compiles_with_config(&config, false, None, Some(Encoding::Bincode));
}

#[test]
fn test_that_ocaml_code_compiles_with_comments() {
    let comments = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        "Some\ncomments".to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_serialization(false)
        .with_comments(comments);
    let (_dir, source_path) = test_that_ocaml_code_compiles_with_config(&config, false, None, None);
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("(*\n  Some\n  comments\n*)\n"));
}

#[test]
fn test_ocaml_code_with_external_definitions() {
    let definitions = vec![
        ("foo".to_string(), vec!["Map".to_string()]),
        (String::new(), vec!["Bytes".into()]),
    ]
    .into_iter()
    .collect();

    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_external_definitions(definitions)
        .with_serialization(false);
    test_that_ocaml_code_compiles_with_config(&config, true, None, None);

    let more = r#"
type bytes = Stdint.uint8

module Foo = struct
    type ('k, 'v) map = ('k, 'v) Serde.map
end
"#;

    test_that_ocaml_code_compiles_with_config(&config, false, Some(more), None);
}

#[test]
fn test_that_ocaml_code_compiles_with_custom_code() {
    let custom_code = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        r#"let serde_data_to_string = function
  | SerdeData_PrimitiveTypes _ -> "primitive types"
  | SerdeData_OtherTypes _ -> "other types"
  | SerdeData_UnitVariant -> "unit variant"
  | SerdeData_NewTypeVariant _ -> "new type variant"
  | _ -> "etc""#
            .to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_serialization(false)
        .with_custom_code(custom_code);
    let (_dir, source_path) = test_that_ocaml_code_compiles_with_config(&config, false, None, None);
    let content = std::fs::read_to_string(source_path).unwrap();
    assert!(content.contains("serde_data_to_string"));
}

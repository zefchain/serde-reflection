// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{ocaml, CodeGeneratorConfig, SourceInstaller};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "\"{}\"",
        bytes
            .iter()
            .map(|x| format!("\\{:03}", x))
            .collect::<Vec<_>>()
            .join("")
    )
}

#[test]
fn test_ocaml_bcs_runtime_on_simple_data() {
    test_ocaml_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_ocaml_bincode_runtime_on_simple_data() {
    test_ocaml_runtime_on_simple_data(Runtime::Bincode);
}

fn test_ocaml_runtime_on_simple_data(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir0 = tempdir().unwrap();
    let dir = dir0.path();
    let installer = ocaml::Installer::new(dir.to_path_buf());
    let runtime_str = match runtime {
        Runtime::Bcs => {
            installer.install_bcs_runtime().unwrap();
            "bcs"
        }
        Runtime::Bincode => {
            installer.install_bincode_runtime().unwrap();
            "bincode"
        }
    };

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);

    let dir_path = dir.join(config.module_name());
    std::fs::create_dir_all(&dir_path).unwrap();

    let dune_project_source_path = dir.join("dune-project");
    let mut dune_project_file = std::fs::File::create(dune_project_source_path).unwrap();
    writeln!(dune_project_file, "(lang dune 3.0)").unwrap();

    let dune_source_path = dir_path.join("dune");
    let mut dune_file = std::fs::File::create(dune_source_path).unwrap();

    writeln!(
        dune_file,
        r#"
(env (_ (flags (:standard -w -30-42))))

(library
 (name testing)
 (modules testing)
 (preprocess (pps ppx))
 (libraries {}_runtime))

(executable
 (name main)
 (modules main)
 (libraries serde testing))
"#,
        runtime_str
    )
    .unwrap();

    let lib_path = dir_path.join("testing.ml");
    let mut lib = File::create(lib_path).unwrap();
    let generator = ocaml::CodeGenerator::new(&config);
    generator.output(&mut lib, &registry).unwrap();

    let exe_path = dir_path.join("main.ml");
    let mut exe = File::create(exe_path).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    let reference_bytes = quote_bytes(&reference);

    writeln!(
        exe,
        r#"
open Serde
open Stdint

exception Unexpected_success

let () =
  let input = Bytes.of_string {0} in
  let value = Deserialize.apply Testing.test_de input in
  let a = List.map Uint32.of_int [4; 6] in
  let b = -3L, Uint64.of_int 5 in
  let c = Testing.Choice_C {{ x = Uint8.of_int 7 }} in
  let value2 = {{Testing.a; b; c}} in
  assert (value = value2);
  let output = Serialize.apply Testing.test_ser value2 in
  assert (input = output);
  let input2 = Bytes.of_string ({0} ^ "\001") in
  try
    let _ = Deserialize.apply Testing.test_de input2 in
    raise Unexpected_success
  with
  | Unexpected_success -> assert false
  | _ -> ()
"#,
        reference_bytes
    )
    .unwrap();

    let status = Command::new("dune")
        .arg("exec")
        .arg("testing/main.exe")
        .arg("--root")
        .arg(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_ocaml_bcs_runtime_on_supported_types() {
    test_ocaml_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_ocaml_bincode_runtime_on_supported_types() {
    test_ocaml_runtime_on_supported_types(Runtime::Bincode);
}

fn test_ocaml_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir0 = tempdir().unwrap();
    let dir = dir0.path();
    let installer = ocaml::Installer::new(dir.to_path_buf());
    let runtime_str = match runtime {
        Runtime::Bcs => {
            installer.install_bcs_runtime().unwrap();
            "bcs"
        }
        Runtime::Bincode => {
            installer.install_bincode_runtime().unwrap();
            "bincode"
        }
    };

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);

    let dir_path = dir.join(config.module_name());
    std::fs::create_dir_all(&dir_path).unwrap();

    let dune_project_source_path = dir.join("dune-project");
    let mut dune_project_file = std::fs::File::create(dune_project_source_path).unwrap();
    writeln!(dune_project_file, "(lang dune 3.0)").unwrap();

    let dune_source_path = dir_path.join("dune");
    let mut dune_file = std::fs::File::create(dune_source_path).unwrap();

    writeln!(
        dune_file,
        r#"
(env (_ (flags (:standard -w -30-42))))

(executable
 (name test)
 (modules test)
 (preprocess (pps ppx)) 
 (libraries {}_runtime))
"#,
        runtime_str
    )
    .unwrap();

    let source_path = dir_path.join("test.ml");
    println!("{:?}", source_path);
    let mut source = File::create(&source_path).unwrap();
    let generator = ocaml::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let positive_encodings: Vec<_> = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect();

    let negative_encodings: Vec<_> = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect();

    writeln!(
        source,
        r#"
open Serde 

exception Unexpected_success

let () = 
  List.iter (fun s ->
      let b = Bytes.of_string s in
      let sd = Deserialize.apply serde_data_de b in
      let b2 = Serialize.apply serde_data_ser sd in
      assert (b = b2)) [{}];

  List.iter (fun s ->
      let b = Bytes.of_string s in
      try 
        let _ = Deserialize.apply serde_data_de b in
        raise Unexpected_success
      with 
      | Unexpected_success -> assert false
      | _ -> ()) [{}]
"#,
        positive_encodings.join("; "),
        negative_encodings.join("; ")
    )
    .unwrap();

    let status = Command::new("dune")
        .arg("exec")
        .arg("testing/test.exe")
        .arg("--root")
        .arg(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

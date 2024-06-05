// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use regex::Regex;
use serde_generate::{typescript, CodeGeneratorConfig, Encoding, SourceInstaller};
use std::{
    collections::BTreeMap,
    fs::File,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::tempdir;

fn test_typescript_code_compiles_with_config(
    dir_path: &Path,
    config: &CodeGeneratorConfig,
) -> std::path::PathBuf {
    let registry = test_utils::get_registry().unwrap();
    make_output_file(dir_path);

    let installer = typescript::Installer::new(dir_path.to_path_buf());
    installer.install_serde_runtime().unwrap();
    assert_deno_info(dir_path.join("serde/mod.ts").as_path());

    installer.install_bcs_runtime().unwrap();
    assert_deno_info(dir_path.join("bcs/mod.ts").as_path());

    let source_path = dir_path.join("testing").join("test.ts");
    let mut source = File::create(&source_path).unwrap();

    let generator = typescript::CodeGenerator::new(config);
    generator.output(&mut source, &registry).unwrap();

    assert_deno_info(&source_path);
    dir_path.join("testing")
}

fn assert_deno_info(ts_path: &Path) {
    let output = Command::new("deno")
        .arg("info")
        .arg(ts_path)
        .stderr(Stdio::inherit())
        .output()
        .expect("deno info failed, is deno installed? brew install deno");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !is_error_output(stdout.as_str()),
        "deno info detected an error\n{}",
        stdout
    );
}

fn is_error_output(output: &str) -> bool {
    let re = Regex::new(r"\berror\b").unwrap();
    re.is_match(output)
}

fn make_output_file(dir: &Path) {
    std::fs::create_dir_all(dir.join("testing")).unwrap_or(());
}

#[test]
fn test_is_error_output() {
    let table = vec![
        ("file:///var/folders/l0/x592_pjj18n6r2m0nqn05vmc0000gn/T/.tmp5NPlE2/serde/mod.ts (176B)", false),
        ("https://deno.land/std@0.85.0/node/_errors.ts (60.89KB)", false),
        ("error: Cannot resolve module \"file:///var/folders/l0/x592_pjj18n6r2m0nqn05vmc0000gn/T/.tmp5NPlE2/bcs/mod.ts\"", true),
        ("file:///var/folders/l0/x592_pjj18n6r2m0nqn05vmc0000gn/T/.tmpG1an6c/something/noSerializer.ts (error)", true),
    ];

    for (input, expectation) in table {
        assert_eq!(is_error_output(input), expectation);
    }
}

#[test]
fn test_typescript_code_compiles_with_bcs() {
    let dir = tempdir().unwrap();
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    test_typescript_code_compiles_with_config(dir.path(), &config);
}

#[test]
fn test_typescript_code_compiles_with_comments() {
    let dir = tempdir().unwrap();
    let comments = vec![(vec!["SerdeData".to_string()], "Some\ncomments".to_string())]
        .into_iter()
        .collect();
    let config = CodeGeneratorConfig::new("testing".to_string()).with_comments(comments);

    let path = test_typescript_code_compiles_with_config(dir.path(), &config);
    // Comment was correctly generated.
    let content = std::fs::read_to_string(path.join("test.ts")).unwrap();
    assert!(content.contains(
        r#"
/**
 * Some
 * comments
 */
"#
    ));
}

#[test]
fn test_typescript_code_compiles_with_external_definitions() {
    let dir = tempdir().unwrap();

    // create external definition
    std::fs::create_dir_all(dir.path().join("external")).unwrap_or(());
    std::fs::write(
        dir.path().join("external/mod.ts"),
        "export const CustomType = 5;",
    )
    .unwrap();

    let mut external_definitions = BTreeMap::new();
    external_definitions.insert(String::from("external"), vec![String::from("CustomType")]);
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_external_definitions(external_definitions);

    test_typescript_code_compiles_with_config(dir.path(), &config);
}

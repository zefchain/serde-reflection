// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use heck::CamelCase;
use serde_generate::{typescript, CodeGeneratorConfig, SourceInstaller};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

#[test]
fn test_typescript_runtime_bcs_serialization() {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();
    let dir_path = dir.path();
    std::fs::create_dir_all(dir_path.join("tests")).unwrap();

    let installer = typescript::Installer::new(dir_path.to_path_buf());
    installer.install_serde_runtime().unwrap();
    installer.install_bcs_runtime().unwrap();

    let source_path = dir_path.join("tests/test.ts");
    let mut source = File::create(&source_path).unwrap();

    let runtime = Runtime::Bcs;
    let config = CodeGeneratorConfig::new("main".to_string()).with_encodings(vec![runtime.into()]);
    let generator = typescript::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    writeln!(
        source,
        r#"
import {{ assertEquals }} from "https://deno.land/std@0.110.0/testing/asserts.ts";
Deno.test("{1} serialization matches deserialization", () => {{
	const expectedBytes = new Uint8Array([{0}]);
  const {1}Deserializer: {2}Deserializer = new {2}Deserializer(expectedBytes);
  const deserializedInstance: Test = Test.deserialize({1}Deserializer);

  const expectedInstance: Test = new Test(
		[4, 6],
    [BigInt(-3), BigInt(5)],
		new ChoiceVariantC(7),
	);

  assertEquals(deserializedInstance, expectedInstance, "Object instances should match");

  const {1}Serializer = new {2}Serializer();
	expectedInstance.serialize({1}Serializer);
  const serializedBytes = {1}Serializer.getBytes();

  assertEquals(serializedBytes, expectedBytes, "{1} bytes should match");
}});
"#,
        reference
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<_>>()
            .join(", "),
        runtime.name().to_lowercase(),
        runtime.name().to_camel_case(),
    )
    .unwrap();

    let status = Command::new("deno")
        .current_dir(dir_path)
        .arg("test")
        .arg(&source_path)
        .status()
        .unwrap();
    assert!(status.success());
}

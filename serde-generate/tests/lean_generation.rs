// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use serde_generate::{lean, CodeGeneratorConfig, Encoding, SourceInstaller};
use serde_reflection::Registry;
use std::process::Command;
use tempfile::{tempdir, TempDir};

/// Install the module `testing` and all runtimes, then build the module with Lake.
/// Optionally add a module `Foo` to the package of the generated code.
fn test_that_lean_code_compiles_with_config(
    config: &CodeGeneratorConfig,
    foo_module: Option<&str>,
) -> (TempDir, String) {
    let registry = test_utils::get_registry().unwrap();
    test_that_lean_code_compiles_with_registry(config, &registry, foo_module)
}

fn test_that_lean_code_compiles_with_registry(
    config: &CodeGeneratorConfig,
    registry: &Registry,
    foo_module: Option<&str>,
) -> (TempDir, String) {
    let dir = tempdir().unwrap();
    let installer = lean::Installer::new(dir.path().to_path_buf());
    installer.install_bcs_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    installer.install_module(config, registry).unwrap();

    let module_dir = dir.path().join(config.module_name());
    if let Some(source) = foo_module {
        std::fs::write(module_dir.join("Foo.lean"), source).unwrap();
        let lakefile = module_dir.join("lakefile.toml");
        let content = std::fs::read_to_string(&lakefile).unwrap();
        std::fs::write(
            lakefile,
            content + "\n[[lean_lib]]\nname = \"Foo\"\nroots = [\"Foo\"]\n",
        )
        .unwrap();
    }

    let status = Command::new("lake")
        .arg("build")
        .current_dir(&module_dir)
        .status()
        .unwrap();
    assert!(status.success());

    let source = std::fs::read_to_string(module_dir.join(format!(
        "{}.lean",
        lean::lean_module_name(config.module_name()).replace('.', "/")
    )))
    .unwrap();
    (dir, source)
}

#[test]
fn test_that_lean_code_compiles() {
    let config = CodeGeneratorConfig::new("testing".to_string()).with_serialization(false);
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, None);
    assert!(!source.contains("Serialize"));
}

#[test]
fn test_that_lean_code_compiles_with_bcs() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bcs]);
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, None);
    assert!(source.contains("instance : Bcs.Serialize SerdeData"));
    assert!(!source.contains("Bincode"));
    // Recursive types are defined by recursion on the depth budget.
    assert!(!source.contains("partial def"));
    // All types come with proofs of correctness.
    assert!(source.contains("instance : Bcs.LawfulSerialize SerdeData"));
    assert!(source.contains("instance : Bcs.LawfulSerialize List"));
    // Types without maps also come with proofs of injectivity.
    assert!(source.contains("instance : Bcs.InjectiveSerialize SimpleList"));
    assert!(!source.contains("instance : Bcs.InjectiveSerialize List"));
    assert!(source.contains("instance : Bcs.InjectiveSerialize Struct"));
    assert!(!source.contains("instance : Bcs.InjectiveSerialize SerdeData"));
    // Types with maps come with proofs of injectivity up to equivalence.
    assert!(source.contains("instance : Bcs.EquivSerialize SerdeData"));
    assert!(source.contains("instance : Bcs.EquivSerialize List"));
    // Maps are represented by hash maps, when possible.
    assert!(
        source.contains("complexMap (value : Std.HashMap (Vector UInt32 2 × Vector UInt8 4) Unit)")
    );
}

#[test]
fn test_that_lean_code_compiles_with_bincode() {
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![Encoding::Bincode]);
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, None);
    assert!(source.contains("instance : Bincode.Serialize SerdeData"));
    assert!(!source.contains("Bcs"));
    // Recursive types are defined by recursion on the depth budget.
    assert!(!source.contains("partial def"));
    // Types without maps come with proofs of correctness and injectivity.
    assert!(source.contains("instance : Bincode.LawfulSerialize SimpleList"));
    assert!(source.contains("instance : Bincode.InjectiveSerialize SimpleList"));
    assert!(source.contains("instance : Bincode.LawfulSerialize Struct"));
    // Types with hash maps come with proofs of round trip up to equivalence.
    assert!(!source.contains("instance : Bincode.LawfulSerialize OtherTypes"));
    assert!(source.contains("instance : Bincode.EquivSerialize OtherTypes"));
    // There are no proofs for floats.
    assert!(!source.contains("instance : Bincode.LawfulSerialize PrimitiveTypes"));
    assert!(!source.contains("instance : Bincode.EquivSerialize SerdeData"));
}

#[test]
fn test_that_lean_code_compiles_with_bcs_and_bincode_in_submodule() {
    let config = CodeGeneratorConfig::new("testing.types".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode]);
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, None);
    assert!(source.contains("namespace Testing.Types"));
}

#[test]
fn test_that_lean_code_compiles_with_comments() {
    let comments = vec![
        (
            vec!["testing".to_string(), "SerdeData".to_string()],
            "Some\ncomments".to_string(),
        ),
        (
            vec!["testing".to_string(), "Struct".to_string(), "x".to_string()],
            "A field".to_string(),
        ),
        (
            vec![
                "testing".to_string(),
                "List".to_string(),
                "Node".to_string(),
            ],
            "A variant".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![Encoding::Bcs])
        .with_comments(comments);
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, None);
    assert!(source.contains("/--\nSome\ncomments\n-/\ninductive SerdeData where"));
    assert!(source.contains("  /--\n  A field\n  -/\n  x : UInt32"));
    assert!(source.contains("  /--\n  A variant\n  -/\n  | node"));
}

#[test]
fn test_lean_code_with_external_definitions() {
    // Replace the definitions of `Struct` and `CStyleEnum` with the ones of module `Foo`.
    let definitions = vec![(
        "foo".to_string(),
        vec!["Struct".to_string(), "CStyleEnum".to_string()],
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![Encoding::Bcs])
        .with_external_definitions(definitions);

    let foo = r#"
import Bcs

namespace Foo

structure Struct where
  x : UInt32
  y : UInt64
  deriving Repr, BEq, Inhabited

abbrev CStyleEnum := Bool

instance : Bcs.Serialize Struct := ⟨fun s => do Bcs.serialize s.x; Bcs.serialize s.y⟩
instance : Bcs.Deserialize Struct where
  deserialize := do return ⟨← Bcs.deserialize, ← Bcs.deserialize⟩

end Foo
"#;
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, Some(foo));
    assert!(source.contains("import Foo\n"));
    // No proofs are generated for types that depend on external definitions.
    assert!(!source.contains("instance : Bcs.LawfulSerialize SerdeData"));
    assert!(source.contains("instance : Bcs.LawfulSerialize UnitStruct"));
    assert!(source.contains("fOption : Option Foo.Struct"));
    assert!(source.contains("(Bcs.Ser.option (Bcs.serialize (α := Foo.Struct)))"));
}

#[test]
fn test_that_lean_code_compiles_with_custom_code() {
    let custom_code = vec![(
        vec!["testing".to_string(), "SerdeData".to_string()],
        r#"def SerdeData.isUnitVariant : SerdeData → Bool
  | .unitVariant => true
  | _ => false
"#
        .to_string(),
    )]
    .into_iter()
    .collect();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![Encoding::Bcs])
        .with_custom_code(custom_code);
    let (_dir, source) = test_that_lean_code_compiles_with_config(&config, None);
    assert!(source.contains("SerdeData.isUnitVariant"));
}

#[test]
fn test_that_lean_code_compiles_with_lean_keywords_and_reserved_names() {
    let registry: Registry = serde_yaml::from_str(
        r#"---
Type:
  STRUCT:
    - end: U8
    - mk: BOOL
    - fun:
        TUPLE: [U8, {TUPLE: [U16, U32]}, {OPTION: {TUPLE: [STR, BYTES]}}]
Rec:
  ENUM:
    0:
      Rec: UNIT
    1:
      Match:
        NEWTYPE:
          TYPENAME: Type
    2:
      Where:
        STRUCT:
          - instance: U64
          - nested:
              SEQ:
                TUPLEARRAY:
                  CONTENT:
                    TYPENAME: Rec
                  SIZE: 2
"#,
    )
    .unwrap();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode]);
    let (_dir, source) = test_that_lean_code_compiles_with_registry(&config, &registry, None);
    assert!(source.contains("structure «Type» where"));
    assert!(source.contains("  «end» : UInt8\n  mk_ : Bool\n"));
    assert!(source.contains("  «fun» : UInt8 × (UInt16 × UInt32) × Option (String × ByteArray)\n"));
    assert!(source.contains("  | rec_\n"));
    // `Vector` cannot be used in recursive definitions.
    assert!(source.contains("(nested : Array (Array Rec))"));
}

#[test]
fn test_lean_code_with_mutually_recursive_types() {
    // `P` only has values through the variant `Q.stop`, which Lean's `deriving Inhabited`
    // does not find.
    let registry: Registry = serde_yaml::from_str(
        r#"---
P:
  STRUCT:
    - q:
        TYPENAME: Q
Q:
  ENUM:
    0:
      P:
        NEWTYPE:
          TYPENAME: P
    1:
      Stop: UNIT
"#,
    )
    .unwrap();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode]);
    let (_dir, source) = test_that_lean_code_compiles_with_registry(&config, &registry, None);
    assert!(source.contains("instance : Inhabited Q := ⟨Q.stop⟩\ninstance : Inhabited P"));
    assert!(source.contains("instance : Bcs.InjectiveSerialize P"));
    assert!(source.contains("instance : Bincode.InjectiveSerialize P"));
}

#[test]
fn test_lean_code_with_maps_in_recursive_types() {
    // The values of `children` refer to `Node`, so it is an array of entries. The other maps
    // are hash maps.
    let registry: Registry = serde_yaml::from_str(
        r#"---
Key:
  STRUCT:
    - id: U32
    - name: STR
Node:
  ENUM:
    0:
      Leaf: UNIT
    1:
      Branch:
        STRUCT:
          - children:
              MAP:
                KEY: STR
                VALUE:
                  TYPENAME: Node
          - tags:
              MAP:
                KEY:
                  TYPENAME: Key
                VALUE:
                  MAP: {KEY: U8, VALUE: BOOL}
"#,
    )
    .unwrap();
    let config = CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![Encoding::Bcs, Encoding::Bincode]);
    let (_dir, source) = test_that_lean_code_compiles_with_registry(&config, &registry, None);
    assert!(source.contains(
        "(children : Array (String × Node)) (tags : Std.HashMap Key (Std.HashMap UInt8 Bool))"
    ));
    assert!(source.contains("structure Key where\n  id : UInt32\n  name : String\n  deriving Repr, BEq, Hashable, ReflBEq, LawfulBEq"));
    // The equivalence relations are shared by both encodings.
    assert_eq!(source.matches("def Node.serdeEquiv ").count(), 1);
    assert!(source.contains("instance : Bcs.LawfulSerialize Node"));
    assert!(source.contains("instance : Bcs.EquivSerialize Node"));
    assert!(!source.contains("instance : Bincode.LawfulSerialize Node"));
    assert!(source.contains("instance : Bincode.EquivSerialize Node"));
}

#[test]
fn test_lean_code_rejects_types_without_values() {
    let registry: Registry = serde_yaml::from_str(
        r#"---
S:
  NEWTYPESTRUCT:
    TYPENAME: S
A:
  STRUCT:
    - b:
        TYPENAME: B
B:
  STRUCT:
    - a:
        TYPENAME: A
Empty:
  ENUM: {}
Fine:
  STRUCT:
    - s:
        OPTION:
          TYPENAME: S
"#,
    )
    .unwrap();
    let config = CodeGeneratorConfig::new("testing".to_string());
    let mut out = Vec::new();
    let error = lean::CodeGenerator::new(&config)
        .output(&mut out, &registry)
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the following types have no values and cannot be deserialized: A, B, Empty, S"
    );
}

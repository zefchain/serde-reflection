// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{lean, CodeGeneratorConfig, SourceInstaller};
use std::{path::Path, process::Command};
use tempfile::tempdir;

/// Lean helpers shared by the test programs.
const HEX_HELPERS: &str = r#"
def hexDigit (c : Char) : UInt8 :=
  if c ≤ '9' then (c.toNat - '0'.toNat).toUInt8 else (c.toNat - 'a'.toNat + 10).toUInt8

def ofHex (s : String) : ByteArray := Id.run do
  let cs := s.toList.toArray
  let mut b := ByteArray.empty
  for i in [0:cs.size / 2] do
    b := b.push (hexDigit cs[2 * i]! * 16 + hexDigit cs[2 * i + 1]!)
  return b
"#;

/// Lean checks on the BCS proofs generated for the test registry.
const BCS_PROOF_CHECKS: &str = r#"
/-- info: 'Testing.SerdeData.bcsCorrect' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms SerdeData.bcsCorrect

/-- info: 'Testing.SimpleList.bcsInjective' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms SimpleList.bcsInjective

/-- info: 'Testing.SerdeData.bcsRoundTrip' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms SerdeData.bcsRoundTrip

example (b : ByteArray) (x : SerdeData) (h : Bcs.fromBytes b = .ok x) : Bcs.toBytes x = .ok b :=
  Bcs.toBytes_of_fromBytes h

example (b : ByteArray) (x y : SerdeData) (hx : Bcs.toBytes x = .ok b)
    (hy : Bcs.toBytes y = .ok b) : Bcs.Equivalent x y :=
  Bcs.equiv_of_toBytes hx hy

example (b : ByteArray) (x : SimpleList) : Bcs.fromBytes b = .ok x ↔ Bcs.toBytes x = .ok b :=
  Bcs.fromBytes_eq_ok_iff
"#;

/// Lean checks on the Bincode proofs generated for the test registry. There are no proofs for
/// `SerdeData`, which contains floats.
const BINCODE_PROOF_CHECKS: &str = r#"
/-- info: 'Testing.SimpleList.bincodeCorrect' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms SimpleList.bincodeCorrect

/-- info: 'Testing.OtherTypes.bincodeRoundTrip' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in #print axioms OtherTypes.bincodeRoundTrip

example (b : ByteArray) (x : OtherTypes) (h : Bincode.toBytes x = .ok b) :
    ∃ y, Bincode.fromBytes b = .ok y ∧ Bincode.Equivalent y x :=
  Bincode.fromBytes_of_toBytes_equiv h

example (b : ByteArray) (x : CStyleEnum) :
    Bincode.fromBytes b = .ok x ↔ Bincode.toBytes x = .ok b :=
  Bincode.fromBytes_eq_ok_iff
"#;

fn quote_hex(bytes: &[u8]) -> String {
    format!("\"{}\"", hex::encode(bytes))
}

fn runtime_namespace(runtime: Runtime) -> &'static str {
    match runtime {
        Runtime::Bcs => "Bcs",
        Runtime::Bincode => "Bincode",
    }
}

/// Install the runtime, then create a package `main` containing an executable built from
/// `main_source` and depending on the given packages. Return the status of `lake exe main`.
fn run_lean_main(dir: &Path, requires: &[&str], main_source: &str) -> std::process::ExitStatus {
    let main_dir = dir.join("main");
    std::fs::create_dir_all(&main_dir).unwrap();
    let mut lakefile = "name = \"main\"\ndefaultTargets = [\"main\"]\n".to_string();
    for name in requires {
        lakefile += &format!("\n[[require]]\nname = \"{name}\"\npath = \"../{name}\"\n");
    }
    lakefile += "\n[[lean_exe]]\nname = \"main\"\nroot = \"Main\"\n";
    std::fs::write(main_dir.join("lakefile.toml"), lakefile).unwrap();
    std::fs::write(
        main_dir.join("lean-toolchain"),
        format!("{}\n", lean::LEAN_TOOLCHAIN),
    )
    .unwrap();
    std::fs::write(main_dir.join("Main.lean"), main_source).unwrap();

    Command::new("lake")
        .arg("exe")
        .arg("main")
        .current_dir(&main_dir)
        .status()
        .unwrap()
}

fn install_runtime(installer: &lean::Installer, runtime: Runtime) {
    match runtime {
        Runtime::Bcs => installer.install_bcs_runtime().unwrap(),
        Runtime::Bincode => installer.install_bincode_runtime().unwrap(),
    }
}

#[test]
fn test_lean_bcs_runtime_on_simple_data() {
    test_lean_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_lean_bincode_runtime_on_simple_data() {
    test_lean_runtime_on_simple_data(Runtime::Bincode);
}

fn test_lean_runtime_on_simple_data(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();
    let installer = lean::Installer::new(dir.path().to_path_buf());
    install_runtime(&installer, runtime);
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    installer.install_module(&config, &registry).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });
    let ns = runtime_namespace(runtime);
    let reference = quote_hex(&reference);

    let main_source = format!(
        r#"
import Testing
open Testing
{HEX_HELPERS}
def main : IO UInt32 := do
  let input := ofHex {reference}
  let expected : Test := {{ a := #[4, 6], b := (-3, 5), c := .c 7 }}
  let .ok value := {ns}.fromBytes (α := Test) input
    | IO.eprintln "failed to deserialize"; return 1
  unless value == expected do
    IO.eprintln s!"unexpected value {{repr value}}"; return 1
  let .ok output := {ns}.toBytes value
    | IO.eprintln "failed to serialize"; return 1
  unless output == input do
    IO.eprintln "unexpected serialization"; return 1
  let .error _ := {ns}.fromBytes (α := Test) (input.push 1)
    | IO.eprintln "trailing bytes were accepted"; return 1
  -- A different value has a different serialization, of the same length.
  let .ok other := {ns}.toBytes {{ expected with b := (-3, 0) }}
    | IO.eprintln "failed to serialize"; return 1
  unless other.size == input.size && other != input do
    IO.eprintln "unexpected serialization of a different value"; return 1
  return 0
"#
    );
    let status = run_lean_main(dir.path(), &["testing"], &main_source);
    assert!(status.success());
}

#[test]
fn test_lean_bcs_runtime_on_supported_types() {
    test_lean_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_lean_bincode_runtime_on_supported_types() {
    test_lean_runtime_on_supported_types(Runtime::Bincode);
}

fn test_lean_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let installer = lean::Installer::new(dir.path().to_path_buf());
    install_runtime(&installer, runtime);
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    installer.install_module(&config, &registry).unwrap();

    let positive_encodings = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_hex(bytes))
        .collect::<Vec<_>>()
        .join(",\n  ");
    let negative_encodings = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_hex(bytes))
        .collect::<Vec<_>>()
        .join(",\n  ");
    let ns = runtime_namespace(runtime);
    let proofs = match runtime {
        // Check that the generated proofs only depend on the standard axioms, and that the
        // main theorems apply to the generated types.
        Runtime::Bcs => BCS_PROOF_CHECKS,
        Runtime::Bincode => BINCODE_PROOF_CHECKS,
    };

    let main_source = format!(
        r#"
import Testing
open Testing
{HEX_HELPERS}
{proofs}
def positiveEncodings : Array String := #[
  {positive_encodings}]

def negativeEncodings : Array String := #[
  {negative_encodings}]

def main : IO UInt32 := do
  let mut failures := 0
  for encoding in positiveEncodings do
    let input := ofHex encoding
    match {ns}.fromBytes (α := SerdeData) input with
    | .error e =>
      IO.eprintln s!"failed to deserialize {{encoding}}: {{e}}"
      failures := failures + 1
    | .ok value =>
      match {ns}.toBytes value with
      | .error e =>
        IO.eprintln s!"failed to serialize {{encoding}}: {{e}}"
        failures := failures + 1
      | .ok output =>
        if output != input then
          IO.eprintln s!"unexpected serialization of {{encoding}}"
          failures := failures + 1
      -- Values are equal to themselves.
      unless ({ns}.fromBytes (α := SerdeData) input).toOption == some value do
        IO.eprintln s!"value of {{encoding}} is not equal to itself"
        failures := failures + 1
      -- Simple mutations of the input are rejected, or give a different value.
      for i in [0:input.size] do
        let input' := input.set! i (input.get! i ^^^ 0x80)
        if let .ok value' := {ns}.fromBytes (α := SerdeData) input' then
          if value' == value then
            IO.eprintln s!"mutation of byte {{i}} of {{encoding}} gives the same value"
            failures := failures + 1
  for encoding in negativeEncodings do
    if let .ok _ := {ns}.fromBytes (α := SerdeData) (ofHex encoding) then
      IO.eprintln s!"unexpected success for {{encoding}}"
      failures := failures + 1
  IO.println s!"checked {{positiveEncodings.size}} positive and {{negativeEncodings.size}} negative samples"
  return if failures == 0 then 0 else 1
"#
    );
    let status = run_lean_main(dir.path(), &["testing"], &main_source);
    assert!(status.success());
}

#[test]
fn test_lean_runtime_unit_tests() {
    let dir = tempdir().unwrap();
    let installer = lean::Installer::new(dir.path().to_path_buf());
    installer.install_bcs_runtime().unwrap();
    installer.install_bincode_runtime().unwrap();
    let main_source = std::fs::read_to_string("runtime/lean/tests/Main.lean").unwrap();
    let status = run_lean_main(dir.path(), &["bcs", "bincode"], &main_source);
    assert!(status.success());
}

// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{kotlin, CodeGeneratorConfig, SourceInstaller};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir;
use which::which;

#[test]
fn test_kotlin_bcs_runtime_on_simple_data() {
    test_kotlin_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_kotlin_bincode_runtime_on_simple_data() {
    test_kotlin_runtime_on_simple_data(Runtime::Bincode);
}

fn test_kotlin_runtime_on_simple_data(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let installer = kotlin::Installer::new(dir.path().to_path_buf());
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    match runtime {
        Runtime::Bcs => installer.install_bcs_runtime().unwrap(),
        Runtime::Bincode => installer.install_bincode_runtime().unwrap(),
    }

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    let main_path = dir.path().join("Main.kt");
    let mut main = File::create(&main_path).unwrap();
    writeln!(
        main,
        r#"
import com.novi.serde.DeserializationError
import com.novi.serde.Tuple2
import testing.Choice
import testing.Test

fun expect(condition: Boolean, message: String) {{
    if (!condition) {{
        throw RuntimeException(message)
    }}
}}

fun main() {{
    val input = byteArrayOf({0})

    val value = Test.{1}Deserialize(input)

    val a = listOf(4u, 6u)
    val b = Tuple2(-3L, 5uL)
    val c = Choice.C(7.toUByte())
    val value2 = Test(a, b, c)

    expect(value == value2, "value != value2")

    val output = value2.{1}Serialize()
    expect(output.contentEquals(input), "input != output")

    val input2 = input + byteArrayOf(1)
    var failed = false
    try {{
        Test.{1}Deserialize(input2)
    }} catch (e: DeserializationError) {{
        failed = true
    }}
    expect(failed, "expected extra bytes to fail")

    val input3 = byteArrayOf(0, 1)
    failed = false
    try {{
        Test.{1}Deserialize(input3)
    }} catch (e: DeserializationError) {{
        failed = true
    }}
    expect(failed, "expected invalid input to fail")
}}
"#,
        reference
            .iter()
            .map(|x| format!("{}", *x as i8))
            .collect::<Vec<_>>()
            .join(", "),
        runtime.name(),
    )
    .unwrap();

    let mut sources = Vec::new();
    collect_kotlin_sources(dir.path(), &mut sources).unwrap();
    let output_path = dir
        .path()
        .join(format!("kotlin_runtime_simple_{}", runtime.name()));
    if !compile_and_run_kotlin(sources, output_path) {}
}

#[test]
fn test_kotlin_bcs_runtime_on_supported_types() {
    test_kotlin_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_kotlin_bincode_runtime_on_supported_types() {
    test_kotlin_runtime_on_supported_types(Runtime::Bincode);
}

fn test_kotlin_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let installer = kotlin::Installer::new(dir.path().to_path_buf());
    installer.install_module(&config, &registry).unwrap();
    installer.install_serde_runtime().unwrap();
    match runtime {
        Runtime::Bcs => installer.install_bcs_runtime().unwrap(),
        Runtime::Bincode => installer.install_bincode_runtime().unwrap(),
    }

    let positive_encodings = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join(", ");
    let negative_encodings = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join(", ");

    let main_path = dir.path().join("Main.kt");
    let mut main = File::create(&main_path).unwrap();
    writeln!(
        main,
        r#"
import com.novi.serde.DeserializationError
import testing.SerdeData

fun expect(condition: Boolean, message: String) {{
    if (!condition) {{
        throw RuntimeException(message)
    }}
}}

fun main() {{
    val positiveInputs = listOf<ByteArray>({0})
    val negativeInputs = listOf<ByteArray>({1})

    for (input in positiveInputs) {{
        val value = SerdeData.{2}Deserialize(input)
        val output = value.{2}Serialize()
        expect(output.contentEquals(input), "input != output")

        val value2 = SerdeData.{2}Deserialize(input)
        expect(value == value2, "value != value2")

        for (i in input.indices) {{
            val input2 = input.copyOf()
            input2[i] = (input2[i].toInt() xor 0x80).toByte()
            try {{
                val mutated = SerdeData.{2}Deserialize(input2)
                expect(mutated != value, "mutated input should not match original value")
            }} catch (e: DeserializationError) {{
                // All good
            }}
        }}
    }}

    for (input in negativeInputs) {{
        try {{
            SerdeData.{2}Deserialize(input)
            val formatted = input.joinToString(", ") {{ (it.toInt() and 0xFF).toString() }}
            throw RuntimeException("Input should fail to deserialize: [$formatted]")
        }} catch (e: DeserializationError) {{
            // All good
        }}
    }}
}}
"#,
        positive_encodings,
        negative_encodings,
        runtime.name(),
    )
    .unwrap();

    let mut sources = Vec::new();
    collect_kotlin_sources(dir.path(), &mut sources).unwrap();
    let output_path = dir
        .path()
        .join(format!("kotlin_runtime_supported_{}", runtime.name()));
    if !compile_and_run_kotlin(sources, output_path) {}
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "byteArrayOf({})",
        bytes
            .iter()
            .map(|x| format!("{}", *x as i8))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn find_kotlin_compiler() -> Option<PathBuf> {
    which("kotlinc-native").ok()
}

fn collect_kotlin_sources(root: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_kotlin_sources(&path, output)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("kt") {
            output.push(path);
        }
    }
    Ok(())
}

fn resolve_executable(output_path: &Path) -> PathBuf {
    if output_path.exists() {
        return output_path.to_path_buf();
    }
    let kexe_path = output_path.with_extension("kexe");
    if kexe_path.exists() {
        return kexe_path;
    }
    let exe_path = output_path.with_extension("exe");
    if exe_path.exists() {
        return exe_path;
    }
    output_path.to_path_buf()
}

fn compile_and_run_kotlin(sources: Vec<PathBuf>, output_path: PathBuf) -> bool {
    let compiler = match find_kotlin_compiler() {
        Some(path) => {
            println!("Kotlin/Native compiler found: {}", path.display());
            path
        }
        None => {
            eprintln!("Skipping Kotlin/Native runtime test: compiler not found");
            return false;
        }
    };

    let mut args = vec![
        "-produce".to_string(),
        "program".to_string(),
        "-o".to_string(),
        output_path.to_str().unwrap().to_string(),
    ];
    for source in &sources {
        args.push(source.to_str().unwrap().to_string());
    }

    let compile_output = Command::new(compiler).args(&args).output().unwrap();
    if !compile_output.status.success() {
        eprintln!(
            "Kotlin compile stdout:\n{}",
            String::from_utf8_lossy(&compile_output.stdout)
        );
        eprintln!(
            "Kotlin compile stderr:\n{}",
            String::from_utf8_lossy(&compile_output.stderr)
        );
    }
    assert!(compile_output.status.success());

    let executable = resolve_executable(&output_path);
    let run_output = Command::new(executable).output().unwrap();
    if !run_output.status.success() {
        eprintln!(
            "Kotlin runtime stdout:\n{}",
            String::from_utf8_lossy(&run_output.stdout)
        );
        eprintln!(
            "Kotlin runtime stderr:\n{}",
            String::from_utf8_lossy(&run_output.stderr)
        );
    }
    assert!(run_output.status.success());
    true
}

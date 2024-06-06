// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use serde_generate::{java, CodeGeneratorConfig};
use std::{fs::File, io::Write, process::Command};
use tempfile::tempdir;

#[test]
fn test_java_bcs_runtime_on_simple_data() {
    test_java_runtime_on_simple_data(Runtime::Bcs);
}

#[test]
fn test_java_bincode_runtime_on_simple_data() {
    test_java_runtime_on_simple_data(Runtime::Bincode);
}

fn test_java_runtime_on_simple_data(runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let dir = tempdir().unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let generator = java::CodeGenerator::new(&config);
    generator
        .write_source_files(dir.path().to_path_buf(), &registry)
        .unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    let mut source = File::create(dir.path().join("Main.java")).unwrap();
    writeln!(
        source,
        r#"
import java.util.List;
import java.util.Arrays;
import com.novi.serde.DeserializationError;
import com.novi.serde.Unsigned;
import com.novi.serde.Tuple2;
import testing.Choice;
import testing.Test;

public class Main {{
    public static void main(String[] args) throws java.lang.Exception {{
        byte[] input = new byte[] {{{0}}};

        Test value = Test.{1}Deserialize(input);

        List<@Unsigned Integer> a = Arrays.asList(4, 6);
        Tuple2<Long, @Unsigned Long> b = new Tuple2<>(Long.valueOf(-3), Long.valueOf(5));
        Choice c = new Choice.C(Byte.valueOf((byte) 7));
        Test value2 = new Test(a, b, c);

        assert value.equals(value2);

        byte[] output = value2.{1}Serialize();

        assert java.util.Arrays.equals(input, output);

        byte[] input2 = new byte[] {{{0}, 1}};
        try {{
            Test.{1}Deserialize(input2);
        }} catch (DeserializationError e) {{
            return;
        }}
        assert false;
    }}
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

    let paths = std::iter::empty()
        .chain(std::fs::read_dir("runtime/java/com/novi/serde").unwrap())
        .chain(std::fs::read_dir("runtime/java/com/novi/".to_string() + runtime.name()).unwrap())
        .chain(std::fs::read_dir(dir.path().join("testing")).unwrap())
        .map(|e| e.unwrap().path());
    let status = Command::new("javac")
        .arg("-Xlint")
        .arg("-d")
        .arg(dir.path())
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("javac")
        .arg("-Xlint")
        .arg("-cp")
        .arg(dir.path())
        .arg("-d")
        .arg(dir.path())
        .arg(dir.path().join("Main.java"))
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("java")
        .arg("-enableassertions")
        .arg("-cp")
        .arg(dir.path())
        .arg("Main")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_java_bcs_runtime_on_supported_types() {
    test_java_runtime_on_supported_types(Runtime::Bcs);
}

#[test]
fn test_java_bincode_runtime_on_supported_types() {
    test_java_runtime_on_supported_types(Runtime::Bincode);
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "{{{}}}",
        bytes
            .iter()
            .map(|x| format!("{}", *x as i8))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn test_java_runtime_on_supported_types(runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();

    let config =
        CodeGeneratorConfig::new("testing".to_string()).with_encodings(vec![runtime.into()]);
    let generator = java::CodeGenerator::new(&config);
    generator
        .write_source_files(dir.path().to_path_buf(), &registry)
        .unwrap();

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

    let mut source = File::create(dir.path().join("Main.java")).unwrap();
    writeln!(
        source,
        r#"
import java.util.List;
import java.util.Arrays;
import com.novi.serde.DeserializationError;
import com.novi.serde.Unsigned;
import com.novi.serde.Tuple2;
import testing.SerdeData;

public class Main {{
    static final byte[][] positive_inputs = new byte[][] {{{0}}};
    static final byte[][] negative_inputs = new byte[][] {{{1}}};

    public static void main(String[] args) throws java.lang.Exception {{
        for (byte[] input : positive_inputs) {{
            SerdeData value = SerdeData.{2}Deserialize(input);
            byte[] output = value.{2}Serialize();

            assert java.util.Arrays.equals(input, output);

            // Test self-equality for the Serde value.
            {{
                SerdeData value2 = SerdeData.{2}Deserialize(input);
                assert value.equals(value2);
            }}

            // Test simple mutations of the input.
            for (int i = 0; i < input.length; i++) {{
                byte[] input2 = input.clone();
                input2[i] ^= 0x80;
                try {{
                    SerdeData value2 = SerdeData.{2}Deserialize(input2);
                    assert !value2.equals(value);
                }} catch (DeserializationError e) {{
                    // All good
                }}
            }}

        }}

        for (byte[] input : negative_inputs) {{
            try {{
                SerdeData value = SerdeData.{2}Deserialize(input);
                Integer[] bytes = new Integer[input.length];
                Arrays.setAll(bytes, n -> Math.floorMod(input[n], 256));
                throw new Exception("Input should fail to deserialize: " + Arrays.asList(bytes));
            }} catch (DeserializationError e) {{
                    // All good
            }}
        }}
    }}
}}
"#,
        positive_encodings.join(", "),
        negative_encodings.join(", "),
        runtime.name(),
    )
    .unwrap();

    let paths = std::iter::empty()
        .chain(std::fs::read_dir("runtime/java/com/novi/serde").unwrap())
        .chain(std::fs::read_dir("runtime/java/com/novi/".to_string() + runtime.name()).unwrap())
        .chain(std::fs::read_dir(dir.path().join("testing")).unwrap())
        .map(|e| e.unwrap().path());
    let status = Command::new("javac")
        .arg("-Xlint")
        .arg("-d")
        .arg(dir.path())
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("javac")
        .arg("-Xlint")
        .arg("-cp")
        .arg(dir.path())
        .arg("-d")
        .arg(dir.path())
        .arg(dir.path().join("Main.java"))
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("java")
        .arg("-enableassertions")
        .arg("-cp")
        .arg(dir.path())
        .arg("Main")
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_java_bcs_runtime_autotest() {
    let dir = tempdir().unwrap();
    let paths = std::iter::empty()
        .chain(std::fs::read_dir("runtime/java/com/novi/serde").unwrap())
        .chain(std::fs::read_dir("runtime/java/com/novi/bcs").unwrap())
        .map(|e| e.unwrap().path());
    let status = Command::new("javac")
        .arg("-Xlint")
        .arg("-d")
        .arg(dir.path())
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("java")
        .arg("-enableassertions")
        .arg("-cp")
        .arg(dir.path())
        .arg("com.novi.bcs.BcsTest")
        .status()
        .unwrap();
    assert!(status.success());
}

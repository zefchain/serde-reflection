// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::test_utils;
use crate::test_utils::{Choice, Runtime, Test};
use heck::CamelCase;
use serde_generate::{csharp, CodeGeneratorConfig, SourceInstaller};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

lazy_static::lazy_static! {
    // `dotnet build` spuriously fails on linux if run concurrently
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

/// Returns:
/// 1. A `PathBuf` to the directory to write test data into
/// 2. Optionally, a `tempfile::TempDir` which deletes the directory when it goes out of scope
fn create_test_dir(test_name: &'static str) -> (PathBuf, Option<tempfile::TempDir>) {
    // Set env var to generate into subdirectories for inspection
    if std::env::var("TEST_USE_SUBDIR").is_ok() {
        let mut tries = 0;
        while tries < 20 {
            let test_dir_name = if tries == 0 {
                test_name.into()
            } else {
                format!("{}_{}", test_name, tries)
            };
            let dir = Path::new("tests").join(test_dir_name).to_path_buf();
            match std::fs::create_dir(&dir) {
                Ok(()) => return (dir, None),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => tries += 1,
                Err(e) => panic!("Error creating test directory: {:?}", e),
            }
        }
        panic!("Error creating test directory: Too many existing test directories");
    } else {
        let tempdir = tempfile::Builder::new()
            .suffix(&format!("_{}", test_name))
            .tempdir()
            .unwrap();
        (tempdir.path().to_path_buf(), Some(tempdir))
    }
}

fn dotnet_build(proj_dir: &Path) {
    let _lock = MUTEX.lock();
    let status = Command::new("dotnet")
        .arg("build")
        .current_dir(proj_dir)
        .status()
        .unwrap();
    assert!(status.success());
}

fn run_nunit(proj_dir: &Path) {
    let _lock = MUTEX.lock();
    let status = Command::new("dotnet")
        .arg("test")
        .current_dir(proj_dir)
        .status()
        .unwrap();
    assert!(status.success());
}

fn make_test_project(
    tmp_dir: &Path,
    runtime: Runtime,
    test_name: &str,
    library_name: &str,
) -> std::io::Result<PathBuf> {
    let test_dir = tmp_dir.join(test_name.replace('.', "/"));
    std::fs::create_dir(&test_dir)?;
    let mut proj = std::fs::File::create(test_dir.join(format!("{}.csproj", test_name)))?;
    write!(
        proj,
        r#"
<Project Sdk="Microsoft.NET.Sdk">

  <PropertyGroup>
    <TargetFrameworks>net6</TargetFrameworks>
    <IsPackable>false</IsPackable>
    <LangVersion>7.2</LangVersion>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="NUnit" Version="3.12.0" />
    <PackageReference Include="NUnit3TestAdapter" Version="3.17.0" />
    <PackageReference Include="Microsoft.NET.Test.Sdk" Version="16.8.0" />
  </ItemGroup>

  <ItemGroup>
    <ProjectReference Include="..\Serde\Serde.csproj" />
    <ProjectReference Include="..\{0}\{0}.csproj" />
    <ProjectReference Include="..\{1}\{1}.csproj" />
  </ItemGroup>

</Project>
"#,
        runtime.name().to_camel_case(),
        library_name
    )?;
    Ok(test_dir)
}

#[test]
fn test_csharp_bcs_runtime_tests() {
    let test_dir = Path::new("runtime/csharp/Serde.Tests");
    dotnet_build(test_dir);
    run_nunit(test_dir);
}

#[test]
fn test_csharp_bcs_runtime_on_simple_data() {
    let (dir, _tmp) = create_test_dir("test_csharp_runtime_on_simple_data");
    test_csharp_runtime_on_simple_data(dir, Runtime::Bcs);
}

#[test]
fn test_csharp_bincode_runtime_on_simple_data() {
    let (dir, _tmp) = create_test_dir("test_csharp_bincode_runtime_on_simple_data");
    test_csharp_runtime_on_simple_data(dir, Runtime::Bincode);
}

fn test_csharp_runtime_on_simple_data(dir: PathBuf, runtime: Runtime) {
    let registry = test_utils::get_simple_registry().unwrap();
    let test_dir = make_test_project(&dir, runtime, "Testing", "SimpleData").unwrap();
    let config =
        CodeGeneratorConfig::new("SimpleData".to_string()).with_encodings(vec![runtime.into()]);

    let installer = csharp::Installer::new(dir);
    installer.install_serde_runtime().unwrap();
    match runtime {
        Runtime::Bincode => installer.install_bincode_runtime().unwrap(),
        Runtime::Bcs => installer.install_bcs_runtime().unwrap(),
    }
    installer.install_module(&config, &registry).unwrap();

    let reference = runtime.serialize(&Test {
        a: vec![4, 6],
        b: (-3, 5),
        c: Choice::C { x: 7 },
    });

    let mut source = File::create(test_dir.join("TestRuntime.cs")).unwrap();
    writeln!(
        source,
        r#"
using System;
using System.Collections.Generic;
using System.IO;
using NUnit.Framework;
using {1};
using Serde;
using SimpleData;

namespace Testing {{
    [TestFixture]
    public class Test{1}Runtime {{
        [Test]
        public void TestRoundTrip() {{
            byte[] input = new byte[] {{{0}}};

            Test test = Test.{1}Deserialize(input);

            var a = new ValueArray<uint>(new uint[] {{ 4, 6 }});
            var b = ((long)-3, (ulong)5);
            Choice c = new Choice.C((byte) 7);
            Test test2 = new Test(a, b, c);

            Assert.AreEqual(test, test2);

            byte[] output = test2.{1}Serialize();

            CollectionAssert.AreEqual(input, output);

            byte[] input2 = new byte[] {{{0}, 1}};
            Assert.Throws<DeserializationException>(() => Test.{1}Deserialize(input2));
        }}
    }}
}}
"#,
        reference
            .iter()
            .map(|x| format!("{}", *x))
            .collect::<Vec<_>>()
            .join(", "),
        runtime.name().to_camel_case(),
    )
    .unwrap();

    dotnet_build(&test_dir);
    run_nunit(&test_dir);
}

#[test]
fn test_csharp_bcs_runtime_on_supported_types() {
    let (dir, _tmp) = create_test_dir("test_csharp_bcs_runtime_on_supported_types");
    test_csharp_runtime_on_supported_types(dir, Runtime::Bcs);
}

#[test]
fn test_csharp_bincode_runtime_on_supported_types() {
    let (dir, _tmp) = create_test_dir("test_csharp_bincode_runtime_on_supported_types");
    test_csharp_runtime_on_supported_types(dir, Runtime::Bincode);
}

fn quote_bytes(bytes: &[u8]) -> String {
    format!(
        "yield return new TestCaseData(new byte[] {{ {} }});",
        bytes
            .iter()
            .map(|x| format!("{}", *x))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn test_csharp_runtime_on_supported_types(dir: PathBuf, runtime: Runtime) {
    let registry = test_utils::get_registry().unwrap();
    let test_dir = make_test_project(&dir, runtime, "Testing", "Data").unwrap();
    let config = CodeGeneratorConfig::new("Data".to_string()).with_encodings(vec![runtime.into()]);

    let installer = csharp::Installer::new(dir);
    installer.install_serde_runtime().unwrap();
    match runtime {
        Runtime::Bincode => installer.install_bincode_runtime().unwrap(),
        Runtime::Bcs => installer.install_bcs_runtime().unwrap(),
    }
    installer.install_module(&config, &registry).unwrap();

    let positive_encodings = runtime
        .get_positive_samples_quick()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join("\n\t\t\t\t");

    let negative_encodings = runtime
        .get_negative_samples()
        .iter()
        .map(|bytes| quote_bytes(bytes))
        .collect::<Vec<_>>()
        .join("\n\t\t\t\t");

    let mut source = File::create(test_dir.join("TestRuntime.cs")).unwrap();
    writeln!(
        source,
        r#"
using System;
using System.Collections;
using System.Linq;
using System.IO;
using NUnit.Framework;
using {2};
using Serde;
using Data;

namespace Testing {{
    [TestFixture]
    public class Test{2}Runtime {{
        public static IEnumerable TestPositiveInputs
        {{
            get
            {{
                {0}
                yield break;
            }}
        }}

        public static IEnumerable TestNegativeInputs
        {{
            get
            {{
                {1}
                yield break;
            }}
        }}

        [Test, TestCaseSource("TestPositiveInputs")]
        public void TestRoundTrip(byte[] input) {{
            SerdeData test = SerdeData.{2}Deserialize(input);
            byte[] output = test.{2}Serialize();
            CollectionAssert.AreEqual(input, output);
        }}

        [Test, TestCaseSource("TestPositiveInputs")]
        public void TestValueEquality(byte[] input) {{
            SerdeData test1 = SerdeData.{2}Deserialize(input);
            SerdeData test2 = SerdeData.{2}Deserialize(input);
            Assert.AreEqual(test1, test2);
        }}

        [Test, TestCaseSource("TestPositiveInputs")]
        public void TestChangedBytes(byte[] input) {{
            SerdeData test = SerdeData.{2}Deserialize(input);
            for (int i = 0; i < input.Length; i++) {{
                byte[] input2 = input.ToArray();
                input2[i] ^= 0x80;
                SerdeData test2;
                try {{
                    test2 = SerdeData.{2}Deserialize(input2);
                }}
                catch (Exception) {{ continue; }}
                Assert.AreNotEqual(test2, test);
            }}
        }}

        [Test, TestCaseSource("TestNegativeInputs")]
        public void TestNegativeInputsFails(byte[] input) {{
            Assert.Catch(() => SerdeData.{2}Deserialize(input));
        }}
    }}
}}"#,
        positive_encodings,
        negative_encodings,
        runtime.name().to_camel_case()
    )
    .unwrap();

    dotnet_build(&test_dir);
    run_nunit(&test_dir);
}

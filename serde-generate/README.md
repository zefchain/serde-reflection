# serde-generate

[![serde-generate on crates.io](https://img.shields.io/crates/v/serde-generate)](https://crates.io/crates/serde-generate)
[![Documentation (latest release)](https://docs.rs/serde-generate/badge.svg)](https://docs.rs/serde-generate/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](../LICENSE-APACHE)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](../LICENSE-MIT)

This crate aims to compile the data formats extracted from Rust by [`serde-reflection`](https://crates.io/crates/serde-reflection)
into type definitions and (de)serialization methods for other programming languages.

It can be used as a library or as a command-line tool (see `serdegen` below).

### Supported Languages

The following programming languages are fully supported as target languages:

* C++ 17
* Java 8
* Python 3 (requires numpy >= 1.20.1)
* Rust 2018
* Go >= 1.14
* C# (NetCoreApp >= 6.0)
* Swift 5.3
* OCaml
* Dart >= 3

The following languages are partially supported and/or still considered under development:

* TypeScript 4 (packaged and tested with Deno) [(follow-up issue)](https://github.com/zefchain/serde-reflection/issues/58)
* Solidity (tested with Revm) [(initial PR with discussion)](https://github.com/zefchain/serde-reflection/pull/61)

### Supported Encodings

Type definitions in a target language are meant to be used together with a runtime library that
provides (de)serialization in a particular [Serde encoding format](https://serde.rs/#data-formats).

This crate provides easy-to-deploy runtime libraries for the following binary formats, in all supported languages:

* [Bincode](https://docs.rs/bincode/1.3.1/bincode/) (default configuration only),
* [BCS](https://github.com/diem/bcs) (short for Binary Canonical Serialization, the main format used
  in the [Diem blockchain](https://github.com/diem/diem)).

### Quick Start with Python and Bincode

In the following example, we transfer a `Test` value from Rust to Python using [`bincode`](https://docs.rs/bincode/1.3.1/bincode/).
```rust
use serde::{Deserialize, Serialize};
use serde_reflection::{Registry, Tracer, TracerConfig};
use std::io::Write;

#[derive(Serialize, Deserialize)]
struct Test {
    a: Vec<u64>,
    b: (u32, u32),
}

// Obtain the Serde format of `Test`. (In practice, formats are more likely read from a file.)
let mut tracer = Tracer::new(TracerConfig::default());
tracer.trace_simple_type::<Test>().unwrap();
let registry = tracer.registry().unwrap();

// Create Python class definitions.
let mut source = Vec::new();
let config = serde_generate::CodeGeneratorConfig::new("testing".to_string())
    .with_encodings(vec![serde_generate::Encoding::Bincode]);
let generator = serde_generate::python3::CodeGenerator::new(&config);
generator.output(&mut source, &registry)?;

assert!(
    String::from_utf8_lossy(&source).contains(
    r#"
@dataclass(frozen=True)
class Test:
    a: typing.Sequence[st.uint64]
    b: typing.Tuple[st.uint32, st.uint32]
"#));

// Append some test code to demonstrate Bincode deserialization
// using the runtime in `serde_generate/runtime/python/bincode`.
writeln!(
    source,
    r#"
value = Test.bincode_deserialize(bytes({:?}))
assert value == Test(a=[4, 6], b=(3, 5))
"#,
    bincode::serialize(&Test { a: vec![4, 6], b: (3, 5) }).unwrap(),
)?;

// Execute the Python code.
let mut child = std::process::Command::new("python3")
    .arg("-")
    .env("PYTHONPATH", std::env::var("PYTHONPATH").unwrap_or_default() + ":runtime/python")
    .stdin(std::process::Stdio::piped())
    .spawn()?;
child.stdin.as_mut().unwrap().write_all(&source)?;
let output = child.wait_with_output()?;
assert!(output.status.success());
```

### Binary Tool

In addition to a Rust library, this crate provides a binary tool `serdegen` to process Serde formats
saved on disk.

The tool `serdegen` assumes that a Rust value of type `serde_reflection::Registry` has
been serialized into a YAML file. The recommended way to generate such a value is to
use the library `serde-reflection` to introspect Rust definitions (see also the
example above).

For a quick test, one may create a test file like this:
```bash
cat >test.yaml <<EOF
---
Foo:
  ENUM:
    0:
      A:
        NEWTYPE:
          U64
    1:
      B: UNIT
EOF
```

Then, the following command will generate Python class definitions and write them into `test.py`:
```bash
cargo run -p serde-generate-bin -- --language python3 test.yaml > test.py
```

To create a python module `test` and install the bincode runtime in a directory `$DEST`, you may run:
```bash
cargo run -p serde-generate-bin -- --language python3 --with-runtimes serde bincode --module-name test --target-source-dir "$DEST" test.yaml
```

See the help message of the tool with `--help` for more options.

Note: Outside of this repository, you may install the tool with `cargo install serde-generate-bin` then use `$HOME/.cargo/bin/serdegen`.

## Contributing

See the [CONTRIBUTING](../CONTRIBUTING.md) file for how to help out.

## License

This project is available under the terms of either the [Apache 2.0 license](../LICENSE-APACHE) or the [MIT
license](../LICENSE-MIT).

<!--
README.md is generated from README.tpl by cargo readme. To regenerate:

cargo install cargo-readme
cargo readme > README.md
-->

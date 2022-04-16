# serde-name

[![serde-name on crates.io](https://img.shields.io/crates/v/serde-name)](https://crates.io/crates/serde-name)
[![Documentation (latest release)](https://docs.rs/serde-name/badge.svg)](https://docs.rs/serde-name/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](../LICENSE-APACHE)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](../LICENSE-MIT)

This crate provides fast and reliable ways to extract and to override the Serde name
of a Rust container.

## Extracting Serde names

Name extraction relies on the Deserialize trait of Serde:

```rust
#[derive(Deserialize)]
struct Foo {
  bar: Bar,
}

#[derive(Deserialize)]
#[serde(rename = "ABC")]
enum Bar { A, B, C }

assert_eq!(trace_name::<Foo>(), Some("Foo"));
assert_eq!(trace_name::<Bar>(), Some("ABC"));
assert_eq!(trace_name::<Option<Bar>>(), None);
```

## Overriding Serde names

`SerializeNameAdapter` and `DeserializeNameAdapter` may be used to override the name
of a container in the cases where `#[serde(rename = "..")]` is not flexible enough.

```rust
#[derive(Serialize, Deserialize)]
#[serde(remote = "Foo")] // Generates Foo::(de)serialize instead of implementing Serde traits.
struct Foo<T> {
    data: T,
}

impl<'de, T> Deserialize<'de> for Foo<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Foo::deserialize(DeserializeNameAdapter::new(
            deserializer,
            std::any::type_name::<Self>(),
        ))
    }
}

impl<T> Serialize for Foo<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        Foo::serialize(
            self,
            SerializeNameAdapter::new(serializer, std::any::type_name::<Self>()),
        )
    }
}

// Testing the Deserialize implementation
assert!(trace_name::<Foo<u64>>().unwrap().ends_with("Foo<u64>"));

// Testing the Serialize implementation
use serde_reflection::*;
let mut tracer = Tracer::new(TracerConfig::default());
let mut samples = Samples::new();
let (mut ident, _) = tracer.trace_value(&mut samples, &Foo { data: 1u64 }).unwrap();
ident.normalize().unwrap();
assert!(matches!(ident, Format::TypeName(s) if s.ends_with("Foo<u64>")));
```

## Contributing

See the [CONTRIBUTING](../CONTRIBUTING.md) file for how to help out.

## License

This project is available under the terms of either the [Apache 2.0 license](../LICENSE-APACHE) or the [MIT license](../LICENSE-MIT).

<!--
README.md is generated from README.tpl by cargo readme. To regenerate:

cargo install cargo-readme
cargo readme > README.md
-->

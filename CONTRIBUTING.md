# Contributing to serde-reflection

Development for `serde-reflection` happens in the open on GitHub.

## Pull Requests

We actively welcome your pull requests. If you have a new feature in mind, please discuss the feature in an issue to
ensure that your contributions will be accepted.

1. Fork the repo and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation.
4. Ensure the test suite passes. `cargo test` runs the tests of every target language enabled by default, which
   requires the corresponding toolchains. To test a single language, run
   `cargo test --no-default-features --features <language>` (e.g. `python3`). See
   [`.github/workflows/main.yml`](.github/workflows/main.yml) for the toolchains used by CI.
5. Run `cargo fmt` to automatically format your changes, and
   `cargo clippy --workspace --all-features --all-targets` to check for warnings (CI rejects both formatting issues
   and warnings).
6. If you've changed the crate-level documentation in `src/lib.rs`, regenerate the corresponding `README.md` with
   [`cargo readme`](https://crates.io/crates/cargo-readme): `cargo readme > README.md` in the crate directory.

## Issues

We use GitHub issues to track public bugs. Please ensure your description is clear and has sufficient instructions to be
able to reproduce the issue.

To report a security vulnerability, please do not file a public issue. Instead, use GitHub's
[private vulnerability reporting](https://github.com/serde-tools/serde-reflection/security/advisories/new).

## License

By contributing to `serde-reflection`, you agree that your contributions will be dual-licensed under the terms of the
[`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE) files in the root directory of this source
tree.

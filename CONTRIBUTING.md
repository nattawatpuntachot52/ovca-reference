# Contributing

Keep changes focused on the public contract and deterministic reducer boundary.
Do not add operational integrations or include sensitive, personal, or
machine-specific content in source, fixtures, documentation, or test output.

## Contract changes

A contract change should update every affected surface together:

1. the Rust type and Validate implementation;
2. its JSON Schema document;
3. the matching synthetic sample;
4. reducer behavior when the contract participates in the event stream;
5. focused positive and rejection tests; and
6. relevant architecture, workflow, limitation, or decision documentation.

Maintain contract_version 1.0 compatibility. Use a new explicit version for an
incompatible wire change.

Contract tests must describe their actual assurance. The current suite parses
schema documents as JSON, deserializes samples into Rust types, and runs
semantic validation. It does not evaluate samples with a JSON Schema engine.

## Required checks

Run these commands from the repository root:

~~~text
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --all-targets --locked
cargo test --manifest-path rust/Cargo.toml --all-targets --locked
cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked -- -D warnings
~~~

Contributions are accepted under Apache-2.0.

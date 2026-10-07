# Contributing

Keep changes focused on the public contract, deterministic reducer boundary,
and explicitly published reusable skills. Do not add operational integrations
or include sensitive, personal, or machine-specific content in source,
fixtures, skills, documentation, or test output.

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

## Skill changes

Keep public skills domain-neutral, self-contained, and instruction-only unless
the contribution explicitly establishes a reviewed need for executable
resources. A skill must not depend on a contributor's local paths, private
context, credentials, or unavailable helper files.

For each added or changed skill:

1. keep `SKILL.md` frontmatter limited to supported fields with a concise,
   discriminating name and description;
2. link every supporting reference from `SKILL.md` and add only resources the
   workflow actually needs;
3. preserve user authority over output language, destination, and mutations;
4. check instructions for secret disclosure, machine-specific paths, unfinished
   placeholders, and broken relative links; and
5. run the Codex skill-creator `scripts/quick_validate.py` against the skill
   directory when that validator is available.

Instruction-only skills do not execute roles or connect this repository to
external systems.

## Required checks

For Rust or contract changes, run these commands from the repository root:

~~~text
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --all-targets --locked
cargo test --manifest-path rust/Cargo.toml --all-targets --locked
cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked -- -D warnings
~~~

Contributions are accepted under Apache-2.0.

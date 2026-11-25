## Summary & Contract Surface
- **Task / Issue Reference**: `<issue_number_or_bounded_scope_summary>`
- **OVCA Role**: `<coordinator | engineer | reviewer | auditor>`
- **Touched Crates / Schemas / Docs**:
  - Crates / modules changed:
  - JSON Schemas & synthetic samples updated (if any):
  - Backward compatibility with `contract_version = "1.0"`: `<preserved / explain>`

## Pre-Merge Verification (`No Implicit Pass`)
Report only commands executed against the current head of this branch:
- [ ] `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` -> `<pass / fail / not run>`
- [ ] `cargo check --manifest-path rust/Cargo.toml --all-targets --locked` -> `<pass / fail / not run>`
- [ ] `cargo test --manifest-path rust/Cargo.toml --all-targets --locked` -> `<pass / fail / not run>`
- [ ] `cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked -- -D warnings` -> `<pass / fail / not run>`

## Semantic Conflict Check (Parallel / Stacked PRs)
- **Other open PRs checked**: `<none / PR numbers>`
- **Potential schema, crate, or base-SHA collision**: `<none / details>`

## AGENT_ACK

> Do not guess unexposed runtime details. Record `UNKNOWN (not exposed by runtime)` when the host harness does not surface an exact model build identifier, and `UNKNOWN / NOT_CONNECTED` for disconnected nodes.

```text
AGENT_ACK
mission_id: <mission_id>
task_id: <task_id>
assignment_id: <branch_name>
phase_id: <phase_id>
agent_id: <agent_identifier>
ovca_role: <coordinator | engineer | reviewer | auditor>
actual_provider_runtime: <runtime_and_os>
actual_model_id_version: <exact_model_id_or_UNKNOWN (not exposed by runtime)>
tools_environment: <host / rustc_cargo / git / gh>
mutation_boundary: <branch_and_writable_scope>
verification_summary: <exact_commands_run_and_observed_outcomes>
```

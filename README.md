# OVCA

OVCA is a public, domain-neutral reference for an evidence-bound workflow with
five responsibilities:

- **Owner** supplies the objective, authority, scope, constraints, and bounded
  execution budget.
- **Coordinator** accepts the intake, dispatches the Engineer, and produces the
  final public response.
- **Engineer** returns a typed result with evidence records.
- **Reviewer** records an independent pass or repair decision.
- **Auditor** counterchecks the review and either concurs or disagrees.

The repository is deliberately small. It contains versioned data contracts, a
synchronous deterministic reducer, JSON Schema documents, and synthetic event
streams. It does not execute roles or connect the workflow to external systems.

## Workflow

Every attempt follows the same evidence path:

~~~text
IntakeAccepted
  -> CoordinatorDispatched
  -> EngineerReturned
  -> VerificationRecorded
  -> ReviewerDecided
  -> AuditorCounterchecked
~~~

The audit step selects one of three bounded continuations:

- Reviewer pass plus Auditor concurrence leads to CoordinatorFinalized with a
  passed outcome.
- Reviewer repair plus Auditor concurrence leads to RepairDirected and a new
  CoordinatorDispatched event while repair budget remains.
- Auditor disagreement, or a repair decision after the repair budget is
  exhausted, leads to OwnerEscalated and then CoordinatorFinalized with an
  escalated outcome.

See [System workflows](docs/system-workflows.md) for the complete transition and
reference rules.

## Repository contents

| Path | Purpose |
|---|---|
| rust/ovca-types | Version 1.0 Rust contracts and semantic validation |
| rust/ovca-workflow | Deterministic WorkflowReducer and recover_public helper |
| contracts | Draft 2020-12 JSON Schema documents and synthetic samples |
| examples | Complete pass, repair-then-pass, and disagreement event streams |
| docs | Architecture, trust boundary, limitations, and design decisions |

## Deterministic reference behavior

WorkflowReducer::apply validates an event against a cloned reducer state and
commits the new state only when every check succeeds. Invalid events therefore
leave the prior state unchanged. recover_public replays a caller-supplied event
slice from the initial phase and returns a PublicWorkflowSnapshot. That public
projection contains only phase, event_count, repair_attempts, outcome,
evidence_count, and final_response_present. It does not return caller-supplied
identifiers or text, or the FinalResponse record.

The reducer checks event order, predecessor links, workflow identity, event
budget, role and attempt bindings, dispatch input continuity, cross-record
references, globally unique evidence identifiers, known evidence references,
review and audit combinations, bounded repair, and final outcome consistency.
It performs no I/O and owns no durable state.

## Validation

From the repository root:

~~~text
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo check --manifest-path rust/Cargo.toml --all-targets --locked
cargo test --manifest-path rust/Cargo.toml --all-targets --locked
cargo clippy --manifest-path rust/Cargo.toml --all-targets --locked -- -D warnings
~~~

The contract tests parse every JSON Schema file as JSON, then deserialize each
sample into its Rust type and run semantic validation through Validate. They do
not evaluate samples with a JSON Schema engine. The workflow tests replay the
three synthetic examples and exercise rejection paths.

## Privacy

All repository examples use synthetic identifiers, summaries, and references.
Workflow events and their nested contracts retain caller-supplied strings and
do not redact them. Callers remain responsible for keeping sensitive content
out of public summaries, evidence summaries, sources, and identifiers. The
PublicWorkflowSnapshot is a separate minimized projection containing only
enums, counts, and a final-response-presence flag.

## Scope

This repository demonstrates contract compatibility and deterministic workflow
reduction. It does not establish identity, authenticate authority, inspect
evidence contents, run verification commands, execute role work, persist event
streams, or prove that a real workflow is useful. See [Security boundary](docs/security-boundary.md)
and [Limitations](docs/limitations.md).

Licensed under Apache-2.0. See [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
for dependency notices.

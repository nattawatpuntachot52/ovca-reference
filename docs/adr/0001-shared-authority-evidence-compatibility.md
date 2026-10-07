# ADR 0001: Shared authority, evidence, decision, and event compatibility

- Status: Accepted
- Contract version: 1.0
- Scope: Public data contracts and semantic validation

## Context

The workflow needs a shared vocabulary for owner-granted scope, evidence
metadata, decisions, and ordered events. The vocabulary must be stable across
callers without treating valid JSON as proof of identity, authority, or truth.

## Decision

ovca-types defines closed, versioned records:

- Authority identifies the Owner-granted objective, scope, and constraints.
- Evidence carries a stable identifier, kind, summary, source, and
  caller-supplied timestamp.
- Decision carries a role, pass, repair, or escalate disposition, rationale,
  evidence references, and timestamp.
- WorkflowEvent binds a workflow identifier, event identifier, sequence,
  predecessor, timestamp, and one typed WorkflowEventKind payload.

Every top-level record carries contract_version 1.0. Rust deserialization rejects
unknown struct fields. Validate supplies semantic checks that cannot be inferred
from a parsed JSON object alone.

Authority validation requires granted_by to be Owner. That requirement records
the intended source of authority; it does not authenticate the record creator.
Evidence references identify prior records in the accepted stream and do not
contain or validate the referenced content.

## Compatibility

Version 1.0 has one explicit Rust and JSON representation. A future incompatible
shape requires a new version and an explicit conversion. Callers must not
reinterpret version 1.0 bytes silently.

The JSON Schema documents describe the public structural surface. Rust
deserialization and Validate remain the semantic reference. Tests parse the
schema documents and validate samples through Rust; a separate schema engine is
outside the current test boundary.

## Privacy

Published samples contain only synthetic identifiers, summaries, and sources.
The contracts do not redact strings, so callers control what they place in
public records.

## Consequences

Callers gain a compact compatibility surface for exchanging workflow records.
They remain responsible for identity, admission, evidence custody, execution,
and persistence.

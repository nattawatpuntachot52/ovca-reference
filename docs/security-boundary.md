# Security Boundary

OVCA is a reference library for validating typed records and reducing an event
stream. Its security properties stop at that boundary.

## What the crates enforce

- Rust deserialization rejects unknown fields on public contract structs.
- Validate checks contract versions, required text, basic timestamp form,
  collection rules, and record-specific semantic invariants.
- WorkflowReducer checks ordered event chains, role and attempt bindings,
  cross-record references, known evidence references, repair limits, and final
  outcome consistency.
- WorkflowReducer::apply leaves the prior value unchanged when an event fails.

These checks protect the internal consistency of caller-supplied values. They do
not prove that a caller, actor, timestamp, source, or evidence statement is
authentic.

## Caller responsibilities

Authority is a typed assertion. The granted_by field must be Owner, but the
library does not identify the caller or prove that an Owner created the record.

Evidence contains an identifier, kind, summary, source, and timestamp. The
library does not fetch the source, inspect content, bind content
cryptographically, or determine whether the summary is true. Review, audit,
decision, and final-response evidence references prove only that the identifier
was introduced earlier in the accepted stream.

Workflow events and their nested contracts retain caller-supplied strings.
Applications must keep sensitive content out of identifiers, objectives,
summaries, rationale, instructions, sources, and constraints when records may
be published. PublicWorkflowSnapshot is a separate minimized projection: it
contains only enum, count, and boolean fields and does not expose identifiers,
text, evidence records, or FinalResponse.

## Capability boundary

Neither crate receives filesystem, network, clock, credential, persistence, or
role-execution capabilities. The reducer does not run
verification checks, invoke any role, or write an event log. An integrating
application must define and review those boundaries separately.

## Reporting a vulnerability

Use GitHub private vulnerability reporting when it is available for this
repository. If it is unavailable, open a minimal public issue asking the
maintainers for a private contact channel. Do not include exploit details or
sensitive content in a public issue.

# Architecture

OVCA is a two-crate Rust workspace with a file-based contract surface and
synthetic examples. The design separates data compatibility from workflow
reduction.

## Components

| Component | Responsibility |
|---|---|
| ovca-types | Defines version 1.0 role, authority, evidence, decision, invocation, result, verification, review, audit, repair, budget, response, and event types |
| ovca-workflow | Applies caller-supplied WorkflowEvent values to a deterministic state machine |
| contracts | Publishes closed Draft 2020-12 structural schemas and matching samples |
| examples | Supplies complete event streams for three synthetic scenarios |

All public contract structs reject unknown fields during Rust deserialization.
Their Validate implementations enforce semantic rules that are not represented
fully by the schemas. Schema parsing, Rust deserialization, and semantic
validation are separate checks.

## Contract layer

ovca-types exposes:

- Role and authority records for Owner, Coordinator, Engineer, Reviewer, and
  Auditor responsibilities.
- Evidence, decisions, role invocation and result, and verification outcome
  records.
- ReviewPacket, AuditPacket, and RepairDirective records for independent
  judgment and bounded repair.
- ExecutionBudget, FinalResponse, WorkflowEventKind, and WorkflowEvent for the
  event stream.

Every top-level contract carries contract_version 1.0. The public JSON wire
format uses snake-case enum values and a tagged WorkflowEventKind shape with
type and data fields.

Authority.constraints, RoleInvocation.input_refs, ReviewPacket.findings, and
AuditPacket.findings are required wire fields even when their semantic rules
permit an empty array. Within the public reducer, every dispatch must carry one
exact current input reference.

## Reducer layer

ovca-workflow exposes WorkflowReducer, PublicWorkflowSnapshot, WorkflowError,
WorkflowPhase, and recover_public.

WorkflowReducer::apply is transactional at the value level: it applies an event
to a clone, then replaces the current value only on success. The reducer tracks
the event cursor, current attempt, known evidence identifiers, relevant record
identifiers, repair consumption, pending outcome, and final response.

Those caller-controlled records remain internal to the reducer.
PublicWorkflowSnapshot projects only phase, event_count, repair_attempts,
outcome, evidence_count, and final_response_present. It does not expose the
workflow identifier, evidence identifiers, caller text, or FinalResponse.
evidence_count counts globally unique Evidence records introduced by Engineer
results and verification outcomes; later references do not increase it.

recover_public constructs a fresh reducer and applies the supplied events in
order. Its result depends only on the supplied values and crate code. The
library performs no persistence, role execution, verification execution, or
external calls.

## Dependency direction

~~~text
contracts and examples
        |
        v
    ovca-types
        |
        v
   ovca-workflow
~~~

ovca-types has no dependency on ovca-workflow. The reducer consumes the public
contracts and never changes their wire format.

The exact event transitions are documented in [System workflows](system-workflows.md).
Trust assumptions are documented in [Security boundary](security-boundary.md),
and deliberately unsupported behavior is listed in [Limitations](limitations.md).

# ADR 0003: Provider-neutral role execution ports

- Status: Accepted as a boundary
- Enforcement status: Contract available; execution adapter not included
- Scope: Translation between a role invocation and a role result

## Context

Role work may be performed by different implementations, while the public
workflow requires one stable input and output shape. Execution details must not
leak into the reducer or change the event contract.

## Decision

RoleInvocation is the input port record. It names the request, requesting role,
target role, objective, authority reference, input references, one-based
attempt, and timestamp. In the public reducer, input_refs contains exactly the
intake request reference on the first attempt or the preceding repair directive
identifier on a repaired attempt.

RoleResult is the output port record. It binds the invocation, role, succeeded
or failed status, public summary, evidence records, and completion timestamp.

An execution adapter belongs outside these crates. It may translate a validated
RoleInvocation into implementation-specific work and return a RoleResult, but
it must not bypass the event sequence. The caller records
CoordinatorDispatched before execution and EngineerReturned afterward, then
supplies separate VerificationRecorded, ReviewerDecided, and
AuditorCounterchecked events.

The public repository intentionally includes no concrete execution adapter.
WorkflowReducer never calls one. Tests use complete synthetic event records and
therefore remain deterministic.

## Failure boundary

A failed RoleResult is still a typed workflow record. It proceeds to
VerificationRecorded and ReviewerDecided, where a pass decision is rejected.
Retry through the public workflow occurs only through a valid repair decision,
audit concurrence, RepairDirected, and a new CoordinatorDispatched event within
budget.

## Consequences

Execution can vary without changing public contracts or reducer behavior. An
integrating application owns execution safety, cancellation, usage accounting,
and external effects, while OVCA checks only the supplied records and their
workflow relationships.

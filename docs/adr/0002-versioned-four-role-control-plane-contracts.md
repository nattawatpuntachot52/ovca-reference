# ADR 0002: Versioned four-role workflow contracts

- Status: Accepted
- Contract version: 1.0
- Scope: Coordinator, Engineer, Reviewer, and Auditor workflow records under
  Owner authority

## Context

The public reference needs one domain-neutral sequence for delegating bounded
work, recording evidence, obtaining independent judgment, and returning a
public outcome. Owner supplies authority and handles escalation; the four
workflow roles perform coordination, implementation, review, and audit
responsibilities.

## Decision

The common attempt sequence is fixed:

~~~text
IntakeAccepted
  -> CoordinatorDispatched
  -> EngineerReturned
  -> VerificationRecorded
  -> ReviewerDecided
  -> AuditorCounterchecked
~~~

The corresponding records are Authority, ExecutionBudget, RoleInvocation,
RoleResult, VerificationOutcome, ReviewPacket, AuditPacket, RepairDirective,
Decision, FinalResponse, and WorkflowEvent.

The reducer constrains CoordinatorDispatched to a Coordinator request for an
Engineer on the current attempt. EngineerReturned must bind that invocation.
VerificationRecorded must bind that result. ReviewPacket binds the result and
verification; AuditPacket binds the review.

The initial dispatch input_refs array contains exactly the intake request
reference. A dispatch after RepairDirected contains exactly the preceding
directive identifier. This preserves an explicit input chain across attempts.

RoleResult reports succeeded or failed and introduces evidence records.
VerificationOutcome reports passed or failed and introduces evidence records.
A Reviewer cannot pass a failed Engineer result or failed verification. Review
and audit findings may cite only evidence already known to the reducer.
Evidence identifiers introduced by results and verification outcomes are
globally unique within one workflow.

## Branches

- Review pass plus audit concurrence sets a passed outcome and permits
  CoordinatorFinalized.
- Review repair plus audit concurrence permits RepairDirected while budget
  remains, then returns to Coordinator dispatch with the next attempt.
- Audit disagreement, or an exhausted repair budget, requires OwnerEscalated
  before CoordinatorFinalized with an escalated outcome.

FinalResponse must report the reducer's pending outcome and cite only known
evidence.

## Budget

ExecutionBudget declares max_repair_attempts and max_events. Semantic validation
requires max_events to be at least 8 + (6 * max_repair_attempts), preserving an
Owner escalation and final response after the last permitted repair.

## Consequences

The public workflow has an explicit, replayable control sequence and bounded
repair behavior. The contracts do not execute roles, schedule work, or persist
events; callers supply the complete records.

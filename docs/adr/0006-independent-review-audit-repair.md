# ADR 0006: Independent review, audit, and bounded repair

- Status: Accepted
- Contract version: 1.0
- Scope: Review, countercheck, repair, and escalation semantics

## Context

An Engineer result and verification outcome are evidence for judgment; they are
not a final decision. The public workflow needs separate Reviewer and Auditor
records and must prevent either record from mutating the candidate directly.

## Decision

ReviewerDecided carries a ReviewPacket bound to the current Engineer result and
verification outcome. ReviewDecision is pass or repair. Repair requires at
least one blocking finding; pass forbids blocking findings. A Reviewer pass is
also rejected when the Engineer result or verification failed.

AuditorCounterchecked carries an AuditPacket bound to the current review.
AuditDecision is concur or disagree. Disagreement requires a finding;
concurrence cannot contain a blocking finding. Review and audit packets may
refer only to evidence already introduced by the Engineer result or
verification outcome.

Audit occurs after every review:

- pass plus concur permits a passed CoordinatorFinalized response;
- repair plus concur permits RepairDirected while budget remains;
- disagree requires OwnerEscalated;
- repair plus concur with no remaining repair budget also requires
  OwnerEscalated.

RepairDirective binds the current review and audit, current attempt,
instructions, and known evidence. It grants no direct mutation capability.
After it is accepted, the reducer increments the attempt and requires
CoordinatorDispatched before another Engineer result.

OwnerEscalated carries an Owner Decision with the escalate disposition. It
records the escalation branch; CoordinatorFinalized then emits the public
escalated outcome.

## Determinism and bounds

The reducer tracks repair consumption and rejects attempts or directives that
do not match current state. ExecutionBudget reserves enough event capacity for
every declared repair and a final escalation path. Invalid review, audit,
repair, or escalation events leave reducer state unchanged.

## Consequences

Review and audit remain distinct evidence-bound judgments. Repairs return
through Coordinator and Engineer rather than being applied by Reviewer or
Auditor. The design demonstrates contract conformance and branch reduction; it
does not authenticate actors, execute work, inspect evidence content, or
validate a real operating workflow.

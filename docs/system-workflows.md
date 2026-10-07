# System Workflows

The public workflow is a deterministic reduction of caller-supplied events.
Each event uses contract version 1.0, a stable workflow and event identifier, a
zero-based sequence number, the exact preceding event identifier after sequence
zero, a caller-supplied timestamp, and one WorkflowEventKind variant.

## Common attempt sequence

| Order | Event | Required relationship | Next phase |
|---:|---|---|---|
| 1 | IntakeAccepted | Carries a request reference, public summary, Owner-granted Authority, and ExecutionBudget | AwaitingCoordinatorDispatch |
| 2 | CoordinatorDispatched | Coordinator invokes Engineer with the current attempt, intake authority reference, and exactly one current input reference | AwaitingEngineerResult |
| 3 | EngineerReturned | Engineer result refers to the current invocation and contributes evidence records | AwaitingVerification |
| 4 | VerificationRecorded | Verification refers to the current Engineer result and contributes evidence records | AwaitingReview |
| 5 | ReviewerDecided | Review refers to the current result and verification and may cite only known evidence | AwaitingAudit |
| 6 | AuditorCounterchecked | Audit refers to the current review and may cite only known evidence | Branch selection |

A Reviewer pass is rejected when the Engineer result failed or verification
failed. A repair decision requires at least one blocking finding. A pass
decision cannot contain a blocking finding.

## Pass and finalization

ReviewerDecided with pass followed by AuditorCounterchecked with concur sets the
pending outcome to passed. The only valid next event is CoordinatorFinalized
with a FinalResponse whose outcome is passed and whose evidence references are
already known.

~~~text
ReviewerDecided(pass)
  -> AuditorCounterchecked(concur)
  -> CoordinatorFinalized(passed)
~~~

## Bounded repair and redispatch

ReviewerDecided with repair followed by AuditorCounterchecked with concur enters
the repair branch. When repair budget remains, RepairDirected must bind the
current review, audit, attempt, instructions, and known evidence. It consumes
one repair attempt, increments the role attempt, and returns the reducer to
AwaitingCoordinatorDispatch.

~~~text
ReviewerDecided(repair)
  -> AuditorCounterchecked(concur)
  -> RepairDirected
  -> CoordinatorDispatched
  -> EngineerReturned
  -> VerificationRecorded
  -> ReviewerDecided
  -> AuditorCounterchecked
~~~

The repeated attempt is subject to the same result, verification, review,
audit, evidence, and finalization rules as the first attempt.

The initial CoordinatorDispatched input_refs array must contain only the
IntakeAccepted request_ref. After RepairDirected, the next dispatch input_refs
array must contain only that directive_id.

## Owner escalation and finalization

Auditor disagreement enters AwaitingOwnerEscalation regardless of the review
decision. A repair decision also enters that phase when the declared repair
budget is exhausted. OwnerEscalated must carry a Decision made by Owner with
the escalate disposition and known evidence references. CoordinatorFinalized
then closes the stream with an escalated outcome.

~~~text
AuditorCounterchecked(disagree)
  -> OwnerEscalated
  -> CoordinatorFinalized(escalated)
~~~

~~~text
ReviewerDecided(repair)
  -> AuditorCounterchecked(concur)
  -> OwnerEscalated
  -> CoordinatorFinalized(escalated)
~~~

## Stream invariants

The reducer rejects an event when any of these checks fail:

- workflow identifiers change, sequence numbers skip, predecessor links do not
  match, or an event identifier repeats;
- the event arrives in a phase where its variant is not permitted;
- the declared event budget is exceeded;
- Coordinator or Engineer role bindings, invocation references, result
  references, review references, or attempt numbers do not match current state;
- a dispatch does not contain exactly the current request or repair-directive
  input reference;
- an Engineer result or verification outcome introduces an evidence identifier
  that already appeared in either kind of record;
- review, audit, repair, decision, or final response records cite evidence that
  has not already been introduced by a result or verification outcome;
- review and audit decisions select an unsupported branch; or
- CoordinatorFinalized reports an outcome different from the pending outcome.

ExecutionBudget requires enough events to preserve the escalation and final
response path after every declared repair: max_events must be at least
8 + (6 * max_repair_attempts).

## Recovery

recover_public replays a WorkflowEvent slice from a fresh WorkflowReducer and
returns a PublicWorkflowSnapshot. The public projection contains exactly phase,
event_count, repair_attempts, outcome, evidence_count, and
final_response_present. It exposes no caller-controlled identifiers or text and
does not return FinalResponse. The input WorkflowEvent records remain unchanged
and retain their caller-supplied fields. evidence_count is the number of unique
Evidence records introduced by EngineerReturned and VerificationRecorded;
later references do not add to it.

## Synthetic examples

- examples/pass.json reaches a passed final response without repair.
- examples/repair_then_pass.json consumes one repair and then reaches a passed
  final response.
- examples/disagreement_escalation.json records audit disagreement, Owner
  escalation, and an escalated final response.

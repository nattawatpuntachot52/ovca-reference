# ADR 0007: Pre-intake admission, authorization mandates, and observe mode

- Status: Accepted
- Contract version: 1.0
- Scope: Pre-intake admission policy, bounded mandates, follow-up handback, and observe-mode calibration

## Context

`ovca-types` and `ovca-workflow` deliberately start at `IntakeAccepted`, where an `Authority` record is already supplied by the caller. When an external harness listens to repository events (issues, pull requests, comments, bots, or default-branch CI), two pre-intake failure modes arise before `IntakeAccepted`:

1. **Context confused with authority (`Context != Authority`):** Thread subscriptions, bot comments, CI notifications, or out-of-scope follow-up remarks can be mistaken for permission to resume or widen an existing task.
2. **Uncalibrated proactive automation:** Turning automated triggers directly on or off provides no safe middle state to measure rule precision without mutating the repository, and evaluator outages can be silently misclassified as benign `ignore` outcomes.

## Decision

Add a third pure-Rust workspace crate, `ovca-admission`, alongside matching Draft 2020-12 JSON Schemas and synthetic samples, without modifying any `ovca-types` or `ovca-workflow` v1.0 wire shapes:

1. **`AdmissionPolicy` and canonical `policy_version`:**
   - Separates `allowed_actors`, `bot_allowlist`, and `automated_reviewers_non_bot` into disjoint sets.
   - Defines per-surface `AdmissionTableRow` entries across `owner_cli`, `github_issue`, `github_pr`, `github_comment`, and `github_default_branch_ci`.
   - Computes a deterministic `sha256:<64 hex>` `policy_version` over canonical sorted JSON using an internal FIPS 180-4 SHA-256 implementation with zero additional crate dependencies.
2. **Immutable `AuthorizationMandate` and `Authority` binding:**
   - Binds a mandate to `Authority.authority_id` and enforces segment-safe repository-relative containment (`is_path_within_scope`) so `writable_scope` never widens `Authority.scope` or falls prey to prefix confusion (`rust/ovca-types` vs `rust/ovca-types-extra`) or `..` traversal.
   - Prohibits bot logins and default-branch CI triggers from holding `explicit` mandate strength.
   - Revalidates mandates (`revalidate_mandate`) against the current `AdmissionPolicy` before resuming work, revoking `inferred` mandates if `accept_mode` leaves `auto`, a matched rule is disabled, or its `writable_scope` narrows.
3. **Follow-up isolation and `HandbackRecord`:**
   - `evaluate_followup` delivers `context_only` subscriptions, secondary threads, bots, CI events, and automated reviewers strictly without authority or stop capability.
   - Human follow-ups requesting paths outside `AuthorizationMandate.writable_scope` do not mutate the mandate; they return `FollowUpAction::HandbackRequired` with a `HandbackRecord` (`route_to: "fresh_eval"`).
4. **Three-state `AcceptMode` (`off | observe | auto`) and `unavailable != ignore`:**
   - `evaluate_proactive_admission` evaluates candidate rule matches against enabled policy rules and intersects `writable_scope` across matched rules.
   - In `observe` mode, qualifying triggers emit an `AdmissionDecisionRecord` with `decision = "ignore"`, `would_accept = true`, and `None` for the mandate.
   - Evaluator failures (`AssessmentInput::Unavailable`) emit `decision = "unavailable"` with a required `error_kind` and `would_accept = false`, never collapsing operational outages into `ignore`.

## Determinism and bounds

`ovca-admission` performs no network, clock, or filesystem I/O. Callers supply timestamps, identifiers, policies, triggers, and rule-match candidates. `Cargo.lock` gains no external dependencies.

## Consequences

`ovca-types` and `ovca-workflow` remain minimal and backward-compatible while integrating applications gain deterministic reference contracts for pre-intake admission, scope-bounded mandates, follow-up handback, and non-mutating `observe` calibration. The library still does not authenticate actor identities at the transport layer; callers remain responsible for verifying webhook or CLI provenance.

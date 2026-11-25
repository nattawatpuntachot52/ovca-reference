---
name: Bounded Task Brief (Medium / High-Impact Work)
about: Canonical task brief for contract additions, reducer/admission changes, skills, or multi-file work (not required for small isolated fixes).
title: "[TASK] "
labels: []
assignees: []
---

> **Minimum Sufficient Collaboration:** Use this template for medium or high-impact changes (contract additions, crate/reducer changes, reusable skills, or governance updates). Trivial single-file fixes may proceed directly via pull request.

## Outcome
- Describe the observable result that must hold when this task is accepted:

## Canonical Source
- **Repository:** `nattawatpuntachot52/ovca-reference`
- **Related Paths:**
- **Starting Branch / Commit SHA:**

## Ownership
- **Accountable Human Owner:** `@nattawatpuntachot52`
- **Execution Lead:**
- **Independent Reviewer / Auditor:**

## Scope & Permissions
- **Allowed Read Paths:**
- **Allowed Write Paths (`writable_scope`):**
- **Explicitly Excluded Paths / Prohibited Actions:** (e.g., no breaking changes to `contract_version = "1.0"`, no external service integrations, no machine-specific paths or secrets)

## Acceptance Criteria
- [ ] Criterion 1 (observable and deterministic)
- [ ] Required `cargo fmt / check / test / clippy --locked` checks pass on head revision
- [ ] Pull request includes an accurate `AGENT_ACK` disclosure block

## Risk & Resource Boundary
- **Risk Level:** `Low` | `Medium` | `High`
- **Reason (if Medium/High):**
- **Repair / Retry Bound:**

## Expected Evidence
- Changed paths and head commit SHA
- Exact verification commands executed and observed outcomes (`pass` / `fail` / `not run`)
- ADR under `docs/adr/` if introducing a material contract or boundary decision

## Stop Conditions
- Stop and request explicit owner direction if `writable_scope` must expand, `contract_version = "1.0"` compatibility is threatened, or acceptance criteria conflict.

---

## Agent Acknowledgement (Fill Before Substantive Work)
- **Agent / Assignee:**
- **Actual Execution Environment:**
- **Model & Version (or `UNKNOWN (not exposed by runtime)`):**
- **Starting Revision:**
- **Confirmed Outcome, Acceptance Criteria & Scope:** `Yes` | `Needs clarification`
- **Missing Access / Blockers:**
- **Planned First Action:**

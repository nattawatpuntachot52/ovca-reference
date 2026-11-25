# ADR 0008: Lightweight GitHub operational baseline

- Status: Accepted
- Scope: Repository review ownership, bounded task briefs, pull-request `AGENT_ACK` disclosure, and capability evidence ledger

## Context

`ovca-reference` demonstrates evidence-bound role workflows in code, yet contributors and coding agents interacting through GitHub pull requests benefit from lightweight, repository-native templates that record task boundaries, current-head verification, and execution environment facts without adding external services or heavy ceremony for small fixes.

## Decision

Adopt four static operational artifacts in `ovca-reference`:

1. **`.github/CODEOWNERS`:** Routes review ownership on `/contracts/`, `/examples/`, `/rust/`, `/skills/`, `/.github/`, `/docs/adr/`, and `/docs/operations/` to the repository maintainer.
2. **`.github/ISSUE_TEMPLATE/task.md`:** Provides an optional `Bounded Task Brief` for medium or high-impact work declaring `writable_scope`, acceptance criteria, stop conditions, and pre-work agent acknowledgement (`Minimum Sufficient Collaboration`).
3. **`.github/pull_request_template.md`:** Requires explicit current-head reporting for the four `--locked` Cargo checks (`No Implicit Pass`), parallel semantic-conflict awareness, and a structured `AGENT_ACK` block using `UNKNOWN (not exposed by runtime)` whenever a host harness hides exact model build identifiers.
4. **`docs/operations/agent-inventory.md`:** Establishes a provider-neutral ledger structure for recording verified execution outcomes by task category.

## Consequences

Pull requests carry standardized boundary and verification disclosures aligned with OVCA's evidence-bound philosophy, while the repository remains dependency-free, domain-neutral, and free of external runtime couplings.

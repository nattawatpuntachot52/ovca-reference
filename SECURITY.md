# Security Policy

OVCA validates typed records and deterministically reduces caller-supplied
events. It does not authenticate actors, admit authority, inspect evidence
content, execute roles, or persist workflow state. Integrations that add those
capabilities require their own security design and review.

The maintained code is the current default branch. Reports should identify the
affected contract or reducer rule, the smallest reproducible event sequence,
the observed result, and the expected invariant.

Use GitHub private vulnerability reporting when it is available. If it is not
available, open a minimal public issue asking maintainers for a private contact
channel. Do not publish exploit details or sensitive content in the issue.

See [docs/security-boundary.md](docs/security-boundary.md) for the complete
trust boundary.

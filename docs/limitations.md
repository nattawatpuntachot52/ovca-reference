# Limitations

- OVCA is a contract and reducer reference, not a complete orchestration
  system.
- The library does not execute Coordinator, Engineer, Reviewer, Auditor, or
  verification work. Events are supplied by the caller.
- Role enum values and event variants express responsibility but do not
  authenticate people or prove actor independence.
- Authority is declarative. Validation does not perform admission or establish
  that the asserted Owner granted it.
- Evidence validation checks metadata and references only. It does not inspect
  evidence content, calculate content digests, or establish truth.
- Timestamp validation requires nonempty caller-supplied text containing T and
  ending in Z. It does not parse calendar values, compare event time, or provide
  a trusted clock.
- State exists only in the WorkflowReducer value. The crates provide no durable
  log, concurrency control, or coordination between callers.
- PublicWorkflowSnapshot omits caller-controlled identifiers, text, evidence
  records, and FinalResponse. This minimized output does not redact or rewrite
  the caller-supplied WorkflowEvent records from which it is derived.
- The repair branch emits and validates RepairDirective data. It does not apply
  the instructions or create the next Engineer result.
- Event and repair budgets constrain accepted streams; they do not meter time,
  compute, or external effects.
- A completed PublicWorkflowSnapshot proves that the supplied records conform
  to reducer rules. It does not prove the quality of implementation,
  verification, review, audit, or final summary.
- Contract tests parse schema files as JSON and test Rust deserialization plus
  semantic validation of samples. They do not run a JSON Schema engine.
- The examples are synthetic conformance fixtures. No real workflow validation
  or production-readiness claim is included.

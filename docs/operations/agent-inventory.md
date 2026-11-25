# Agent & Execution Environment Inventory

This document provides a provider-neutral operational ledger for recording execution harnesses, tool boundaries, and observed verification outcomes on `ovca-reference` without coupling workflow roles to fixed model vendors.

> **Epistemic Rule (No Implicit Claims):** Never infer an exact model build identifier from a display label. When a runtime does not expose its exact model build string, record `UNKNOWN (not exposed by runtime)`. Always state observed verification results alongside sample size and scope constraints.

---

## 1. Execution Harness Schema

| Field | Requirement |
|---|---|
| **Harness / Runtime** | Record the actual CLI, IDE, or CI runner and OS executing the change |
| **Model Identifier** | Exact runtime-exposed identifier or `UNKNOWN (not exposed by runtime)` |
| **Writable Boundary** | Repository-relative `writable_scope` on a non-default feature branch |
| **Network & Secret Posture** | Offline `--locked` Cargo verification; zero secrets, personal data, or local host paths in committed artifacts |

---

## 2. Task Categories & Required Verification

| Task Category | Execution Pattern | Required Verification |
|---|---|---|
| **Contract & Schema Additions** | Update Rust type + `Validate`, Draft 2020-12 JSON Schema, synthetic sample, and ADR together | `cargo fmt / check / test / clippy --locked` + missing-field and unknown-field wire rejection tests |
| **Deterministic Reducer / Admission Logic** | Pure synchronous Rust with zero I/O or external crate additions unless explicitly reviewed | Positive conformance fixtures + adversarial boundary/rejection tests |
| **Reusable Public Skills & Docs** | Self-contained, domain-neutral Markdown with synthetic examples only | Link integrity check + public boundary / secret scan |

---

## 3. Capability Evidence Log

| Date (UTC) | Ref | Task Category | Execution Environment | Model ID / Version | Observed Verification | Notes |
|---|---|---|---|---|---|---|
| `2026-11-24` | `main` (`d0261b2`, `5b0726c`) | Public v0.1 contracts, deterministic `WorkflowReducer`, and `zoom-out` skill | Reference baseline + GitHub Actions `windows-latest` | `UNKNOWN (not exposed by runtime)` | `25/25` Rust contract and workflow fixture tests passing under `--locked` | Establishes initial v1.0 wire contract baseline |

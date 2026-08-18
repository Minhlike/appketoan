# Current Task: Prompt 1 — Multi-Source Requirements, Accounting Rules & Data Contract

## Task Description
Formalize multi-source accounting requirements, define reconciliation matching semantics, design the canonical data contract, and implement the Rust/TypeScript core data models with rigorous automated tests.

## Scope & Acceptance Criteria
- [x] Requirements specification documented in `docs/01-requirements/01-multi-source-reconciliation.md`.
- [x] Accounting reconciliation scenarios (1-1, 1-N, N-1, N-M, aggregate, partial, missing, duplicate, ambiguous, field discrepancies) documented in `docs/02-accounting-rules/01-matching-scenarios.md`.
- [x] Vietnamese accounting practices and e-invoice standards documented in `docs/02-accounting-rules/02-vietnamese-accounting-standards.md`.
- [x] Canonical Data Contract & schemas defined in `docs/03-data-contract/`.
- [x] Rust core models implemented in `crates/reconciliation-core/src/models/` with serde serialization, normalization, and invariant validation.
- [x] TypeScript interfaces synchronized in `src/types/dataContract.ts`.
- [x] Synthetic fixtures created in `fixtures/synthetic/` and `fixtures/expected/`.
- [x] Unit tests for data models, validation, and contract integrity PASS in Rust and TypeScript.
- [x] Memory files (`PROJECT_STATE.md`, `CURRENT_TASK.md`, `DECISIONS.md`, `HANDOFF.md`, `TEST_STATUS.md`) updated.
- [x] Clean Git commit created.

## Task Status
- **Status**: COMPLETED & READY FOR PROMPT 2

# Project State

## Product Goal
A standalone, 100% offline desktop application for Windows to perform automated multi-source accounting data reconciliation across diverse Excel workbooks (electronic invoices, general ledgers TK 511/3331/131/133, cash books, bank statements, multi-branch books, custom workbooks).

## Core Technical Stack
- **Desktop Shell**: Tauri v2 (native Windows WebView2)
- **Local Engine (Domain Crate)**: `reconciliation-core` (Rust standalone crate, zero UI coupling)
- **Toolchain**: Rust stable GNU (`x86_64-pc-windows-gnu`) with GCC 16.2.0 (w64devkit)
- **Frontend**: React 19 + TypeScript + Vite + Vanilla CSS
- **Testing**: Vitest + Testing Library (Frontend), `cargo test` (Rust Domain Crate)
- **VCS**: Git

## High-Level Architecture
```text
UI Layer (React 19 / TypeScript in src/)
    ↓ IPC Commands / Events
Desktop Shell (Tauri 2 in src-tauri/)
    ↓ Rust crate dependency
Reconciliation Core (crates/reconciliation-core/)
    ├── Models (CanonicalRecord, DataSource, MatchingRule, ReconciliationResult)
    ├── Parser Engine (Calamine streaming reader - Phase 2)
    └── Matching Engine (Multi-source reconciliation algorithms - Phase 3)
```

## Current Phase
- **Phase 0 (Bootstrap)**: COMPLETED
- **Phase 1 (Requirements, Accounting Rules & Data Contract)**: COMPLETED & VERIFIED
- **Phase 2 (Excel Ingestion & Parser Engine)**: PENDING PROMPT 2

## Completed Components
- Repository structure, Git baseline, and strict security filters (`.gitignore`).
- Long-term memory system in `.agent/` and operating rules in `AGENTS.md`.
- Functional Requirements Specification (`docs/01-requirements/`).
- Accounting Rules & Vietnamese Practice Standards (`docs/02-accounting-rules/`).
- Canonical Data Contract Specifications (`docs/03-data-contract/`).
- Independent domain crate `crates/reconciliation-core` implementing `CanonicalRecord`, `DataSource`, `MatchingRule`, `ReconciliationResult` with normalization and invariant validation.
- Synchronized TypeScript interfaces in `src/types/dataContract.ts`.
- Synthetic test fixtures and expected results in `fixtures/synthetic/` and `fixtures/expected/`.
- Automated test suites (5 Vitest tests + 6 Rust unit tests) passing 100%.

## Pending Components
- Local Excel ingestion & high-speed streaming parser in Rust using Calamine (Prompt 2).
- Dynamic header detection & column auto-mapping heuristics (Prompt 2).
- Multi-source reconciliation matching algorithms (1-1, 1-N, N-M, aggregate, tolerance) (Prompt 3).
- Interactive reconciliation UI and discrepancy inspector (Prompt 4).
- Production packaging and Windows installer pipeline (Prompt 5).

## Blockers
- None.

## Release Status
- v0.1.0-alpha.1 (Prompt 1 Complete).

# Project State

## Product Goal
A standalone, 100% offline desktop application for Windows to perform automated multi-source accounting data reconciliation across diverse Excel workbooks (electronic invoices, general ledgers TK 511/3331/131/133, cash books, bank statements, multi-branch books, custom workbooks).

## Core Technical Stack
- **Desktop Shell**: Tauri v2 (native Windows WebView2)
- **Local Engine (Domain Crate)**: `reconciliation-core` (Rust standalone pure crate, zero UI coupling)
- **Toolchain**: Rust stable GNU (`x86_64-pc-windows-gnu`) with GCC 16.2.0 (w64devkit)
- **Frontend**: React 19 + TypeScript + Vite + Vanilla CSS
- **Testing**: Vitest (Frontend), `cargo test` (Rust Domain Crate & Golden Datasets)
- **Security**: 100% Offline Air-Gapped Mode (Zero Telemetry, Zero Cloud, Zero AI APIs)
- **VCS**: Git

## High-Level Architecture
```text
UI Layer (React 19 / TypeScript in src/)
    ↓ IPC Commands / Events
Desktop Shell (Tauri 2 in src-tauri/)
    ↓ Rust crate dependency
Reconciliation Core (crates/reconciliation-core/)
    ├── Models (CanonicalRecord, DataSource, MatchingRule, ReconciliationResult)
    ├── Parser Engine (Calamine streaming reader - Next Phase)
    ├── Matching Engine (Deterministic multi-pass matching pipeline - Next Phase)
    └── Discrepancy Analyzer & Report Generator
```

## Current Phase
- **Pre-Development Preparation**: 100% COMPLETED & AUDITED
- **Next Phase**: READY_FOR_IMPLEMENTATION (Implementation of Calamine Ingestion & Matching Engine)

## Completed Preparation Deliverables
- Multi-Source Requirements & UX Workflows (`docs/01-requirements/`).
- Accounting Rules, VAS Standards & 5 Built-in Profiles (`docs/02-accounting-rules/`).
- Canonical Data Contract & Normalization Specs (`docs/03-data-contract/`).
- Engine Pipeline Architecture & $O(N)$ Indexing Strategy (`docs/04-architecture/`).
- Test Strategy & Performance Benchmark Targets (`docs/05-testing/`).
- Offline Security & Zero-Exfiltration Policy (`docs/06-security/`).
- Windows Production Release Strategy (`docs/07-release/`).
- Standalone Domain Crate (`crates/reconciliation-core`) with serialization, normalizers, and invariants.
- Synchronized TypeScript types in `src/types/dataContract.ts`.
- Comprehensive synthetic fixtures & golden test results in `fixtures/`.
- 12 automated test suites passing 100% in $< 1$ second.

## Blockers
- NONE.

## Release Status
- v0.1.0-alpha.1 (Pre-Development Ready).

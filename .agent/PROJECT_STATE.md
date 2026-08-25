# Project State

## Product Goal
A standalone, 100% offline desktop application for Windows to perform automated multi-source accounting data reconciliation across diverse Excel workbooks (electronic invoices, general ledgers TK 511/3331/131/133, cash books, bank statements, multi-branch books, custom workbooks).

## Core Technical Stack
- **Desktop Shell**: Tauri v2 (native Windows WebView2)
- **Local Engine (Domain Crate)**: `reconciliation-core` (Rust standalone pure crate, zero UI coupling)
- **Toolchain**: Rust stable GNU (`x86_64-pc-windows-gnu`) with GCC 16.2.0 (w64devkit)
- **Precision Accounting Type**: `rust_decimal::Decimal` (Zero float rounding inaccuracies)
- **Frontend**: React 19 + TypeScript + Vite + Vanilla CSS
- **Testing**: Vitest (Frontend), `cargo test` (Rust Domain Crate, Golden Datasets & Performance Benchmarks)
- **Security**: 100% Offline Air-Gapped Mode (Zero Telemetry, Zero Cloud, Zero External APIs)
- **VCS**: Git

## High-Level Architecture
```text
UI Layer (React 19 / TypeScript in src/)
    ↓ IPC Commands (cmd_inspect_excel_file, cmd_inspect_excel_bytes, cmd_run_reconciliation, cmd_export_reconciliation_report)
Desktop Shell (Tauri 2 in src-tauri/)
    ↓ Rust crate dependency
Reconciliation Core (crates/reconciliation-core/)
    ├── Models (CanonicalRecord, DataSource, MatchingRule, ReconciliationResult, ExcelFileMetadata, ExportSummary)
    ├── Reader Engine (Calamine streaming reader, Vietnamese accounting keyword sniffer & header auto-detector)
    ├── Normalizer Engine (DocNo, TaxID, Excel float & string dates, Decimal amounts, full-row garbage filter, empty amount filter)
    ├── Matching Engine (Pass 0: duplicates, Pass 1: exact keys & tolerance, Pass 2: tax ID + amount, Pass 3: residual sweep, 1-to-N aggregate)
    ├── Discrepancy Analyzer (Detailed field diffs & Vietnamese audit explanations with Decimal formatting)
    └── Excel Exporter (3-tab formatted audit workbook using rust_xlsxwriter)
```

## Current Phase
- **Reconciliation Correctness & Accounting Invariants**: 100% FIXED & VERIFIED
- **Application Status**: Fully functional, tested with rigorous regression test suite against real-world accounting datasets.

## Production Artifacts
- **NSIS Setup Installer**: `src-tauri/target/release/bundle/nsis/appketoan_0.1.0_x64-setup.exe` (4.99 MB)
- **WiX MSI Installer**: `src-tauri/target/release/bundle/msi/appketoan_0.1.0_x64_en-US.msi` (7.78 MB)
- **Portable Executable**: `src-tauri/target/release/tauri-app.exe` (26.4 MB)
- **Source Archive**: `D:\appketoan-source.zip` (404.67 KB)

## Blockers
- NONE.

## V15 Local Dataset Gate (2026-08-25)
- COMPLETE: the user confirmed the seven suffixless workbooks in `.local-testdata/` are the authoritative local acceptance set. They were read only through the local Rust reader and remain ignored by Git.
- V15 adds typed handling for partner masters and sales analysis, directional safeguards for TK112/bank matching, adaptive header detection, and additive UI mapping/source-kind support.
- The immutable reconciliation baseline remains covered by the real-local regression suite.

## V15 Blocker Closure (2026-08-25)
- Matching executes every explicit rule for a source pair; a source pair is no longer reduced to its first rule.
- TK112/bank candidate selection rejects opposite directions before exact, tolerance, fallback, or aggregate acceptance.
- One-source partner-master and sales-analysis controls run through typed local normalization and return dedicated control results instead of transaction groups.
- A mapped invoice lifecycle is typed at execution and non-standard/unknown values are fail-closed.

## Latest Verification (2026-08-25)
- GNU workspace test suite completed successfully after the v14 intake and packaging changes.
- Frontend Vitest suite completed successfully.
- Repository source changes were committed as `b2e8658`.
- Root `Cargo.lock` is tracked to make the Rust dependency graph reproducible for repository handoff.
- The `main` branch has been published to the configured GitHub `origin`.

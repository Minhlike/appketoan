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
- No correctness or build blocker. Multi-control execution time and unmeasured peak memory remain performance follow-up items.

## V15 Local Dataset Gate (2026-08-25)
- COMPLETE: the user confirmed the seven suffixless workbooks in `.local-testdata/` are the authoritative local acceptance set. They were read only through the local Rust reader and remain ignored by Git.
- V15 adds typed handling for partner masters and sales analysis, directional safeguards for TK112/bank matching, adaptive header detection, and additive UI mapping/source-kind support.
- The immutable reconciliation baseline remains covered by the real-local regression suite.

## V15 Blocker Closure (2026-08-25)
- Matching executes every explicit rule for a source pair; a source pair is no longer reduced to its first rule.
- TK112/bank candidate selection rejects opposite directions before exact, tolerance, fallback, or aggregate acceptance.
- One-source partner-master and sales-analysis controls run through typed local normalization and return dedicated control results instead of transaction groups.
- A mapped invoice lifecycle is typed at execution and non-standard/unknown values are fail-closed.

## V15 Control-Plan Follow-up (2026-08-25)
- Added the additive generic `LedgerEntry` view while retaining legacy account-kind adapters.
- Each secondary source now builds one physical source index and compiles all of its semantic controls against it.
- Bank candidates can be accepted without document number or tax ID only on deterministic direction/amount/date evidence; otherwise they fail closed to review.
- Transactional and typed reference controls can run together through IPC. The new tri-source UI scenario covers invoice, sales register, and TK511.
- Aggregate search is capped at 12 candidates and stops after the second valid subset; gross discrepancy reporting cannot net different semantics to zero.

## V15 Semantic Closure (2026-08-25)
- Result presentation is source-aware: semantic labels include their actual secondary source and no longer imply fixed ledger accounts.
- Canonical bank fields and a read-only cross-source partner identity control are additive; they preserve source provenance and never auto-merge by fuzzy name.
- Aggregate controls are fail-closed over the candidate budget; aggregate-disabled paths scan all 1:1 candidates without subset enumeration.

## Latest Verification (2026-08-25)
- GNU workspace test suite completed successfully after the v14 intake and packaging changes.
- Frontend Vitest suite completed successfully.
- Repository source changes were committed as `b2e8658`.
- Root `Cargo.lock` is tracked to make the Rust dependency graph reproducible for repository handoff.
- The `main` branch has been published to the configured GitHub `origin`.

## V15.4 Final Acceptance In Progress (2026-08-25)
- The feature branch remains unmerged. Bank candidate policy now requires unique strong evidence for acceptance; a unique direction/amount/date candidate is review-only and remains unconsumed.
- Transactional execution derives its primary only after typed reference controls are removed. Partner identity evaluates declared transactional kinds only.
- Direct 1:1 candidates are scanned across the full set before aggregate subset search. Aggregate search remains fail-closed above its candidate budget.
- The local acceptance harness is ignored, path-based, and does not retain workbook content. It completed against the required local workbook set; running-balance validation is unavailable when normalized balance values are absent.
- Final V15.4 workspace checks, frontend checks, formatting, strict Clippy, Tauri build, and release executable smoke all completed. The branch is ready for source/diff review only and remains unmerged.

## V15 Final Micro-Fix (2026-08-25)
- Aggregate matching now follows a strict direct-first state machine: multiple direct matches are ambiguous, one direct match is accepted without subset search, and aggregate subsets are considered only with no direct match and at most the configured candidate budget.
- Bank secondary records have disjoint final classifications for accepted, review-linked, and truly unlinked state. Suggested/ambiguous links remain review evidence but no longer produce duplicate bank-only residual groups.
- The ignored real-local harness asserts every known oracle and proves bank-record classification conservation. Running-balance verification remains unavailable only because normalized balance evidence is insufficient.
- Full release gate and executable smoke passed. The feature branch remains unmerged and is ready for final diff review.

## V15 Merge Freeze (2026-08-25)
- Status: **MERGED / FROZEN**. PR #1 was squash-merged into `main` as `c5b03da4ed44eb9f5c88886256698beeafc7b4f9` after its head was locked to the approved revision.
- Post-merge gate on `main`: Rust workspace tests, frontend tests, TypeScript typecheck, Rust format check, strict workspace Clippy, Tauri release build, and release executable smoke all passed.
- The fresh ignored NSIS release candidate is under `target/release/bundle/nsis/`; its SHA256 is `936B775D65BD7D3BF5FD08CF60DCCC6CF2E9328D8331D2D2E7E4D413BE981E76`.
- Official limitations remain: TK112 running balance is `NOT_VERIFIED`; the real-data bank run has no strong auto-accepted match; Suggested/Ambiguous bank evidence is review-only; tri-source benchmark performance remains a follow-up rather than a correctness blocker.
- No V15 feature, account-kind expansion, architecture rewrite, or performance refactor is authorized during the freeze.

## V16 Audit Workspace / Control Planner (2026-08-25)
- Status: implementation complete on `codex/v16-control-planner`; not merged.
- The default workflow is now an additive “Bộ hồ sơ kế toán” workspace with a mandatory accounting period, capability-based source catalog, declarative control plans, and independent control results. Scenario execution remains available under advanced settings.
- Implemented control definitions: revenue invoice/register/ledger, bank/ledger, partner identity, and sales analysis. TK131 receivable and TK3331 VAT accounting are planning definitions only; no new account feature was implemented.
- Transactional data is bounded by the session period and then by each control's required-source date intersection. Missing dates, empty intersections, and duplicate required capabilities fail closed to review.
- Each source is read, normalized, and prepared once per audit execution; controls reuse the retained typed datasets and preserve provenance.
- The real-local V15 and V16 acceptance harnesses passed without persisting workbook data. The full Rust/frontend/type/format/Clippy/Tauri gate passed; the known GNU WebView2 linker warning remains non-fatal.

## V17 Resilience / UI Foundation (2026-08-26)
- Status: implementation complete on `codex/v17-resilience-ui-foundation`, stacked on the unmerged V16 branch; not merged.
- Audit reports now carry typed errors, independent control outcomes, structured stage/control metrics, cooperative cancellation state, and accounting-specific result summaries.
- Tauri owns a bounded app-process prepared-source cache keyed by source identity, content SHA-256, sheet, mapping, source kind, and normalization schema. Period views and indexes are rebuilt fail-closed; no workbook content is persisted by the cache.
- The default React workflow is decomposed into source intake, period, planner/results, and unified review components. The legacy scenario workflow remains available under advanced settings.
- Office lock files and exact duplicates are reported; native Tauri drag/drop is path-first while browser/file-picker bytes remain a compatibility fallback.
- Local acceptance, full Rust/frontend/type/format/Clippy gates, Tauri production build, and release executable smoke passed. Peak RAM, pure IPC serialization time, inner-loop cancellation, and automated browser visual inspection remain explicitly unverified.

## V18 Document Integrity / Field-Level Reconciliation (2026-08-26)
- Status: implementation and release gate complete on `codex/v18-document-integrity`; stacked PR #4 targets the unmerged V17 branch and is not merged.
- The revenue tri-source control now has an authoritative typed document result. BK validation, exact five-field Thuế/BK comparison, strict three-field TK511 comparison, diagnostic-only links, independent totals, and provenance-preserving multi-error cases are implemented in the pure Rust core.
- Invalid-date records stay outside auto-match indexes but remain available to the Review Queue. Valid records retain the narrow per-control period intersection.
- The Audit Workspace shows document summaries, a field-level table, side-by-side evidence, and every error code without changing the advanced scenario workflow.
- Synthetic adversarial coverage, local ignored acceptance, all Rust/frontend/type/format/Clippy gates, Tauri release build, and executable smoke passed. The legacy compatibility result still adds material 100k runtime and automated browser visual inspection remains unverified.

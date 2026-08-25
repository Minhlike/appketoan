# Current Task

## Task Name
V17 — FUTURE-READY PRODUCT / RESILIENCE / UI FOUNDATION

## Objectives & Status
1. [x] Migrate all monetary values to `rust_decimal::Decimal` (eliminating `f64` float rounding issues across ingestion, models, rules, engine, discrepancies, and export).
2. [x] Fix Header Sniffer & Column Mapping:
   - Added distinct `debit_amount_column` and `credit_amount_column` in `ColumnMapping` and `MappingModal`.
   - Removed "Phát sinh Nợ" and "Phát sinh Có" from `total_amount` detection.
   - Accurately mapped "Phát sinh Có" $\rightarrow$ `credit_amount_column` and "Phát sinh Nợ" $\rightarrow$ `debit_amount_column`.
3. [x] Fix Accounting Profile Rules (Revenue Matching):
   - Configured `Invoice.pretax_amount` matching against `TK511.credit_amount`.
4. [x] Fix Full-Row Garbage & Summary Detection:
   - Scanned all columns for "SỐ DƯ ĐẦU KỲ", "PHÁT SINH TRONG KỲ", "SỐ DƯ CUỐI KỲ", "TỔNG CỘNG", "CỘNG PHÁT SINH", etc.
   - Result: Exactly 45 valid TK511 rows out of 49 raw rows.
5. [x] Fix Empty-Amount Invoice Rows:
   - Rows with doc numbers but empty/zero monetary amounts are excluded from reconciliation scope rather than falsely flagged as missing.
   - Result: Exactly 46 valid invoice rows out of 64 raw rows.
6. [x] Fix Missing Document Detection:
   - Correctly identified Invoice `#233` (Date `06/07/2026`, Pretax `105.000.000` đ, Tax `10.500.000` đ, Total `115.500.000` đ) as Missing in TK511.
   - Financial variance: `105.000.000` đ.
   - 45 Exact Matches, 0 Amount Mismatches, 0 Missing in Invoice.
7. [x] Multi-Source Architecture & Source Identity:
   - Preserved per-source match breakdown (`SourceMatchBreakdown`, `source_breakdowns`).
8. [x] Regression & Acceptance Test Verification:
   - 17 Rust tests passed (`cargo test`).
   - 10 Vitest frontend tests passed (`npm test`).
   - Production Vite frontend build passed (`npm run build`).

## Handover Verification — 2026-08-25
1. [x] Reviewed the project handoff, current state, decisions, and test records before making changes.
2. [x] Re-ran the Rust workspace test suite with the required GNU toolchain.
3. [x] Re-ran the frontend Vitest suite.
4. [x] Reviewed, scoped, and committed the inherited v14 source changes.
5. [x] Kept generated portable artifacts out of source control.

## Next Task
V17 is implemented and verified on `codex/v17-resilience-ui-foundation`. Publish it as a PR stacked on `codex/v16-control-planner`, then stop for ChatGPT source/diff review. Do not merge either PR, add TK131/TK3331 behavior, or begin another feature.

## V16 Objectives & Status
1. [x] Add an explicit-period `AuditSession`, `SourceCatalog`, normalized dataset cache, prepared source indexes, and `ControlPlan` collection.
2. [x] Define controls declaratively by source capabilities rather than filenames or scenario selection.
3. [x] Preserve legacy account-kind adapters while allowing content-derived generic ledger capabilities.
4. [x] Generate READY, MISSING_SOURCE, NEEDS_MAPPING, NEEDS_REVIEW, and NOT_APPLICABLE plans fail-closed.
5. [x] Filter session data and compute a safe per-control date intersection before matching.
6. [x] Wire the audit workspace through Tauri IPC and the default React dashboard; keep legacy scenarios advanced.
7. [x] Add synthetic planner, generic ledger, duplicate-capability, UI, local acceptance, and reuse benchmark coverage.
8. [x] Preserve V15 regression and bank review-only policy.
9. [x] Complete the mandatory release gate without committing local workbooks or generated artifacts.

## V17 Objectives & Status
1. [x] Add structured stage, control, IPC-round-trip, and first-render metrics without fabricating peak-memory evidence.
2. [x] Add typed source/control/session/export errors and preserve independent control results on partial failure.
3. [x] Add cooperative cancellation and ensure cancelled work never reports PASS.
4. [x] Add a bounded in-process prepared-source cache with provenance-safe keys and explicit invalidation/reset.
5. [x] Decompose the default UI into an accounting-first four-step workspace with actionable review queue; preserve advanced scenarios.
6. [x] Formalize Office lock and exact duplicate intake behavior.
7. [x] Add resilience, cache, privacy, source-order, invalid-date, and frontend behavior tests.
8. [x] Preserve all V15/V16 correctness and local acceptance policies.
9. [x] Complete Rust/frontend/type/format/Clippy/build/smoke gates without tracking confidential or generated artifacts.

# Handoff Document

## Session Overview
- **Session Objective**: Fix reconciliation correctness, eliminate floating point arithmetic, support semantic debit/credit mapping, fix subtotal/summary and empty-amount row filtering, and achieve exact target reconciliation numbers on real-world accounting datasets.
- **Git Commit Target**: `fix: correct accounting reconciliation and multi-source matching`
- **Current Status**: 100% PASS on all 17 Rust engine tests and 10 Vitest frontend tests.

## Key Accomplishments in this Session
1. **Decimal Precision**:
   - Integrated `rust_decimal` & `rust_decimal_macros`. All amount fields in `CanonicalRecord`, `MatchGroup`, `FieldDiscrepancy`, `ReconciliationSummary`, and `ReconciliationSession` use `Decimal`.
2. **Column Sniffing & Mapping**:
   - Separated `debit_amount_column` ("Phát sinh Nợ") and `credit_amount_column` ("Phát sinh Có") from `total_amount_column`.
   - Updated `detect_header_and_mapping` in `header_detector.rs` and `MappingModal.tsx` in UI.
3. **Accounting Rules & Ingestion Filters**:
   - `Invoice.pretax_amount` matches against `TK511.credit_amount`.
   - Full-row scanning filters all summary/subtotal rows ("SỐ DƯ ĐẦU KỲ", "PHÁT SINH TRONG KỲ", "SỐ DƯ CUỐI KỲ", "TỔNG CỘNG").
   - Rows with empty or zero amounts are skipped from reconciliation scope.
4. **Mandatory Acceptance Verification**:
   - Invoices valid: `46` (out of 64 raw).
   - TK511 valid: `45` (out of 49 raw).
   - Exact Matches: `45`.
   - Amount Mismatch: `0`.
   - Missing in TK511: `1` (Invoice `#233`, Date `06/07/2026`, Pretax `105.000.000` đ, Tax `10.500.000` đ, Total `115.500.000` đ).
   - Missing in Invoice: `0`.
   - Invoice Pretax Sum: `7.328.121.057` đ.
   - TK511 Credit Sum: `7.223.121.057` đ.
   - Financial Variance: `105.000.000` đ.
5. **No False Matches**:
   - Distinct document numbers with equal amounts are never false-matched.

## Commands for Verification
- **Rust Engine Tests**: `cargo test -- --nocapture` (in `crates/reconciliation-core`)
- **Frontend Tests**: `npm test` (in root)
- **Frontend Production Build**: `npm run build` (in root)

## 2026-08-25 Continuation Record
- The current agent reviewed this handoff and the mandatory project-state documents before acting.
- Verified `cargo test --workspace` successfully with the GNU toolchain; the 54-test reconciliation regression suite passed.
- Verified `npm run test` successfully; 18 frontend tests passed.
- Committed inherited v14 source, test, handoff, and GNU WebView2 patch changes: `b2e8658` (`fix(v14): harden multi-source intake and GNU packaging`).
- The generated portable distribution is intentionally ignored and was not committed.
- No release build or portable smoke test was run in this continuation; those require their corresponding release gate when a release is requested.

## Repository Publication Verification — 2026-08-25
- Root `Cargo.lock` is intentionally tracked for reproducible Rust workspace resolution; nested and generated lockfiles remain ignored.
- The full pre-push gate completed successfully: Rust workspace tests, frontend tests, TypeScript typecheck, Rust formatting, and Clippy with warnings denied.
- No product behavior was changed during this repository-publication task.
- The `main` branch was pushed successfully to the configured GitHub `origin`; verify the current commit before any follow-up work.

## V15 Real Dataset Gate — 2026-08-25
- The user confirmed that the suffixless workbooks in `.local-testdata/` are the acceptance set. They were processed locally only and are still ignored.
- V15 source, parser, model, matching-safety, UI mapping, and test changes are on `feature/v15-real-accounting-datasets`; see `docs/REAL_DATASET_SUPPORT_MATRIX.md` for sanitized support evidence.
- The feature branch passed workspace Rust tests, frontend tests, typecheck, formatting, clippy, release build, and executable smoke test. It has not been merged to `main`.

## V15 Blocker Closure — 2026-08-25
- The matcher now evaluates all explicit rules for the same primary/secondary source-kind pair, preserving each semantic comparison.
- Direction compatibility is checked before a candidate can enter exact, tolerance, fallback, or aggregate resolution.
- Partner-master and sales-analysis one-source scenarios return typed reference-control results via the Tauri command and UI rather than transaction matches.
- Mapped invoice lifecycles are typed and fail closed unless standard. The invoice/sales-register/TK511 control raises a high-priority review when the first two agree but TK511 is absent.

## V15 Control-Plan Follow-up — 2026-08-25
- `LedgerEntry` is an additive generic ledger view; legacy `Ledger511`, `Ledger112`, `Ledger131`, `Ledger133`, and `Ledger3331` kinds remain compatible adapters.
- Matching compiles semantic controls against one physical `SourceIndex` per secondary source. Group target IDs are stable and de-duplicated for navigation.
- TK112/bank uses an explicit deterministic policy: compatible money direction, rule amount, date window, then unique reference/counterparty evidence. Missing evidence is `NEEDS_REVIEW`; no fuzzy score and no document/MST precondition.
- Reference-master and sales-analysis sources can coexist with transaction sources in IPC; they remain typed controls and are not passed to the transactional matcher.
- Aggregate subset search has a 12-candidate cap and exits after the second solution. Summary/export use gross discrepancy magnitude so semantic variances cannot cancel to a false zero.
- Verification before handoff: workspace Rust tests passed (54 baseline regression + 9 V15 blocker + 4 V15 support); frontend tests 18/18, TypeScript typecheck, formatting, and strict clippy passed. `npx tauri build` produced fresh ignored NSIS/MSI artifacts locally.

## V15 Semantic Closure — 2026-08-25
- `SemanticFieldComparison` drives UI wording per evaluated source; Sales Register revenue/VAT/receivable controls are separate from TK511 controls.
- Bank properties used for evidence are now typed on `CanonicalRecord`; audit evidence/review reason is retained in group discrepancies. Candidate policy remains deterministic.
- Partner identity is a read-only cross-source control with MST first, partner code fallback, and no name-based auto-merge.
- Aggregate matching never accepts a truncated candidate set. Under budget it searches bounded aggregate candidates; over budget it returns `COMPLEXITY_LIMIT`; when disabled it scans only 1:1 candidates.

## V15.4 Final Acceptance (2026-08-25)
- Bank auto-pass is evidence-gated: unique reference/transaction identity or unique strong counterparty evidence may be accepted; direction/amount/date alone is a non-consuming `SUGGESTED_DIRECTION_AMOUNT_DATE` review result. Opposite-direction amount/date candidates return a directional conflict.
- The header detector recognizes both historical `corresponsive` aliases and standard `correspondent account/name` aliases. A generic Vietnamese balance header is also mapped when present.
- Cross-source partner identity is restricted to explicitly compatible invoice, sales-register, and ledger kinds. Reference controls no longer determine the transactional primary after typed datasets are split.
- Aggregate matching first checks the full O(n) direct candidate set; subset search starts only when no direct match exists and fails closed above the budget.
- `local_real_acceptance_test` is ignored by default and reads only the local ignored dataset directory. It reports sanitized aggregates, confirms the tri-source review condition, and does not persist workbook values. A running balance equation is reported as unavailable rather than inferred when the normalized data lacks balance values.
- Final workspace/build/smoke verification completed successfully, including a five-second release executable smoke launch. The V15.4 commit was pushed to the feature branch; do not merge PR #1.

## V15 Final Micro-Fix — 2026-08-25
- Aggregate evaluation scans every candidate once for direct matches. More than one direct match is ambiguous; exactly one is accepted immediately; only zero direct matches can reach bounded subset search. Candidate sets above the budget return `COMPLEXITY_LIMIT` without a shift or subset enumeration.
- Bank matching maintains accepted and review-linked secondary ID sets separately. Residual output is generated only for IDs in neither set, so Suggested/Ambiguous evidence cannot also appear as bank-only.
- The local acceptance harness now asserts all approved oracles and validates classification conservation across all parsed bank records. Bank classification counts are observed, not hard-coded expectations.
- Workspace tests, explicit local acceptance, frontend tests, typecheck, format, strict Clippy, release build, and executable smoke all passed. The micro-fix is scoped to the existing feature branch only; do not merge PR #1.

## V15 Merge and Freeze — 2026-08-25
- ChatGPT approved the reviewed feature head. PR #1 was squash-merged into `main` with title `feat(v15): generalize real accounting dataset controls`; merge commit: `c5b03da4ed44eb9f5c88886256698beeafc7b4f9`.
- Before merge, the PR head, remote feature head, and local head were identical; the worktree was clean and no workbook, build directory, installer, or binary was tracked.
- Post-merge on `main`, the full Rust/TS/type/format/Clippy gate passed. A fresh Tauri build produced ignored MSI/NSIS artifacts; the release executable passed a five-second hidden smoke run.
- The NSIS RC is local-only under `target/release/bundle/nsis/` with SHA256 `936B775D65BD7D3BF5FD08CF60DCCC6CF2E9328D8331D2D2E7E4D413BE981E76`.
- V15 is **MERGED / FROZEN**. Preserve the official limitations and wait for ChatGPT product direction before starting any new phase.

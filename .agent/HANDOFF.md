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

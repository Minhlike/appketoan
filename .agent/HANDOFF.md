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

## V16 Audit Workspace / Control Planner — 2026-08-25
- Branch: `codex/v16-control-planner`, based exactly on the approved V15 main baseline. It remains unmerged.
- `crates/reconciliation-core/src/audit_workspace.rs` owns the additive session, capability catalog, declarative definitions, planner, period filtering, prepared indexes, executor, independent findings, and data-reuse evidence.
- Required transactional sources are bounded by the explicit session period and then by their common date intersection per control. Missing dates, no overlap, or duplicate required capabilities prevent automatic execution.
- Legacy ledger kinds are adapters only. Generic journal capabilities are derived from normalized account content and projected to the requested account before matching; a corresponding account in a legacy ledger does not imply a second ledger capability.
- `cmd_run_audit_workspace` reads and normalizes every selected source once in one ingestion loop, then runs every READY control against retained datasets. Raw workbook content is never persisted.
- The React default path is “Bộ hồ sơ kế toán”; `SourceRole`, manual kind selection, scenario selection, and legacy execution are under advanced settings.
- Local acceptance produced four READY implemented controls and two missing future controls. The immutable V15 oracle and bank evidence policy remain unchanged.
- The 100k comparison benchmark completed with exact result counts; V16 intentionally includes session preparation plus three independent controls and is not presented as a speedup over the V15 tri-source-only run.
- Full workspace tests, frontend tests, typecheck, format check, strict Clippy, and Tauri release build passed. Generated installers and confidential workbooks remain ignored.

## V17 Resilience / UI Foundation — 2026-08-26
- Branch: `codex/v17-resilience-ui-foundation`, stacked on the unmerged V16 branch. Neither branch may be merged by the agent.
- `audit_resilience.rs` defines typed failures, run status, structured metrics, cancellation token, prepared cache key/data, and fixed memory/source bounds.
- Audit controls execute independently. Recoverable source/control failure produces partial results; cancellation is cooperative at source/control boundaries and cannot produce PASS.
- Tauri runtime state owns cancellation tokens and an in-memory L1 cache. Source identity plus content/sheet/mapping/kind/schema prevents cross-source provenance reuse; period changes rebuild the view.
- The default UI is split into workspace, source intake, period, planner/result, review queue, execution hook, and advanced scenario components. Suggested/ambiguous bank evidence remains review-only.
- Native drag/drop sends local file paths; file-picker/browser fallback still uses byte arrays. The backend enforces duplicate, lock-file, and source-size policies regardless of UI path.
- Full local acceptance and mandatory release gates passed. Exact benchmark numbers and limitations are recorded in `docs/04-architecture/04-v17-resilience-ui-foundation.md`.
- Generated release artifacts and local accounting workbooks remain ignored. Stop after pushing the stacked PR for ChatGPT source/diff review.

## V18 Document Integrity / Field-Level Reconciliation — 2026-08-26
- Branch: `codex/v18-document-integrity`, based exactly on the reviewed V17 head. Stacked PR #4 (`https://github.com/Minhlike/appketoan/pull/4`) targets `codex/v17-resilience-ui-foundation`; neither PR may be merged by the agent.
- `document_integrity.rs` is the authoritative pure-Rust evaluator for Invoice/Sales Register/TK511. It validates BK first, marks all duplicate rows, uses exact field checks, allows only unique strong diagnostic links, and separates totals from document PASS.
- `PreparedSourceIndex.review_records` retains missing/unparseable-date rows outside every auto-match index. ADR 0014 supersedes the initial intersection rule for document completeness: the explicit user period is authoritative, while bank matching retains its distinct fail-closed intersection.
- `ControlResult.document_integrity_result` is wired through Tauri serialization into the Audit Workspace. The existing Review Queue receives each typed error; selecting a row opens side-by-side provenance and field checks.
- The unchanged record-count oracle and #233 review condition passed against ignored local workbooks. One BK row has no parseable required date, so period totals are correctly `NOT_VERIFIED`.
- Initial V18 gates passed, but the final RC task superseded the initial performance/period conclusions. See the final block below.

## V18 Final Correctness Fix and RC — 2026-08-26
- Continued on PR #4 from reviewed head `a759d8a1a10f0d093b694bcf571546ba780eefd3`; do not merge.
- Revenue document completeness now covers the full user-selected accounting period. Four synthetic boundary regressions prove late/early source evidence cannot produce false PASS; bank period behavior has an explicit fail-closed regression.
- The existing typed invoice lifecycle evaluator is reused. Adjusted, affected-by-adjustment, replaced, cancelled, and unknown mapped values emit `INVOICE_LIFECYCLE_NEEDS_REVIEW` and never become fully matched.
- Audit Workspace revenue no longer materializes a second generic reconciliation result. Prepared datasets are borrowed when possible; advanced legacy scenario execution is unchanged.
- The 100k debug benchmark improved to 7.80 seconds cold total and 7.44 seconds warm execution. Sampled peak working set fell to about 1.30 GiB; see `REPORT_V18_FINAL_RC.md` for measurement caveats.
- Both confidential local harnesses and all mandatory source/UI/build gates passed. The authoritative period exposed beginning-of-period BK evidence that the prior intersection had hidden; it remains review evidence, not a hard-coded oracle.
- Fresh portable, NSIS, and MSI artifacts were rebuilt, hashed, and kept ignored. The fresh portable executable passed a five-second smoke run. Stop for final ChatGPT review.

## V18 Portable EXE Blank-Window Correction — 2026-08-26
- A user run proved that the previous process-only smoke could false-pass a blank desktop window. Treat every earlier “alive for five seconds” claim as superseded for UI readiness.
- The desktop window is hidden until the embedded page-load completion event. `index.html` contains a visible startup/failure surface, and React reports uncaught bootstrap errors there instead of leaving an empty root.
- `scripts/smoke_release_ui.ps1` verifies the actual Windows accessibility tree and fails if the process lives without rendering the expected AppKetoan heading. The existing audit packaging script now delegates to this rendered-UI smoke.
- The final portable EXE passed the rendered-UI smoke with 72 elements on both normal and fresh WebView2 profiles. Direct window capture showed the complete dashboard. Current portable SHA256 is `CB38798F06A2F367B5D6B950B18B33886A25E4DACCBF8365A74747F5AA8D7F2B`.
- This correction is EXE-only. NSIS/MSI were not rebuilt, no accounting behavior changed, and PR #4 must remain unmerged pending review.

## V18 Audit Result IPC Crash Correction — 2026-08-26
- The startup fallback successfully exposed a real post-run error instead of a white window: `Object.keys` received an omitted `summaryMetrics` collection from a not-run/missing-source control.
- Root cause was a cross-language contract mismatch: Rust used `skip_serializing_if` for fields that TypeScript marks required. Rust now always emits `{}` and `[]`; React also treats those fields as optional at the wire boundary for compatibility.
- Regression coverage includes exact Rust JSON shape and rendering a legacy partial payload with both collections omitted. No control is promoted to PASS.
- Full source gate and both ignored local acceptance harnesses passed. The portable EXE was rebuilt without installers and passed rendered-UI smoke with 72 UI elements.
- Current local-only portable: `target/release/appketoan.exe`; SHA256 `BC9202025EB73EDB91097F2CBC99716A681DD61862C0F4A0FD824FF189B7E261`. Await the user's reproduction test; do not merge PR #4.

## V18 Two-Source Document Control Correction — 2026-08-26
- A user run with only Invoice and Sales Register exposed that the tri-source planner returned `NOT_RUN` solely because TK511 was absent, suppressing valid two-source evidence.
- ADR 0015 makes Invoice plus Sales Register the minimum executable set while keeping TK511 required for complete assurance. Missing TK511 remains in plan/result capability evidence; every ledger field is `NOT_CHECKED`; `documentsPass` and the control remain fail-closed.
- The result distinguishes exact Invoice-to-Register pairs from fully matched three-source documents. A missing ledger source is not misreported as every individual document being absent from TK511.
- The Review Queue now omits unrelated controls with no loaded source while the full Control Plan still shows them.
- Synthetic regression, ignored local two-source acceptance, full source gates, portable build, and rendered smoke passed. Current portable SHA256: `045FA248A562679BBEF779E3CE5163A2054BC6534F0F3732BA9F14206A409668`. PR #4 remains unmerged.

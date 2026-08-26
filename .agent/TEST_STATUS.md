# Test Status

## Test Execution Summary

### 1. Rust Domain Crate Tests (`crates/reconciliation-core`)
- **Command**: `cargo test -- --nocapture`
- **Result**: `17 PASSED, 0 FAILED, 0 IGNORED`
- **Details**:
  - `models::canonical_record::tests::test_doc_no_normalization`: PASS
  - `models::canonical_record::tests::test_tax_id_normalization`: PASS
  - `models::canonical_record::tests::test_monetary_invariants`: PASS
  - `models::data_source::tests::test_data_source_serialization`: PASS
  - `models::reconciliation_result::tests::test_reconciliation_result_summary`: PASS
  - `models::reconciliation_rule::tests::test_matching_rule_serialization`: PASS
  - `normalizer::row_normalizer::tests::test_parse_excel_dates`: PASS
  - `normalizer::row_normalizer::tests::test_parse_amount_variations`: PASS
  - `normalizer::row_normalizer::tests::test_garbage_row_detection`: PASS
  - `reader::header_detector::tests::test_header_detection_einvoice`: PASS
  - `reader::header_detector::tests::test_header_detection_tk511_credit_debit`: PASS
  - `tests/end_to_end_reconciliation_test.rs`: PASS (with Excel export validation)
  - `tests/golden_dataset_test.rs`: PASS
  - `tests/performance_benchmark_test.rs`: PASS (1,000 to 100,000 pairs benchmarked; 100k pairs matched in 1.42s)
  - `tests/reconciliation_correctness_regression_test.rs`:
    - `test_01_and_02_tk511_credit_debit_semantic_mapping`: PASS
    - `test_03_summary_rows_filtered`: PASS
    - `test_04_empty_amount_invoice_rows_filtered`: PASS
    - `test_05_06_07_08_mandatory_acceptance_regression`: PASS (46 Invoices vs 45 TK511, 45 Matches, 1 Missing #233, 105M Variance)
    - `test_09_no_false_matching_for_same_amount_different_doc`: PASS
    - `test_10_multi_source_identity_preserved`: PASS

### 2. Frontend Vitest Tests (`src/`)
- **Command**: `npm test`
- **Result**: `10 PASSED, 0 FAILED (2 test files)`
- **Details**:
  - `src/types/dataContract.test.ts`: 6 tests PASS
  - `src/App.test.tsx`: 4 tests PASS

### 3. Frontend Production Build
- **Command**: `npm run build`
- **Result**: `SUCCESS` (`tsc && vite build` completed cleanly, 0 errors)

### 4. Continuation Verification (2026-08-25)
- **Rust Command**: `cargo test --workspace`
- **Rust Result**: `SUCCESS`; the reconciliation correctness regression target completed all 54 tests with zero failures.
- **Frontend Command**: `npm run test`
- **Frontend Result**: `SUCCESS`; 18 tests across 3 files passed with zero failures.
- **Scope Note**: Typecheck, formatting, clippy, release build, and portable smoke test were not requested or rerun in this continuation.

### 5. Pre-Push Release Gate (2026-08-25)
- `cargo test --workspace`: SUCCESS; reconciliation regression suite 54/54 PASS.
- `npm run test`: SUCCESS; 18/18 PASS.
- `npm run typecheck`: SUCCESS.
- `cargo fmt --all -- --check`: SUCCESS.
- `cargo clippy --workspace --all-targets -- -D warnings`: SUCCESS.
- The verified `main` state was then published to the configured GitHub `origin`.

### 6. V15 Real-Dataset Gate (2026-08-25)
- User confirmation established the suffixless `.local-testdata/` files as the local acceptance set; no workbook was committed.
- `cargo test --workspace`: SUCCESS; 54/54 reconciliation regression tests plus the V15 support suite passed.
- `npm run test`: SUCCESS; 18/18 passed. `npm run typecheck`, `cargo fmt --all -- --check`, and strict Clippy: SUCCESS.
- `npx tauri build`: SUCCESS. The release executable launched and remained alive for the local five-second smoke check.

### 7. V15 Blocker Closure (2026-08-25)
- `cargo test --workspace`: SUCCESS; all 54 baseline regression tests, 7 new blocker regressions, and existing V15 tests passed.
- `npm run test`: SUCCESS; 18/18 passed. `npm run typecheck`, `cargo fmt --all -- --check`, and strict workspace Clippy: SUCCESS.
- `npx tauri build`: SUCCESS; generated artifacts remain ignored.

### 8. V15 Control-Plan Follow-up (2026-08-25)
- `cargo test --workspace`: SUCCESS; 54/54 baseline reconciliation regressions, 9 V15 blocker regressions, and 4 V15 dataset-support tests passed.
- `npm run test`: SUCCESS; 18/18 passed. `npm run typecheck`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings`: SUCCESS.
- `npx tauri build`: SUCCESS; fresh NSIS/MSI build outputs remain ignored. The GNU linker emitted an existing `webview2-com-sys` `.drectve` warning during tests; strict clippy stayed clean.

### 9. V15 Semantic Closure (2026-08-25)
- `cargo test -p reconciliation-core --test reconciliation_correctness_regression_test`: SUCCESS; 54/54 baseline regressions passed after aggregate-policy changes.
- `cargo test -p reconciliation-core --test v15_blocker_regression_test`: SUCCESS; 11/11 blocker regressions passed.
- `npm run test`: SUCCESS; 19/19 passed. `npm run typecheck`, formatting, and strict workspace Clippy: SUCCESS.

### 10. V15.4 Final Acceptance (2026-08-25)
- Local confidential-workbook harness: SUCCESS. It read the required ignored workbook set directly using `APPKETOAN_LOCAL_TESTDATA` and emitted only sanitized aggregate runtime metrics.
- `cargo test -p reconciliation-core --test performance_benchmark_test -- --nocapture`: SUCCESS; one-, three-, and four-control 100k workloads completed with exact match counts.
- `cargo test --workspace`: SUCCESS; all baseline regression, V15 blocker, support, end-to-end, golden, IPC, and performance targets passed. The confidential local harness remains ignored by default and was run explicitly.
- `npm run test`: SUCCESS (19 tests). `npm run typecheck`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings`: SUCCESS.
- `npx tauri build`: SUCCESS; fresh ignored Windows installer artifacts were generated. The fresh release executable passed a five-second hidden smoke launch.

### 11. V15 Final Micro-Fix (2026-08-25)
- `cargo test --workspace`: SUCCESS; the 54-test correctness baseline, 17 V15 blocker tests, six dataset-support tests, performance targets, and all other workspace targets passed. The existing GNU WebView2 linker warning remained non-fatal.
- Explicit ignored real-local acceptance harness: SUCCESS. All known oracles are assertions; bank matching counts remain observed classifications rather than hard-coded oracle counts.
- Real bank classification: 225 parsed; 0 strong accepted, 93 suggested/review-linked, 42 ambiguous/review-linked, 90 true bank-only, 0 true ledger-only. Classification conservation covered all parsed records.
- Synthetic benchmark: 100k one-control 4.15s, three-control 7.41s, four-control tri-source 13.25s; exactly 100,000 matches in each run.
- `npm run test`: SUCCESS (19/19). TypeScript typecheck, Rust format check, and strict workspace Clippy: SUCCESS.
- `npx tauri build`: SUCCESS; fresh ignored MSI/NSIS and release executable were generated. Five-second hidden executable smoke: SUCCESS.

### 12. V15 Post-Merge Gate and Freeze (2026-08-25)
- Merge commit on `main`: `c5b03da4ed44eb9f5c88886256698beeafc7b4f9`.
- `cargo test --workspace`: SUCCESS; all workspace unit, integration, correctness, V15 blocker/support, benchmark, and doc-test targets passed. The known GNU WebView2 `.drectve` linker warning remained non-fatal.
- `npm run test`: SUCCESS (19/19). `npm run typecheck`, `cargo fmt --all -- --check`, and strict workspace Clippy: SUCCESS.
- `npx tauri build`: SUCCESS; fresh ignored MSI/NSIS and release executable generated from merged `main`.
- Release executable smoke: SUCCESS after five seconds. NSIS SHA256: `936B775D65BD7D3BF5FD08CF60DCCC6CF2E9328D8331D2D2E7E4D413BE981E76`.
- Result: V15 RC gate passed; V15 is frozen pending product direction.

### 13. V16 Audit Workspace / Control Planner Gate (2026-08-25)
- Synthetic planner suite: SUCCESS; capability planning, period fail-closed behavior, generic ledger adapters, duplicate capability review, data reuse, and independent execution passed.
- Local V15 acceptance: SUCCESS against ignored local workbooks; the immutable baseline and bank classification conservation remain unchanged.
- Local V16 acceptance: SUCCESS; four implemented controls planned READY, two future account controls planned MISSING_SOURCE, per-control periods were bounded safely, and every logical source reported one read/normalize/index.
- Performance comparison: SUCCESS with 100,000 records per transactional source. V15 tri-source completed in 9.07s; V16 preparation plus three READY controls completed in 20.09s; both produced exactly 100,000 matches.
- `cargo test --workspace`: SUCCESS; all unit, integration, 54-test correctness, V15, V16, benchmark, and doc-test targets passed. Local confidential harnesses are ignored by the default command and were run explicitly.
- `npm run test`: SUCCESS (20/20). `npm run typecheck`, `cargo fmt --all -- --check`, and strict workspace Clippy: SUCCESS.
- `npx tauri build`: SUCCESS; generated MSI/NSIS artifacts remain ignored. The known GNU WebView2 `.drectve` warning remained non-fatal.

### 14. V17 Resilience / UI Foundation Gate (2026-08-26)
- `cargo test --workspace`: SUCCESS; all unit, integration, 54-test correctness, V15/V16/V17 resilience/privacy, benchmark, and doc-test targets passed. Confidential harnesses remain ignored by default.
- Explicit V15 and V16 local acceptance harnesses: SUCCESS against the ignored local workbook set; immutable oracle, four READY/two MISSING planning behavior, bank review-only policy, and classification conservation remained unchanged.
- `npm run test`: SUCCESS (26/26 across seven files). `npm run typecheck`: SUCCESS.
- `cargo fmt --all -- --check`: SUCCESS. `cargo clippy --workspace --all-targets -- -D warnings`: SUCCESS.
- Equivalent debug benchmark evidence is recorded in the V17 architecture note; it is one iteration and is not labeled median/p95. Peak memory remains unmeasured.
- `npx tauri build`: SUCCESS; fresh ignored MSI/NSIS and release executable generated. Five-second release executable smoke: SUCCESS.
- Automated local browser visual inspection: NOT VERIFIED because the Codex browser runtime could not initialize its local kernel assets. Component behavior tests, production frontend build, Tauri build, and executable smoke passed; no substitute automation was misreported as visual evidence.

### 15. V18 Document Integrity / Field-Level Gate (2026-08-26)
- `cargo test --workspace`: SUCCESS; 129 executed tests passed, including 13 V18 adversarial/integration tests. Two confidential local harnesses remained ignored by the default command. The known GNU WebView2 linker warning remained non-fatal.
- Explicit ignored local acceptance: SUCCESS. The immutable record-count and #233 oracles passed. V18 asserted 45 fully matched documents, no valid-period missing/extra BK document, one missing TK511 document, and one review-only BK invalid-date row. Period totals are fail-closed `NOT_VERIFIED`.
- `npm run test`: SUCCESS (27/27 across eight files). `npm run typecheck`: SUCCESS.
- `cargo fmt --all -- --check`: SUCCESS. `cargo clippy --workspace --all-targets -- -D warnings`: SUCCESS.
- `npx tauri build`: SUCCESS; production frontend, release executable, MSI, and NSIS completed. Fresh release executable five-second smoke: SUCCESS.
- The final debug 100k reuse benchmark completed in 78.20 seconds while materializing both V18 and legacy compatibility results. Automated browser visual inspection was not rerun and remains NOT VERIFIED.

### 16. V18 Final Correctness and Windows RC Gate (2026-08-26)
- `cargo test --workspace`: SUCCESS; 134 executed Rust tests passed and two confidential local harnesses were ignored by default. The existing GNU WebView2 linker warning remained non-fatal.
- Explicit `local_real_acceptance_test`: SUCCESS; immutable baseline and bank classification conservation remained unchanged.
- Explicit `local_v16_acceptance_test`: SUCCESS; #233 remained high-priority review, partner/sales controls passed, and authoritative period handling exposed beginning-of-period BK evidence previously hidden by intersection.
- Boundary/lifecycle suite: SUCCESS; four period adversarial cases, one multi-lifecycle regression, and an independent bank-period fail-closed regression passed.
- 100k debug benchmark: SUCCESS; cold total 7.80s, warm execution 7.44s, exact typed document count preserved. Same-machine reproduced pre-fix cold total was 13.43s; reviewed historical gate was 78.20s.
- Sampled benchmark working set: 1,365,471,232 bytes after versus 2,355,978,240 bytes reproduced before. This is a single debug test-process sample, not production median/p95.
- `npm run test`: SUCCESS (29/29 across eight files). `npm run typecheck`: SUCCESS.
- `cargo fmt --all -- --check`: SUCCESS. `cargo clippy --workspace --all-targets -- -D warnings`: SUCCESS.
- `npx tauri build`: SUCCESS; fresh portable, MSI, and NSIS artifacts all had post-build timestamps. Fresh portable executable five-second smoke: SUCCESS.
- RC SHA256 values are recorded in `REPORT_V18_FINAL_RC.md`. Generated artifacts and confidential workbooks remain ignored. Automated browser visual inspection remains NOT VERIFIED.

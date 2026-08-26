# V18 Document Integrity — Verification Report

## Scope

- Branch: `codex/v18-document-integrity`
- Base: exact V17 reviewed head on `codex/v17-resilience-ui-foundation`
- Merge status: not merged
- No new account control, cloud path, workbook, installer, or generated build directory is part of the source change.

## Implemented

- Added a pure-Rust, deterministic document-integrity evaluator for Invoice ↔ Sales Register ↔ TK511.
- BK validation retains invalid rows as review-only provenance and marks every duplicate normalized document number.
- Thuế ↔ BK checks date, document number, pretax, VAT, and total with exact equality and multiple simultaneous error codes.
- Diagnostic links require unique date/partner plus all three monetary fields; they never become accepted matches.
- Thuế ↔ TK511 checks date, document number, and pretax against credit. VAT and receivable are explicit `NOT_CHECKED` fields.
- Total equality and document PASS are independent.
- Audit Workspace renders the ten-category summary, document table, side-by-side evidence, field checks, and all error codes. Findings reuse the V17 Review Queue.

## Synthetic acceptance

All V18 adversarial cases passed:

- duplicate BK document number marks every row and never selects one;
- wrong date with equal amounts remains review-only;
- wrong document number with equal amounts is only a unique diagnostic link;
- wrong date plus wrong number requires unique partner/monetary evidence;
- independent and combined pretax/VAT/total mismatches;
- missing, extra, and ambiguous BK conservation;
- equal totals with wrong documents does not PASS;
- #233 is `NEEDS_REVIEW` / `HIGH` because TK511 evidence is absent;
- TK511 VAT/receivable are `NOT_CHECKED`;
- cancellation and partial failure cannot convert document errors to PASS;
- V17 amount/date-only bank evidence remains Suggested/review-only.

## Local acceptance

The ignored local harness passed against the existing local workbook set. It asserted the unchanged record-count oracle, 45 fully matched documents, no missing/extra BK document in the valid effective period, and exactly one missing TK511 document (#233).

One BK row has an unparseable/missing required date. V18 now retains it with provenance, so the revenue control plan and result are `NEEDS_REVIEW`. Period totals are `NOT_VERIFIED`; they are not mislabeled as equal because the row cannot safely be scoped to a period. No workbook content was persisted or committed.

## Gates

- `cargo test --workspace`: PASS; 129 executed tests passed, two confidential local harnesses ignored by default.
- Explicit ignored local V15/V16/V18 acceptance: PASS.
- `npm run test`: PASS; 27/27.
- `npm run typecheck`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `npx tauri build`: PASS; frontend production build, Rust release executable, MSI, and NSIS completed.
- Fresh release executable smoke: PASS after five seconds.

## Known limitations

- The legacy `ReconciliationResult` is still produced alongside the V18 typed result for V15/V16 compatibility. The final debug 100k reuse benchmark took 78.20 seconds on this gate run; it is a performance/contract migration follow-up, not a correctness relaxation.
- The local BK row with an unknown date prevents period totals from being verified until the source date is corrected.
- TK511 has no VAT or receivable evidence; those fields remain intentionally `NOT_CHECKED`.
- Cancellation remains cooperative at source/control boundaries, not inside every evaluator loop.
- Automated browser visual inspection remains unverified; component tests, TypeScript, production frontend build, Tauri build, and executable smoke passed.

# V15 REAL DATASET EXPANSION REPORT

## Git

- Base: `3ac2dba15fe59b9e7244d77506104e1816cd56f1`
- Branch: `feature/v15-real-accounting-datasets`
- HEAD: latest blocker-closure commit on the feature branch
- PR: existing PR #1; no merge performed

## Local Acceptance Boundary

- The user confirmed the seven suffixless workbooks in `.local-testdata/` are the required local acceptance files.
- They were processed only by the local Rust reader and remain ignored by Git. No workbook content, identity, transaction, or credential is committed.
- Sanitized schema and control evidence is in `docs/REAL_DATASET_SUPPORT_MATRIX.md`.

## Acceptance

- Invoice/TK511 baseline: PASS through the real-local regression suite.
- Invoice to BK support: PASS for source detection, mapped control fields, preserved provenance, and the local control-document presence check; cross-source period remains a mandatory matching condition.
- Partner master: PASS for non-monetary typed normalization, local required-partner presence, and fail-closed duplicate tax-identity resolution.
- TK112/bank: PASS for directional compatibility and review conservation. The running-balance equation is `NOT_VERIFIED` because the normalized source does not expose enough balance values; no equation is inferred.
- Legacy XLS: PASS through the real Calamine reader with an adaptive physical header position.
- Sales report: PASS; group and detail totals must agree and exactly one verified layer is used.
- FALSE_MATCH: PASS; synthetic adversarial regression and the new TK112/bank direction test reject equal-amount contradictory-direction candidates.
- Tri-source invoice/sales-register/TK511: PASS; the control document is represented as a high-priority `NEEDS_REVIEW` when invoice and sales register agree but TK511 is absent.

## Architecture Changes

- Added V15 `DataSourceKind` and reference-master role support, physical worksheet positions, and additive mappings.
- Added typed partner-master and sales-analysis records, duplicate-master identity control, and sales-analysis double-count prevention.
- Added ledger-112/bank monetary direction semantics and matcher compatibility checks.
- Extended source detection, UI source roles/kinds, mapping editor, and V15 regression coverage.
- Enforced all-rule evaluation per source pair, typed invoice lifecycle gating, and a typed IPC/UI control path for one-source partner/sales-analysis scenarios.
- Added a generic `LedgerEntry` compatibility view, one physical index per source with compiled semantic controls, deterministic bank candidate evidence, bounded aggregate search, and gross (non-netted) discrepancy reporting.
- Partner-master and sales-analysis controls can now accompany transactional sources through IPC; they remain typed controls instead of empty transactional datasets.
- UI semantic labels now include the actual evaluated secondary source, so Sales Register controls are never rendered as TK511/TK3331/TK131 controls. Typed bank fields retain transaction/reference, dates, counterparties and balance while source provenance remains intact.
- Aggregate matching is fail-closed above the configured candidate budget; it never accepts a partial candidate scan.

## Tests

- Rust workspace: PASS (including 54/54 reconciliation regression tests, V15 support tests, and 7 blocker regressions).
- TS: PASS (18/18).
- typecheck, format, strict clippy: PASS.
- release build: PASS.
- executable smoke: PASS (release executable remained alive for five seconds).

## Known Limitations

- Reference controls are now available through IPC and a compact UI result card; dedicated drill-down/report exports remain a follow-up UX task.
- The support matrix deliberately contains only sanitized metadata and approved aggregate controls.

## Local RC

- Release artifacts were built locally and deliberately remain ignored.

## V15.4 Final Acceptance (2026-08-25)

This section supersedes earlier release-gate assertions where it differs. The
local harness reads the seven ignored workbooks directly and prints only
sanitized runtime aggregates; workbook contents and financial amounts are not
persisted in agent memory.

- Baseline control: PASS; the established Invoice/TK511 acceptance shape was reproduced.
- TK112/bank: the bank reader parsed records and the ledger reader produced transactional records. Amount/direction/date-only candidates were reported as suggestions and were not consumed; the run produced no strong-evidence auto-accepts.
- TK112 balance equation: NOT_VERIFIED. The local ledger workbook did not expose two or more normalized running-balance values, so no equation is inferred.
- Tri-source control document: `NEEDS_REVIEW` (high priority) when the ledger evidence is absent.
- Partner identity and sales-analysis layer controls both completed locally; the analytical report selected one verified layer and did not double-count group plus detail rows.
- Benchmark coverage: PASS for 100k one-semantic, three-semantic, and four-control tri-source synthetic runs. Elapsed values are terminal-only runtime evidence, not persisted here.
- Native ingestion: path-first is used whenever the Tauri source path exists; byte ingestion remains only for explicitly supplied IPC bytes and is not represented as an optimization.

## V15 Final Micro-Fix (2026-08-25)

- Real-local assertions: PASS. The known invoice/TK511 record-count, exact-match, missing-document, revenue-gap, bank parsed-count, required-partner, sales-layer, and tri-source high-review oracles are executable assertions rather than console-only observations. Sensitive document and monetary values are intentionally not duplicated in agent memory.
- Bank classification over 225 parsed statement records: 0 strong accepted, 93 suggested/review-linked, 42 ambiguous/review-linked, 90 true bank-only, and 0 true ledger-only. The accepted/review/unlinked union covers every statement record and review-linked records are excluded from residual missing-source output.
- Aggregate policy regressions: PASS for 32 and 64 candidates with one direct match, 32 candidates without a direct match, and multiple direct matches. Subset enumeration is unreachable above the 12-candidate budget.
- Synthetic performance: 100k one-control completed in 4.15s, 100k three-control in 7.41s, and 100k four-control tri-source in 13.25s; each produced exactly 100,000 matches.
- Full Rust/TS/type/format/Clippy gate, Tauri release build, and five-second executable smoke: PASS.

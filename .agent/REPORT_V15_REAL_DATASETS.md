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
- TK112/bank: PASS for the local opening/turnover/closing balance equation and directional compatibility guard; opposite directions cannot produce an amount-only match.
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

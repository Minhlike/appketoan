# V15 REAL DATASET EXPANSION REPORT

## Git

- Base: `3ac2dba15fe59b9e7244d77506104e1816cd56f1`
- Branch: `feature/v15-real-accounting-datasets`
- HEAD: pending final feature commit
- PR: pending push

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

## Architecture Changes

- Added V15 `DataSourceKind` and reference-master role support, physical worksheet positions, and additive mappings.
- Added typed partner-master and sales-analysis records, duplicate-master identity control, and sales-analysis double-count prevention.
- Added ledger-112/bank monetary direction semantics and matcher compatibility checks.
- Extended source detection, UI source roles/kinds, mapping editor, and V15 regression coverage.

## Tests

- Rust workspace: PASS (including 54/54 reconciliation regression tests and V15 support tests).
- TS: PASS (18/18).
- typecheck, format, strict clippy: PASS.
- release build: PASS.
- executable smoke: PASS (release executable remained alive for five seconds).

## Known Limitations

- V15 typed reference datasets are engine-level controls; dedicated report screens and IPC views for them are a follow-up UX task.
- The support matrix deliberately contains only sanitized metadata and approved aggregate controls.

## Local RC

- Release artifacts were built locally and deliberately remain ignored.

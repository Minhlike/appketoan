# ADR 0013: Fail-Closed Document Integrity Matching

- Status: Accepted
- Date: 2026-08-26

## Context

The generic transactional matcher correctly preserves multi-source accounting semantics, but its configurable amount/date tolerances and amount-oriented candidate paths are not sufficient evidence for document-integrity control. A duplicate BK number, wrong date, or compensating record error must not become PASS merely because amounts or file totals agree.

## Decision

`REVENUE_INVOICE_REGISTER_LEDGER` receives an additive typed document-integrity evaluator in the pure Rust core. It consumes the V17 prepared datasets and builds deterministic indexes without re-reading a workbook.

BK validation runs before matching. Invalid and duplicate rows keep file/sheet/row provenance and remain outside accepted matching. Thuế ↔ BK checks five fields exactly. A document-number mismatch can only create a unique diagnostic link from non-monetary evidence plus all corresponding monetary fields; diagnostic links never become accepted. Thuế ↔ TK511 checks date, document number, and credit revenue, while VAT and receivable are explicit `NOT_CHECKED` fields.

Invalid-date rows are retained separately from period-filtered match indexes. A control may execute validation while its plan remains `NEEDS_REVIEW`; this never widens an auto-acceptance boundary. Total checks and document checks have independent statuses.

The V18 result is added to `ControlResult`; the legacy `ReconciliationResult` remains temporarily for V15/V16 IPC and benchmark compatibility. The control's authoritative status is derived only from the V18 document result.

## Consequences

- Duplicate, invalid, missing, extra, ambiguous, and multi-field errors cannot be hidden by equal totals.
- Global date or amount tolerance cannot relax the document-integrity control.
- Review Queue and document detail views receive typed, provenance-preserving evidence.
- V17 bank matching, cache, cancellation, partial failure, and planner behavior stay isolated.
- Until the legacy result contract is migrated, the revenue control pays the CPU/memory cost of producing both result representations.

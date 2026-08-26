# V18 Document Integrity and Field-Level Reconciliation

## Control flow

```text
Prepared V17 datasets
  -> retain invalid-date BK rows as review-only evidence
  -> validate every BK row and mark every duplicate Số ct
  -> exact document-number index
  -> unique diagnostic indexes (date + three amounts / partner + three amounts)
  -> strict Thuế ↔ BK field checks
  -> strict Thuế ↔ TK511 field checks
  -> independent total checks
  -> typed document cases + existing Review Queue
```

The V18 evaluator is additive to the Audit Workspace. It reuses the same normalized records and provenance-safe prepared-source cache. It does not parse or normalize a workbook again.

## Acceptance rules

- A BK row is not auto-acceptable unless its document number is present and unique, its date is a valid ISO date after normalization, and pretax/VAT/total are typed source monetary values.
- Every row sharing a duplicate normalized document number receives `DUPLICATE_INVOICE_NUMBER`. No duplicate row is selected or aggregated.
- Thuế ↔ BK requires exact equality for date, document number, pretax, VAT, and total. Session/global tolerance is not applied.
- Amount is never an identity key by itself. A number-mismatch diagnostic needs a unique candidate with exact date plus all three monetary fields, or exact partner identity plus all three monetary fields when the date also differs.
- Diagnostic links remain review-only. Multiple candidates return `AMBIGUOUS_MATCH`.
- Thuế ↔ TK511 checks date, document number, and invoice pretax against ledger credit. VAT and receivable are explicitly `NOT_CHECKED`.
- Total equality is evidence only. Any duplicate, invalid, missing, extra, ambiguous, or field-level mismatch prevents document PASS.

## Period safety

Missing/unparseable dates remain outside all auto-match indexes. They are retained in a separate review-only collection so V18 can report file, sheet, and row provenance. For document completeness, the explicit user-selected `AccountingPeriod` is authoritative; source earliest/latest values are evidence and warnings only. This prevents a late-start or early-end source from hiding missing/extra evidence at a period boundary. The bank control retains its separate intersection-based fail-closed policy.

## Complexity

Document-number, date-money, partner-money, and ledger indexes are built in linear time. Normal execution is O(n) expected time and O(n) additional memory. Audit Workspace revenue now runs only the authoritative typed evaluator and borrows prepared period-filtered slices when possible. The optional legacy result is not materialized there; the unchanged advanced scenario path remains the compatibility route for the generic matcher.

## UI

The existing Audit Workspace shows the requested ten-category summary and a document table. Selecting a document opens side-by-side Thuế/BK/TK511 provenance, every field check, explicit `NOT_CHECKED` states, and all error codes. The same typed errors are flattened into the existing Review Queue.

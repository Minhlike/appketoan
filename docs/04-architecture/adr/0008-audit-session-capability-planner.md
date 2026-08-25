# ADR 0008: Audit Session and Capability-Based Control Planner

## Status

Accepted for V16.

## Context

V15 executes a user-selected scenario. That remains useful for advanced mapping, but it makes an accountant choose engine concepts before the application can say which checks are possible. It also encourages repeated workbook ingestion when several controls use the same normalized source.

An audit package may contain sources from different sub-periods. The current acceptance package contains revenue evidence for one month and bank evidence for another month, while a sales register spans a wider range. Matching every row in the wider file against the narrower source would create false residual findings.

## Decision

- `AuditSession` owns one explicit inclusive accounting period, a capability-based `SourceCatalog`, normalized typed datasets, one prepared index per source, and generated `ControlPlan` entries.
- Source capabilities are derived from normalized content and source semantics. Legacy `ledger_511`, `ledger_112`, `ledger_131`, and `ledger_3331` kinds are compatibility adapters for `LedgerEntry(account=...)`; a corresponding account on the other side of an entry does not turn that workbook into that account's ledger.
- Controls are declarative `ControlDefinition` values. V16 implements revenue tri-source, bank reconciliation, partner identity, and sales analysis. VAT account 3331 and receivable account 131 are definitions only and remain `MISSING_SOURCE` when those ledgers are absent.
- Transactional records are filtered to the session period before planning. A ready control receives the intersection of date evidence across its required sources as its effective period. This prevents a wide sales register from being matched across months that are absent from the invoice or ledger source.
- Missing or unparseable required dates fail closed as `NEEDS_REVIEW`. An empty date intersection is never auto-matched.
- Each ready control produces an independent `ControlResult`; results are not netted across revenue, VAT, receivable, or bank semantics.
- The Tauri audit command has one ingestion loop. It reads the selected worksheet once for that execution, normalizes each source once, prepares one provenance-preserving index, then runs every ready control against the retained datasets.
- The scenario workflow remains available under advanced settings. The default UI is the “Bộ hồ sơ kế toán” dashboard and does not expose `SourceRole`.

## Consequences

- Control availability is explainable as `READY`, `MISSING_SOURCE`, `NEEDS_MAPPING`, `NEEDS_REVIEW`, or `NOT_APPLICABLE`.
- No account balance is inferred from an unrelated source. In particular, TK131 is not inferred from TK112 or a bank statement.
- A wider session can legitimately contain controls with different effective sub-periods, while every effective period remains bounded by the explicit session period.
- Workbook preview/recognition and audit execution remain separate UI operations; execution itself is import-once. Persisting a normalized session across application restarts is outside V16.

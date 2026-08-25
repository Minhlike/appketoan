# Audit Workspace and Control Planner

## Normal workflow

```text
Import accounting package
  -> recognize source capabilities
  -> require an explicit accounting period
  -> normalize and index each source once
  -> generate control plans
  -> run every READY control against reused datasets
```

The normal UI presents source names, recognized business meaning, record counts, warnings, missing evidence, and control status. Scenario selection, `SourceRole`, and manual column mapping are advanced operations.

## Implemented controls

| Control | Required capabilities | Period policy |
|---|---|---|
| Revenue / invoice / register / ledger | `Invoice`, `SalesTransaction`, `LedgerEntry(account=511)` | Required transactional date |
| Bank / ledger reconciliation | `LedgerEntry(account=112)`, `BankTransaction` | Required transactional date |
| Partner identity | `PartnerMaster`; transaction capabilities are optional | Date not required for the master check |
| Sales analysis | `AnalyticalSalesReport` | Date not required for the typed report check |

The following definitions are planning placeholders only:

- VAT accounting requires `LedgerEntry(account=3331)`.
- Receivable control requires `LedgerEntry(account=131)`.

They do not implement TK3331 or TK131 reconciliation behavior in V16.

## Period safety

The session period is mandatory and inclusive. Each transactional source is filtered to it before matching. The planner then calculates the overlap of date evidence across the required sources of each control. Only that overlap is supplied to the matcher.

If a required record has no parseable date, or if required sources do not overlap, the control is `NEEDS_REVIEW` and is not auto-run. This is intentionally fail-closed.

## Data reuse and provenance

`SourceReuseEvidence` records one read, one normalization, and one prepared index for every source in an audit execution. Controls receive source IDs and retain the original record IDs and raw-file SHA-256 evidence. No cache key replaces source provenance.

## Result isolation

Every control returns its own status, evidence, findings, source IDs, and missing capabilities. Transactional controls may also include their detailed V15 reconciliation result. Suggested or ambiguous bank links remain review-only and cannot become a passing control merely because amount, direction, and date agree.

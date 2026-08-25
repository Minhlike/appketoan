# V17 Resilience and UI Foundation

## Runtime flow

```text
Native path / byte fallback
  -> validate size and workbook
  -> SHA-256 + prepared-cache lookup
  -> parse and normalize on miss
  -> rebuild period view and indexes
  -> declarative control plan
  -> independently execute ready controls
  -> typed report + unified review queue
```

## Failure boundaries

- Source failures are recoverable and scoped to controls requiring that capability.
- Control failures do not erase independent control results.
- Session-fatal failures use typed errors and safe messages.
- Cancellation is cooperative at source/control boundaries; cancelled work cannot report PASS.
- Export failure preserves the current in-memory result.

## Reuse and privacy

- The prepared cache exists only for the current application process.
- Keys use content provenance, sheet, mapping, kind, and schema—not filename.
- Period filtering and indexes are rebuilt for every run.
- No raw workbook or normalized accounting dataset is persisted by V17.

## Performance evidence (Windows GNU, debug test profile, one iteration)

- 100k one-semantic: 4.85 s.
- 100k Invoice/Sales Register, three semantics: 8.405 s.
- 100k tri-source, four controls: 13.973 s.
- V15 tri-source reference: 9.396 s.
- Workspace prepare: 2.574 s; multi-control execute: 19.580 s; total: 22.155 s; reused execute: 18.680 s.

The last workspace run includes additional independent partner and sales-analysis controls and is not a single-control speed comparison. One iteration is insufficient for median/p95, and peak memory was not measured. The current multi-control execution cost is a performance follow-up, not a correctness relaxation.

## Deferred boundaries

- No TK131 or TK3331 accounting control is implemented.
- TK112 running balance remains not verified without opening/running/closing balance evidence.
- Cancellation does not interrupt an active inner matcher loop.
- Native path ingestion avoids browser `number[]` transfer but currently buffers each workbook for parsing.
- Detailed backend progress events and pure IPC serialization timing are not yet available.

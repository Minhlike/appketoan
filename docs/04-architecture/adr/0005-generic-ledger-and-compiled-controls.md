# ADR 0005: Generic Ledger Entry and Compiled Semantic Controls

## Status

Accepted, 2026-08-25.

## Context

Accounting source kinds such as TK112, TK511, TK131 and TK3331 are useful
adapters, but they must not create separate ledger models or rebuild indexes
for every semantic control. A source may participate in several semantics.

## Decision

- Keep `DataSourceKind` as a compatibility and UI adapter layer.
- Normalize ledger-shaped canonical records through the additive `LedgerEntry`
  view; account metadata, counter-account, signed debit/credit, document,
  partner, date and description remain available without breaking old payloads.
- Compile one physical `SourceIndex` per secondary source, then attach all
  semantic rules as `CompiledControl` entries that reference that index.
- Store control evidence per semantic comparison. Group-level target IDs are
  de-duplicated only for navigation and candidate consumption.
- Bank matching uses a separate deterministic candidate policy: direction,
  rule amount, date, then unique reference or counterparty evidence. It never
  requires an invoice number or tax code and never uses fuzzy scoring.
- Aggregate subset evaluation is capped at twelve candidates and stops after
  the second valid subset, returning an ambiguous/review result rather than
  spending unbounded CPU.
- Summary/export discrepancy magnitude is gross absolute discrepancy; opposite
  semantic variances must never net to a false zero.

## Consequences

Legacy scenario kinds stay compatible while new ledgers can use the generic
view. Physical source indexes are no longer cloned per rule. A deterministic
bank candidate without enough evidence is reviewable rather than accepted.

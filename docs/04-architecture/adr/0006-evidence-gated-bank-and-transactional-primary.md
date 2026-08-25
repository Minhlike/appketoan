# ADR 0006: Evidence-Gated Bank Matching and Transactional Primary

## Status

Accepted

## Context

Bank statement amounts, directions, and dates can coincide without proving the
same business event. A session may also load typed reference controls before
transactional datasets, which must not determine the matching primary.

## Decision

- Require unique strong reference, transaction identifier, or strong
  counterparty evidence for an accepted bank match after compatible monetary
  direction, amount, and date checks.
- Treat a unique compatible amount/direction/date-only candidate as a
  non-consuming review suggestion.
- Exclude typed reference controls from the transactional session and resolve
  its primary after that split, preferring an explicit transactional primary,
  then E-Invoice, then the remaining first source.
- Scan all direct candidates before bounded aggregate subset search. Refuse
  aggregate matching above the configured complexity budget.

## Consequences

The system favors audit review over automatic cash matching when evidence is
weak. Reference sources can coexist with transaction sources without affecting
transactional source selection. Aggregate matching remains deterministic and
fails closed under high candidate counts.

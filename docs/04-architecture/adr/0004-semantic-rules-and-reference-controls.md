# ADR 0004: Semantic Rules and Reference Controls

## Context

Some source pairs require several independent monetary semantics. Partner masters and sales-analysis reports are controls, not transactional inputs. Bank reconciliation also requires direction, not absolute amount alone.

## Decision

- Evaluate all explicit rules for each source-kind pair and fail the overall group when any required semantic fails.
- Reject opposite ledger-112/bank money directions before exact, tolerance, fallback, and aggregate candidate resolution.
- Run partner-master and sales-analysis scenarios as typed reference controls with one source; do not place their rows into the transaction matcher.
- Treat mapped invoice lifecycle values other than standard as `NEEDS_REVIEW` until dedicated business rules are defined.

## Consequences

The UI receives a dedicated reference-control result for one-source control scenarios. Tri-source invoice/sales-register/TK511 discrepancies surface as high-priority review rather than a successful partial match.

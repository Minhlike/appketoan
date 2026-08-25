# ADR 0007: Direct-First Aggregate and Bank Review Conservation

## Status

Accepted

## Context

Aggregate subset enumeration is exponential and must never be reached for an
unbounded candidate set. Separately, a bank candidate linked to a manual-review
decision is neither accepted nor truly unlinked; reporting it again as missing
creates contradictory audit evidence.

## Decision

For each transactional comparison:

1. Scan every available candidate once for direct 1:1 matches.
2. Classify multiple direct matches as ambiguous.
3. Accept exactly one direct match without aggregate subset search.
4. With no direct match, stop when aggregate matching is disabled.
5. With no direct match and a candidate count above the fixed budget, return
   `COMPLEXITY_LIMIT`.
6. Only otherwise perform the bounded subset search.

For bank controls, maintain separate accepted and review-linked secondary ID
sets. The residual sweep emits missing-source groups only for IDs in neither
set. A later accepted decision supersedes a prior review-linked classification.

## Consequences

No exponential enumeration or bit shift can occur above the aggregate budget.
Direct evidence has deterministic precedence. Suggested and ambiguous bank
records remain visible as review links without being counted as accepted or
true bank-only records.

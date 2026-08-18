# ADR 0002: Deterministic Matching Pipeline and Audit Trail

## Status
Accepted

## Context
In financial accounting, reconciliation outcomes must be 100% reproducible, explainable, and verifiable. Uncontrolled fuzzy matching or black-box heuristic guessing creates legal and audit risks for chief accountants and auditors.

## Decision
1. **Deterministic Rule Execution**: Matching runs through discrete, sequential rule passes. Every match or mismatch is assigned a deterministic `MatchStatus` and a specific `reasonCode`.
2. **Explicit Variance Disclosure**: If records are matched within a tolerance window (e.g. 1 VND rounding or 2-day date delta), the exact variance is explicitly recorded in `FieldDiscrepancy` and flagged as `MATCHED_WITH_TOLERANCE`.
3. **No Silent Auto-Correction**: The engine never silently alters financial numbers or assumes matches without unambiguous key / bucket correspondence.
4. **Complete Audit Trail**: Every output group retains primary and target record references, original source rows, and timestamped session metadata.

## Consequences
- **Positive**: 100% auditable results compliant with Vietnamese accounting audit standards; reproducible results across runs.
- **Negative**: Ambiguous cases require user review instead of AI-based guessing.

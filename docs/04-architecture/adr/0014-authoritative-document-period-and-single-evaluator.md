# ADR 0014: Authoritative Document Period and Single Revenue Evaluator

- Status: Accepted
- Date: 2026-08-26

## Context

The V16 planner intersected the earliest/latest date evidence of required sources. That policy is safe for bank matching, but unsafe for a document-completeness control: if one source starts late or ends early, the intersection can remove the exact first/last-period document that should be reported missing.

V18 also produced both a full generic `ReconciliationResult` and a full typed `DocumentIntegrityResult` for the same revenue records. The duplicate execution increased CPU and peak memory while the typed result was already authoritative.

## Decision

For `REVENUE_INVOICE_REGISTER_LEDGER`, the explicit user-selected `AccountingPeriod` is the authoritative reconciliation boundary. Source earliest/latest dates remain evidence and warnings. Records outside the session period are excluded once during preparation; missing or unparseable dates remain review-only and never enter match indexes.

The Audit Workspace revenue control executes only the typed document-integrity evaluator over prepared datasets. It borrows prepared slices when capability projection and review-row handling do not require an owned subset. The optional legacy `reconciliationResult` is not materialized for this control. The advanced scenario workflow continues to expose the generic matcher unchanged.

The bank control keeps its existing per-control intersection and fail-closed behavior because bank evidence and document-completeness evidence have different acceptance semantics.

## Consequences

- Missing/extra documents on the first or last day of the selected period cannot be hidden by a source coverage gap.
- Source coverage remains visible without becoming an implicit boundary.
- Non-regular invoice lifecycle and every field-level error remain fail-closed.
- Audit Workspace avoids a second full revenue pipeline and a second large result graph.
- Consumers that require the legacy revenue result must use the preserved advanced scenario path; the Audit Workspace field remains optional by contract.

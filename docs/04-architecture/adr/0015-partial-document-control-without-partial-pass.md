# ADR 0015: Partial Document Control Without Partial PASS

- Status: Accepted
- Date: 2026-08-26

## Context

The revenue document control declares Invoice, Sales Register, and TK511 as required evidence. The planner previously refused to execute the entire control when TK511 was absent, even when Invoice and Sales Register were both recognized and in the selected accounting period. This suppressed valid field-level evidence and made a two-source dossier appear unusable.

Treating TK511 as simply optional would create the opposite risk: an exact Invoice-to-Register result could be presented as a passed tri-source control. The product must use available evidence while remaining explicit about missing evidence.

## Decision

Invoice and Sales Register are the minimum executable capabilities for the revenue document evaluator. When both are unique and usable, the planner runs the authoritative user-period control even if TK511 is missing.

TK511 remains a required capability for complete control assurance:

- the plan and result retain `LedgerEntry(account=511)` in `missingCapabilities`;
- Invoice-to-Register performs all exact date, document-number, pretax, VAT, and total checks;
- every Invoice-to-TK511 field is `NOT_CHECKED` when the ledger source is absent;
- no per-document `MISSING_IN_TK511` finding is fabricated without a loaded TK511 dataset;
- the control and every invoice case remain `NEEDS_REVIEW` and `documentsPass` remains false;
- the result separately reports exact Invoice-to-Register pairs and fully matched tri-source documents.

Controls with no loaded source stay visible in the Control Plan but do not enter the actionable Review Queue. A partially loaded control remains actionable.

## Consequences

- A two-file Invoice/Register workflow produces useful evidence instead of `NOT_RUN`.
- Missing source evidence cannot become a partial PASS.
- Missing TK511 is distinguished from a loaded TK511 that lacks a particular document.
- The selected accounting period remains authoritative and source coverage remains warning evidence.
- Existing full tri-source, lifecycle, duplicate, bank, and performance behavior is preserved.

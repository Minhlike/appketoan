# Architecture & Technical Decisions

## Decision 001: Separation of Concerns (Domain Engine vs Desktop Shell)
- **Decision**: `crates/reconciliation-core` is a standalone Rust library without any UI, Tauri, or browser dependencies.
- **Rationale**: Enables lightning-fast native CLI unit/integration tests and deterministic cross-platform accounting logic.

## Decision 002: Monetary Arithmetic Representation
- **Decision**: Use `rust_decimal::Decimal` with `rust_decimal_macros` for all monetary computations, differences, sums, and tolerances.
- **Rationale**: Eliminates IEEE-754 floating point precision errors (e.g. `0.1 + 0.2 = 0.30000000000000004`), essential for audit compliance.

## Decision 003: Semantic Accounting Column Classification
- **Decision**: Introduce explicit `debit_amount_column` and `credit_amount_column` in `ColumnMapping` and never conflate them with `total_amount_column`.
- **Rationale**: In Vietnamese accounting (VAS/Circular 200/133), TK511 revenue is recorded under "Phát sinh Có", and comparing against Invoice pretax requires semantic awareness of credit amounts.

## Decision 004: Ingestion Filtering Hierarchy
- **Decision**:
  1. Header sniffer ignores preliminary titles/meta rows.
  2. Subtotal/summary scanner evaluates all cells in a row for keywords ("SỐ DƯ ĐẦU KỲ", "PHÁT SINH TRONG KỲ", "TỔNG CỘNG").
  3. Empty/zero amount invoice rows without transactional values are filtered out of the reconciliation scope.
- **Rationale**: Prevents summary and placeholder rows from creating false discrepancies in reconciliation runs.

## Decision 005: Multi-Pass Deterministic Matching Pipeline
- **Decision**:
  - Pass 0: Intra-source duplicate detection.
  - Pass 1: Primary Document Number & Series key matching.
  - Pass 2: Secondary Tax ID + Amount key matching (only applied to doc-less rows to prevent false-matching different doc numbers).
  - Pass 3: Residual sweep for missing records on either side.
  - Aggregate 1-to-N: Sum target amounts within group before evaluation.
- **Rationale**: Guarantees zero false-positive matches for invoices with identical amounts but distinct invoice numbers.

## Decision 006: Semantic Rules and Reference Controls Fail Closed
- **Decision**: Evaluate every explicit comparison rule for a source-kind pair; apply monetary direction compatibility before any candidate is accepted; execute partner-master and sales-analysis sources as typed one-source controls outside the transactional matcher.
- **Rationale**: A revenue-only pass cannot mask VAT/receivable divergence, equal opposite cash flows cannot match, and non-transactional sources must not be misclassified as ledger transactions.

## Decision 007: Generic Ledger Views and Compiled Controls
- **Decision**: Preserve account-specific source kinds as adapters while exposing additive `LedgerEntry` views. Build one physical index per source and compile all semantic controls against it; bank candidates use deterministic evidence only; aggregate matching is bounded.
- **Rationale**: This prevents per-rule index duplication, semantic overwrite/netting, false bank matches, and pathological aggregate runtime without breaking existing scenario contracts.

## Decision 008: Source-Aware Presentation and Fail-Closed Aggregate Budget
- **Decision**: `SemanticFieldComparison` is the presentation/export source of truth; labels always include its actual secondary source. Bank mappings are typed on canonical records. Aggregate matching either searches the full bounded candidate set or emits `COMPLEXITY_LIMIT`; disabled aggregate runs only full 1:1 scans.
- **Rationale**: A semantic label alone cannot identify a control in a multi-source audit, and truncating candidates before acceptance can create false matches.

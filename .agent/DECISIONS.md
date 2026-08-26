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

## Decision 009: Evidence-Gated Bank Matching and Transactional Primary Derivation
- **Decision**: A bank candidate is accepted only with unique strong evidence after direction/amount/date compatibility. A unique compatible amount/date candidate is a non-consuming review suggestion. Before matching, remove typed reference controls and resolve the primary from remaining transactional sources, preferring an explicit transactional primary then E-Invoice.
- **Rationale**: Coincidental cash movements must not be consumed as accounting evidence, and source load order must not turn reference data into a transactional primary.

## Decision 010: Direct-First Aggregate State Machine and Review Conservation
- **Decision**: Scan all direct candidates in O(n). Multiple direct matches are ambiguous; one direct match is accepted without subset search. Only zero direct matches may enter aggregate search, and only within the fixed candidate budget. Track accepted, review-linked, and truly unlinked secondary IDs separately.
- **Rationale**: This prevents exponential enumeration and integer-shift hazards, while preserving audit links without misreporting reviewed bank records as missing from the primary source.

## Decision 011: Capability-Based Audit Session and Per-Control Period Intersection
- **Decision**: Add an `AuditSession` that owns an explicit period, capability catalog, normalized datasets, prepared indexes, generated control plans, and independent control results. Transactional sources are first bounded by the session period, then each control runs only on the intersection of date evidence from its required sources. Legacy account-specific kinds remain adapters; corresponding-account columns do not imply ownership of that ledger capability.
- **Rationale**: This enables import/normalize/index once with multi-control reuse, prevents cross-period false matches, and reports missing TK131/TK3331 evidence without inventing it from unrelated ledgers or bank data. See ADR 0008.

## Decision 012: Structured Execution Evidence
- **Decision**: Keep backend stages, per-control timing, IPC-inclusive overhead, and first-render timing in a typed report. Leave peak memory unset until measured by a reliable process-level mechanism.
- **Rationale**: Performance decisions need comparable workloads and must not convert guessed measurements into release evidence. See ADR 0009.

## Decision 013: Recoverable Audit Boundaries
- **Decision**: Use scoped typed errors, independent control outcomes, cooperative cancellation tokens, and non-destructive export failure. Cancelled controls cannot report PASS.
- **Rationale**: One damaged source or failed control must not erase unrelated audit evidence or crash the desktop process. See ADR 0010.

## Decision 014: Prepared Data Stays In Process
- **Decision**: Cache normalized sources only in memory using content/sheet/mapping/kind/schema identity, with fixed entry/size limits and explicit removal/reset invalidation. Rebuild period views and indexes for each execution.
- **Rationale**: Reruns should avoid repeated parse/normalization without weakening provenance, period correctness, or local-data privacy. See ADR 0011.

## Decision 015: Accounting-First Default UI
- **Decision**: Use a four-step Vietnamese audit workspace and unified actionable review queue as the default; keep role/scenario controls in the preserved advanced workflow.
- **Rationale**: Accounting users should understand what is present, missing, reviewed, and actionable without learning engine terminology. See ADR 0012.

## Decision 016: Dedicated Fail-Closed Document Integrity Control
- **Decision**: Keep the generic matcher unchanged for other controls and add a deterministic document-integrity evaluator for Invoice/Sales Register/TK511. Validate BK before matching, use exact five-field Thuế/BK checks, require non-amount evidence for diagnostic links, retain invalid-date rows outside auto-match indexes, and keep total equality independent from document PASS.
- **Rationale**: Configurable transactional tolerance and amount-oriented matching cannot prove document identity. A separate typed result preserves V17 cache/provenance/error boundaries while preventing duplicates, compensating totals, and diagnostic links from becoming accepted evidence. See ADR 0013.

## Decision 017: User Period Is Authoritative for Document Completeness
- **Decision**: For `REVENUE_INVOICE_REGISTER_LEDGER`, reconcile the full user-selected `AccountingPeriod`; use source earliest/latest only as evidence. Run only the typed document evaluator in Audit Workspace and leave the optional legacy result absent. Preserve the generic matcher in advanced scenarios and preserve bank intersection fail-closed behavior.
- **Rationale**: Intersecting source coverage can hide missing first/last-period documents. Materializing two full revenue result graphs duplicates work and memory without adding authority. See ADR 0014.

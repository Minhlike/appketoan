# Architecture: Reconciliation Engine Pipeline & Component Design

## 1. High-Level Engine Architecture

```text
+-------------------------------------------------------------------------+
|                              UI Layer (React)                           |
+-------------------------------------------------------------------------+
                                   │  Tauri Command: run_reconciliation
                                   ▼
+-------------------------------------------------------------------------+
|                  Application Layer (Tauri Command Handler)              |
+-------------------------------------------------------------------------+
                                   │
                                   ▼
+-------------------------------------------------------------------------+
|                    RECONCILIATION CORE (Rust Crate)                    |
|                                                                         |
|  +-------------------------------------------------------------------+  |
|  | 1. Ingestion & Extraction (Calamine streaming reader)             |  |
|  +-------------------------------------------------------------------+  |
|                                  │ Raw Row Stream                       |
|                                  ▼                                      |
|  +-------------------------------------------------------------------+  |
|  | 2. Header Sniffer & Column Auto-Mapper                            |  |
|  +-------------------------------------------------------------------+  |
|                                  │ Header-Indexed Row Stream            |
|                                  ▼                                      |
|  +-------------------------------------------------------------------+  |
|  | 3. Normalizer & Validator                                         |  |
|  |    - Doc No Normalization ("000123" -> "123")                     |  |
|  |    - Date Parsing (Serial / DD/MM/YYYY)                           |  |
|  |    - Amount Parsing & Invariant Enforcement                       |  |
|  |    - Garbage Filter (Subtotals, Empty Rows)                       |  |
|  +-------------------------------------------------------------------+  |
|                                  │ CanonicalRecord Stream               |
|                                  ▼                                      |
|  +-------------------------------------------------------------------+  |
|  | 4. In-Memory Session Storage & Multi-Key Indexing                 |  |
|  |    - Primary Hash Index: HashMap<(Series, DocNo), RecordId>       |  |
|  |    - Secondary Index: HashMap<TaxId, Vec<RecordId>>               |  |
|  |    - Aggregate Bucket: HashMap<(TaxId, Month), Vec<RecordId>>     |  |
|  +-------------------------------------------------------------------+  |
|                                  │ Indexed Records                      |
|                                  ▼                                      |
|  +-------------------------------------------------------------------+  |
|  | 5. Deterministic Matching Engine                                  |  |
|  |    - Stage 1: Exact Key & Amount Matching                         |  |
|  |    - Stage 2: Tolerance & Date-Window Matching                    |  |
|  |    - Stage 3: Multi-Source 3-Way & 4-Way Verification             |  |
|  |    - Stage 4: Aggregate Group Matching                            |  |
|  |    - Stage 5: Unmatched & Discrepancy Categorization              |  |
|  +-------------------------------------------------------------------+  |
|                                  │ Match Groups & Discrepancies         |
|                                  ▼                                      |
|  +-------------------------------------------------------------------+  |
|  | 6. Discrepancy Analyzer & Summary Aggregator                      |  |
|  |    - Reason Code Assignment                                       |  |
|  |    - Audit Trail & Side-by-Side Diff Generation                   |  |
|  |    - KPI Summary Generation                                       |  |
|  +-------------------------------------------------------------------+  |
|                                  │                                      |
|                                  ▼                                      |
|                        ReconciliationResult                             |
+-------------------------------------------------------------------------+
```

---

## 2. Component Breakdown

### 2.1 Excel Reader (`reconciliation-core::reader`)
- Leverages the high-performance `calamine` Rust library.
- Reads `.xlsx`, `.xls`, `.xlsb`, `.xlsm` directly in-memory or via streaming chunk iterator.
- Zero external Excel / COM automation dependency.

### 2.2 Normalizer (`reconciliation-core::normalizer`)
- Transforms raw text/numbers into strongly typed `CanonicalRecord` instances.
- Reconciles missing totals ($\text{Pretax} + \text{VAT} = \text{Total}$).
- Discards non-transaction rows.

### 2.3 Indexer & Matcher (`reconciliation-core::matcher`)
- Multi-pass deterministic matching pipeline.
- $O(N)$ execution using hash indices.
- Never performs silent fuzzy guessing.

### 2.4 Discrepancy Analyzer (`reconciliation-core::analyzer`)
- Generates field-by-field diffs for unmatched and partially matched groups.
- Emits structured `ReasonCode` (e.g. `ERR_VAT_RATE_MISMATCH`, `WARN_DATE_DIFF_EXCEEDED`).

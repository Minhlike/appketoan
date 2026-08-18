# Functional Requirements: Multi-Source Accounting Reconciliation

## 1. Executive Summary
The system provides a local, high-performance reconciliation platform on Windows capable of ingesting arbitrary sets of Excel workbooks and sheets, normalizing varied accounting formats into a unified Canonical Data Model, and identifying financial matches and discrepancies with zero cloud dependencies.

---

## 2. Ingestion & Multi-Source Support

### 2.1 Arbitrary Source Combinations
The system must not be constrained to a 2-file ("left" vs "right") paradigm. Supported reconciliation configurations include:
- **Bi-Source**: e.g. E-Invoices vs Ledger TK 511; Bank Statement vs Sổ Tiền Gửi.
- **Tri-Source**: e.g. E-Invoices vs Ledger TK 511 vs Ledger TK 3331 (VAT).
- **Multi-Branch Consolidation**: e.g. Branch A Ledger + Branch B Ledger + Branch C Ledger vs Headquarter Ledger.
- **Multi-Period / Multi-Sheet**: Multiple workbooks of the same type across different months or quarters, or multiple sheets within a single workbook.

### 2.2 Standard Accounting Source Types
1. **Electronic Invoices (Hóa đơn điện tử - HĐĐT)**: Input from tax authority portals or e-invoice providers (XML/Excel exports).
2. **Sales Revenue Ledger (Sổ chi tiết TK 511)**: Journal vouchers, revenue records.
3. **VAT Ledger (Sổ chi tiết TK 3331 / Bảng kê thuế GTGT đầu ra)**: Output VAT records.
4. **Accounts Receivable Ledger (Sổ chi tiết TK 131)**: Customer debts, collections, offsets.
5. **Cash & Bank Ledgers (Sổ quỹ tiền mặt, Sổ tiền gửi ngân hàng)**: Receipts and payments.
6. **Bank Statements (Sao kê ngân hàng)**: Bank transaction logs from various Vietnamese commercial banks (Vietcombank, BIDV, Techcombank, ACB, MB, etc.).
7. **Custom User Workbooks**: Arbitrary tabular spreadsheets with user-configured column mapping.

---

## 3. Reconciliation Scenarios & Cardinality
The reconciliation platform must support:
- **1 ↔ 1 Match**: Single invoice matched with a single journal voucher.
- **1 ↔ N Match**: One bank receipt covering multiple invoices; or one invoice settled in multiple installments.
- **N ↔ 1 Match**: Multiple delivery vouchers combined into a single summary invoice.
- **N ↔ M Match**: Multiple grouped invoices matched with multiple grouped payments.
- **Exact Match**: All primary keys, dates (within allowable window), and monetary amounts match exactly.
- **Aggregate Match**: The sum of values across multiple records in Source A equals the sum of values in Source B grouped by a common key (e.g. Partner Tax ID or Reference Code).
- **Partial / Tolerance Match**: Amounts match within acceptable rounding threshold (e.g. $\pm 1, \dots, \pm 1000$ VND) or date tolerance (e.g. $\pm 3$ days).
- **Missing Records**: Records present in Source A but completely absent in Source B (e.g. invoice declared but not booked in ledger).
- **Duplicate Records**: Multiple identical transactions unintentionally recorded in the same source.
- **Ambiguous Match**: Multiple candidate records share the exact same amount and partial keys without sufficient distinguishing metadata.
- **Field-Level Discrepancies**: Key matches, but individual fields differ (e.g. Customer Name typo, VAT Rate mismatch, Tax ID mismatch).

---

## 4. User Interaction & Workflow
1. **Source Ingestion**: User selects or drops Excel workbooks (`.xlsx`, `.xls`).
2. **Source Profiling & Mapping**: System automatically suggests or user confirms source type and column mappings.
3. **Reconciliation Profile Selection**: User selects or customizes matching rules, key fields, and tolerance thresholds.
4. **Offline Local Execution**: Core Rust engine processes records in memory and executes matching algorithms in milliseconds.
5. **Interactive Reconciliation Report**:
   - High-level KPIs: Total Records, Matched Count, Mismatch Count, Missing Count, Total Value Variance.
   - Categorized Data Tables: Matched, Discrepancies, Missing in A, Missing in B, Duplicates.
   - Field-level diff visualizer highlighting exact variances.
6. **Export**: Export clean reconciliation report to Excel with audit trail.

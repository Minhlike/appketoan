# Accounting Rules: Matching Scenarios & Semantic Definitions

## 1. Matching Cardinality Matrix

```
+----------------+-----------------------------------------------------------+
| Cardinality    | Description & Accounting Context                          |
+----------------+-----------------------------------------------------------+
| 1 ↔ 1          | Single document to single voucher.                        |
|                | Example: Invoice #00123 -> Voucher #PKT-045.              |
+----------------+-----------------------------------------------------------+
| 1 ↔ N          | One document matched to multiple vouchers.                |
|                | Example: 1 Bank Statement Credit -> 3 Sales Invoices.     |
+----------------+-----------------------------------------------------------+
| N ↔ 1          | Multiple documents matched to one voucher.                |
|                | Example: 4 Delivery Notes -> 1 Combined VAT Invoice.      |
+----------------+-----------------------------------------------------------+
| N ↔ M          | Batch settlements across multiple invoices and payments.  |
|                | Example: Monthly batch settlement for a major distributor.|
+----------------+-----------------------------------------------------------+
```

---

## 2. Match Status Classifications

1. **`MATCHED_EXACT`**:
   - Primary matching keys match completely (e.g. Invoice Number + Serial + Tax ID, or normalized Ref Code).
   - Financial amounts (Pre-tax Amount, VAT Amount, Total Amount) match to the exact cent/dong ($\Delta = 0$).
   - Transaction dates fall within exact allowable date window.

2. **`MATCHED_WITH_TOLERANCE`**:
   - Primary keys match.
   - Financial variance $|\Delta| \le \epsilon$ where $\epsilon$ is user-configured rounding tolerance (e.g. $\le 1,000$ VND).
   - Date variance $|\Delta t| \le \delta$ where $\delta$ is allowable day delta (e.g. $\le 3$ days).

3. **`MATCHED_AGGREGATE`**:
   - Sub-records in group $A$ sum to group $B$ under a grouping key (e.g. $\sum A_i = \sum B_j$ for same Tax ID & Month).

4. **`MISMATCH_AMOUNT`**:
   - Primary key or partner matches, but total amount differs beyond allowable tolerance ($|\Delta| > \epsilon$).
   - Sub-classifications:
     - `MISMATCH_PRETAX`: Pre-tax amount differs, VAT matches.
     - `MISMATCH_VAT`: VAT rate / VAT amount differs, pre-tax matches (common with 8% vs 10% VAT changes).
     - `MISMATCH_ROUNDING`: Minor variance exceeding threshold.

5. **`MISMATCH_METADATA`**:
   - Monetary amounts match, but metadata differs:
     - `MISMATCH_TAX_ID`: Mismatched buyer/seller Tax Code (Mã số thuế).
     - `MISMATCH_PARTNER_NAME`: Discrepancy in company name spelling.
     - `MISMATCH_DATE`: Transaction dates occur in different fiscal quarters or exceed date threshold.
     - `MISMATCH_ACCOUNT`: Mismatched debit/credit accounting code (e.g. TK 5111 vs TK 5112).

6. **`UNMATCHED_MISSING_IN_TARGET`**:
   - Record exists in Source A (e.g. E-Invoice Portal) but is completely absent in Source B (Ledger TK 511).
   - Indicates: Missing journal entry, omitted revenue, or unrecorded purchase.

7. **`UNMATCHED_MISSING_IN_SOURCE`**:
   - Record exists in Source B (Ledger) but not found in Source A (E-Invoices / Bank).
   - Indicates: Erroneous voucher entry, duplicate ledger entry, or fake/unissued invoice.

8. **`DUPLICATE_SUSPECT`**:
   - Two or more records in the same source share identical invoice numbers, dates, and amounts.

9. **`AMBIGUOUS_MATCH`**:
   - A single record from Source A has multiple equally viable candidates in Source B (same amount and partner without unique invoice ID). Requires user review.

---

## 3. Discrepancy Resolution & Audit Trail
Every non-exact match record must preserve:
- `diff_field`: Name of the diverging field.
- `source_value`: The value in the primary reference source.
- `target_value`: The value in the comparison source.
- `variance_amount`: Numerical variance (if monetary).
- `severity`: `ERROR` (financial discrepancy), `WARNING` (metadata variance), `INFO` (acceptable rounding).

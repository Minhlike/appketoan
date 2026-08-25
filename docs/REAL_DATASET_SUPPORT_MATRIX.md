# Real Dataset Support Matrix (Local Acceptance Only)

All observations below were produced locally through the Rust Calamine reader. The workbooks are ignored by Git; this document contains schema, counts, and approved control totals only.

| Local workbook | Actual format / sheet | Header / data start | Detected kind / confidence | Rows: raw / accepted / rejected | Date range | Mapping and monetary fields | Warnings / outcome |
|---|---|---:|---|---:|---|---|---|
| `T7.2026 Thuế.xlsx` | `.xlsx` / `sheet 1` | 6 / 7 (physical) | `e_invoice` / 0.99 | 64 / 46 / 18 | 2026-07-02 to 2026-07-31 | Invoice number, series, template, buyer/seller MST, pretax, VAT, discount, fee, total, currency, exchange rate, lifecycle/check result | 18 non-monetary rows are retained in source provenance but excluded from monetary reconciliation. |
| `T7.2026.xlsx` | `.xlsx` / `Sheet1` | 6 / 7 | `ledger_511` / 0.70 | 49 / 45 / 4 | 2026-07-02 to 2026-07-31 | Date, document code/no., partner code/name, description, debit/credit | Three summary rows and one non-transaction row rejected. Credit total: 7,223,121,057. |
| `BK.xlsx` | `.xlsx` / `Sheet1` | 6 / 7 | `sales_register` / 0.85 | 293 / 291 / 2 | 2026-01-05 to 2026-08-21 | Date, document code/no., partner code/name, revenue, VAT, discount, receivable, account, description | One summary row plus one empty/non-transaction row rejected. Period must remain part of matching evidence. |
| `DM Khách hàng.xlsx` | `.xlsx` / `Sheet1` | 6 / 7 | `partner_master` / 0.90 | 894 / 894 / 0 | N/A | Partner code/name, address, MST, customer/supplier flags, status | Master records are typed and are not discarded for lacking money. Duplicate MST remains `AMBIGUOUS_MASTER_IDENTITY`. |
| `CÁI tk 112.xlsx` | `.xlsx` / `Sheet1` | 6 / 7 | `ledger_112` / 0.80 | 252 / 248 / 4 | 2026-06-01 to 2026-06-30 | Date, document code/no., partner code/name, description, counteraccount, debit/credit | Three summary rows and one non-transaction row rejected. Debit 23,043,488,603; credit 24,470,271,477. |
| `báo cáo 2 chỉ tiếu T6.2026.xlsx` | `.xlsx` / `Sheet1` | 6 / 7 | `sales_analysis_report` / 0.90 | 89 / 89 / 0 | N/A | Product/group fields, quantity, price, revenue, VAT, discount, receivable, cost, profit | 35 group rows and 54 detail rows. The verified single-layer totals are revenue 45,385,833,451; VAT 103,866,389; receivable 45,489,699,840; cost 39,786,014,723; profit 5,599,818,728. |
| `lich-su-giao-dich(20-08-2026 04_38_08).xls` | Excel 97-2003 `.xls` / `LICH SU GIAO DICH` | 25 / 26 | `bank_statement` / 0.99 | 227 / 225 / 2 | 2026-06-01 to 2026-06-30 | Accounting/transaction date, debit, credit, balance, transaction number, counterpart account/name, description | Proven legacy `.xls` read through the real Rust reader. Two rows had no usable monetary transaction evidence and were excluded. |

## Acceptance Controls

- Invoice monetary control: 46 records; pretax 7,328,121,057; VAT 58,417,949; total 7,386,539,006.
- TK511 control: 45 records; credit 7,223,121,057.
- TK112 balance equation: opening debit balance 1,835,299,550 + period debit 23,043,488,603 - period credit 24,470,271,477 = closing debit balance 408,516,676.
- The legacy bank reader no longer relies on a 25-row header search limit; its actual transaction header is discovered at range row 25.
- Sales-analysis control selects the verified detail layer only after proving equality with the group layer; it never sums both.
- The local acceptance check confirms the specified partner code is present and invoice/register control document 233 is present in both source datasets.

## Privacy Boundary

No transaction description, company name, tax identifier, account number, document value, or workbook content is committed here.

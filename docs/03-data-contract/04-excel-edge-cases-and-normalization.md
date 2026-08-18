# Data Contract: Excel Edge Cases & Normalization Pipeline

## 1. Overview of Excel Formatting Hazards in Accounting
Vietnamese accounting spreadsheets exported from different software (MISA, FAST, Bravo, SAP, ERP, Internet Banking, Tax Portals) contain diverse quirks that must be deterministically sanitized before reconciliation.

```text
Raw Excel Cell Value
        ↓
Data Type Sniffer
        ↓
Sanitizer / Trimmer
        ↓
Canonical Converter
        ↓
CanonicalRecord Field
```

---

## 2. Document Number Normalization (`docNo` & `series`)

### Edge Cases
| Raw Input | Clean Output | Rationale |
|---|---|---|
| `"00001234"` | `"1234"` | Leading zeroes stripped for numeric comparison |
| `1234.0` (numeric cell) | `"1234"` | Numeric float converted to integer string |
| `" 1234 "` | `"1234"` | Whitespace trimmed |
| `"00000000"` | `"0"` | Zero preserved |
| `"HD-00123"` | `"HD-00123"` | Alphanumeric prefix preserved |
| `"1c26taa"` (series) | `"1C26TAA"` | Uppercase normalized |

---

## 3. Tax Code Normalization (`partnerTaxId`)

### Edge Cases
| Raw Input | Clean Output | Rationale |
|---|---|---|
| `"0101234567 "` | `"0101234567"` | Trailing space removed |
| `"MST: 0101234567"` | `"0101234567"` | Text label prefix stripped |
| `"0101234567-001"` | `"0101234567-001"` | 14-digit branch code preserved with dash |
| `"010 123 4567"` | `"0101234567"` | Interior spaces stripped |

---

## 4. Monetary Amount Sanitization (`totalAmount`, `pretaxAmount`, `vatAmount`)

### Parsing Rules
1. **Thousand & Decimal Separators**:
   - Handles European/Vietnamese format: `1.250.000,50` $\rightarrow 1250000.50$.
   - Handles US/UK format: `1,250,000.50` $\rightarrow 1250000.50$.
   - Handles simple integers: `1250000` $\rightarrow 1250000.0$.
2. **Negative Amounts**:
   - Accounting Parentheses format: `(500.000)` $\rightarrow -500000.0$.
   - Leading/trailing minus: `-500000` or `500000-` $\rightarrow -500000.0$.
3. **Currency Symbols & Text**:
   - Strips `₫`, `VND`, `VNĐ`, `USD`, `đồng`.
4. **Empty / Null / Dash Cells**:
   - `""`, `null`, `"-"`, `"N/A"` $\rightarrow `0.0` or `None` depending on field.

---

## 5. Date Parsing Pipeline (`date`)

### Supported Formats
1. **Excel Numeric Serial Dates**:
   - Formula: $\text{Days since 1899-12-30}$ (e.g. `45300` $\rightarrow$ `2024-01-09`).
2. **Standard Vietnamese Text Formats**:
   - `DD/MM/YYYY` (e.g. `15/01/2026` $\rightarrow$ `2026-01-15`).
   - `DD-MM-YYYY` (e.g. `15-01-2026` $\rightarrow$ `2026-01-15`).
   - `YYYY-MM-DD` (ISO-8601).
   - `DD/MM/YYYY HH:mm:ss` $\rightarrow$ Time truncated to date `2026-01-15`.

---

## 6. Row Filtering & Garbage Rejection

The parser must automatically discard non-transactional rows:
1. **Header & Metadata Rows**: Rows before data start row containing company headers, report titles.
2. **Empty Rows**: Rows where all relevant fields are empty or whitespace.
3. **Summary & Subtotal Rows**: Rows where first column contains keywords:
   - `Cộng`, `Tổng cộng`, `Tổng số tiền`, `Total`, `Grand Total`, `Số dư đầu kỳ`, `Số dư cuối kỳ`.
4. **Footer Signature Rows**:
   - `Người lập biểu`, `Kế toán trưởng`, `Giám đốc`, `Thủ trưởng đơn vị`, `Ngày ... tháng ... năm ...`.

---

## 7. Multi-Line Document Aggregation
In invoices or vouchers with multiple item lines sharing the same `docNo` and `date`:
- The engine can optionally group line items into a single document-level `CanonicalRecord` before matching, summing `pretaxAmount` and `vatAmount`.

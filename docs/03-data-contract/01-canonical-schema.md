# Data Contract: Canonical Record Schema

## 1. Concept
A `CanonicalRecord` represents a normalized, source-agnostic accounting transaction extracted from any incoming spreadsheet. All raw records from any file or sheet are parsed and transformed into this standard struct prior to reconciliation.

---

## 2. Canonical Schema Definition

```typescript
export interface CanonicalRecord {
  /** Unique internal identifier within session (e.g. "src_1_row_12") */
  id: string;

  /** Source data source identifier */
  sourceId: string;

  /** Original row index in workbook (1-based) */
  sourceRow: number;

  /** Transaction / Document Date (ISO-8601 YYYY-MM-DD) */
  date?: string;

  /** Primary Document Number (Normalized invoice number or voucher code) */
  docNo?: string;

  /** Invoice Series / Symbol (e.g. "1C24TAA") */
  series?: string;

  /** Invoice Template / Form Code (e.g. "1") */
  templateCode?: string;

  /** Partner Tax Code (MST đối tác - Người mua hoặc Người bán) */
  partnerTaxId?: string;

  /** Partner / Customer / Vendor Name */
  partnerName?: string;

  /** Pre-tax monetary amount (Tiền hàng chưa thuế) in VND */
  pretaxAmount?: number;

  /** VAT monetary amount (Tiền thuế GTGT) in VND */
  vatAmount?: number;

  /** Total monetary amount (Tổng tiền thanh toán) in VND */
  totalAmount: number;

  /** VAT rate percentage (e.g. 0, 5, 8, 10) or tax category */
  vatRate?: string;

  /** Debit Accounting Account (TK Nợ, e.g. "131", "1121") */
  debitAccount?: string;

  /** Credit Accounting Account (TK Có, e.g. "5111", "33311") */
  creditAccount?: string;

  /** Journal / Voucher reference number (Số chứng từ hạch toán) */
  voucherNo?: string;

  /** Transaction description / Memo (Diễn giải nghiệp vụ) */
  description?: string;

  /** Bank account number (for bank statements) */
  bankAccount?: string;

  /** Flexible key-value map preserving unmapped or custom columns */
  rawFields?: Record<string, string>;
}
```

---

## 3. Data Integrity & Normalization Rules
1. **Financial Decimals**: Monies are stored and computed using high-precision 64-bit floating point / fixed-point decimals. Rounding conventions must adhere to standard accounting rounding (half-up).
2. **Missing Totals Calculation**: If `totalAmount` is missing but `pretaxAmount` and `vatAmount` are present, `totalAmount = pretaxAmount + vatAmount`. If `vatAmount` is missing but `totalAmount` and `pretaxAmount` are present, `vatAmount = totalAmount - pretaxAmount`.
3. **Trim & Sanitization**: All string fields (`docNo`, `partnerTaxId`, `partnerName`) must be trimmed of leading/trailing whitespaces, non-printable Unicode characters, and normalized (e.g. uppercase for Tax IDs and series).

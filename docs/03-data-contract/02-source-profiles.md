# Data Contract: Data Source & Source Profiles

## 1. Data Source Representation

```typescript
export type DataSourceKind =
  | "e_invoice"          // Hóa đơn điện tử
  | "ledger_511"         // Sổ chi tiết TK 511 (Doanh thu)
  | "ledger_3331"        // Sổ chi tiết TK 3331 (Thuế GTGT đầu ra)
  | "ledger_133"         // Sổ chi tiết TK 133 (Thuế GTGT đầu vào)
  | "ledger_131"         // Sổ chi tiết TK 131 (Công nợ phải thu)
  | "bank_statement"     // Sao kê ngân hàng
  | "cash_book"          // Sổ quỹ tiền mặt
  | "branch_ledger"      // Báo cáo chi nhánh
  | "custom";            // Tùy biến

export interface DataSource {
  /** Unique ID within session */
  id: string;

  /** User-facing label or alias */
  name: string;

  /** Local file system path to workbook */
  filePath: string;

  /** Selected sheet name */
  sheetName: string;

  /** Inferred or assigned source category */
  kind: DataSourceKind;

  /** Header row index (1-based, default 1) */
  headerRow: number;

  /** Data start row index (1-based, default 2) */
  dataStartRow: number;

  /** Active column mapping profile */
  columnMapping: ColumnMapping;
}

export interface ColumnMapping {
  /** Map of CanonicalRecord field names to Excel column names/letters/indices */
  dateColumn?: string;
  docNoColumn?: string;
  seriesColumn?: string;
  templateCodeColumn?: string;
  partnerTaxIdColumn?: string;
  partnerNameColumn?: string;
  pretaxAmountColumn?: string;
  vatAmountColumn?: string;
  totalAmountColumn?: string;
  vatRateColumn?: string;
  debitAccountColumn?: string;
  creditAccountColumn?: string;
  voucherNoColumn?: string;
  descriptionColumn?: string;
  bankAccountColumn?: string;
}
```

---

## 2. Pre-configured Built-in Profiles
The system provides built-in auto-detection profiles matching standard Vietnamese software exports (MISA SME, FAST, Bravo, Viettel S-Invoice, VNPT Invoice, M-Invoice, Vietcombank, Techcombank, BIDV).

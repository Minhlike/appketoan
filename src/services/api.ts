import { invoke } from "@tauri-apps/api/core";
import type {
  ExcelFileMetadata,
  ReconciliationResult,
  ReconciliationSession,
  ExportSummary,
  PreconfiguredScenario,
} from "../types/dataContract";

import eInvoicesSynthetic from "../../fixtures/synthetic/einvoices_comprehensive_synthetic.json";
import ledger511Synthetic from "../../fixtures/synthetic/ledger_511_comprehensive_synthetic.json";
import goldenResult from "../../fixtures/expected/golden_comprehensive_reconciliation_result.json";

const isTauriRuntime = () => {
  return typeof window !== "undefined" && Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
};

/**
 * Inspects an Excel file from disk path via Tauri IPC or synthetic fallback
 */
export async function inspectExcelFile(filePath: string): Promise<ExcelFileMetadata> {
  if (isTauriRuntime()) {
    return await invoke<ExcelFileMetadata>("cmd_inspect_excel_file", { filePath });
  }

  // Fallback / mock metadata for web dev / tests
  return {
    filePath,
    fileName: filePath.split("/").pop() || filePath.split("\\").pop() || "sample.xlsx",
    fileSizeBytes: 1024 * 45,
    sheets: [
      {
        name: "Sheet1",
        totalRows: 100,
        totalCols: 12,
        detectedHeaderRow: 1,
        detectedDataStartRow: 2,
        columns: [
          "Số HĐ",
          "Ký hiệu",
          "Ngày HĐ",
          "Mã số thuế",
          "Tên khách hàng",
          "Tiền chưa thuế",
          "Thuế GTGT",
          "Tổng tiền",
        ],
        suggestedMapping: {
          docNoColumn: "Số HĐ",
          seriesColumn: "Ký hiệu",
          dateColumn: "Ngày HĐ",
          partnerTaxIdColumn: "Mã số thuế",
          partnerNameColumn: "Tên khách hàng",
          pretaxAmountColumn: "Tiền chưa thuế",
          vatAmountColumn: "Thuế GTGT",
          totalAmountColumn: "Tổng tiền",
        },
        suggestedKind: "e_invoice",
        confidenceScore: 0.95,
        previewRows: [
          ["00000101", "1C26TAA", "05/01/2026", "0109990001", "Công ty Sao Mai", "10,000,000", "1,000,000", "11,000,000"],
        ],
      },
    ],
  };
}

/**
 * Inspects raw bytes of an Excel file dropped or uploaded in browser
 */
export async function inspectExcelBytes(bytes: Uint8Array, fileName: string): Promise<ExcelFileMetadata> {
  if (isTauriRuntime()) {
    return await invoke<ExcelFileMetadata>("cmd_inspect_excel_bytes", {
      bytes: Array.from(bytes),
      fileName,
    });
  }

  return inspectExcelFile(fileName);
}

/**
 * Executes reconciliation session across multiple Excel files
 */
export async function runReconciliation(
  session: ReconciliationSession,
  fileBytesMap?: Record<string, number[]>
): Promise<ReconciliationResult> {
  if (isTauriRuntime()) {
    return await invoke<ReconciliationResult>("cmd_run_reconciliation", {
      session,
      fileBytesMap,
    });
  }

  // Web / dev mode simulation
  await new Promise((r) => setTimeout(r, 400));
  return goldenResult as unknown as ReconciliationResult;
}

/**
 * Exports reconciliation report to Excel (.xlsx)
 */
export async function exportReport(
  result: ReconciliationResult,
  outputPath?: string
): Promise<ExportSummary> {
  if (isTauriRuntime()) {
    return await invoke<ExportSummary>("cmd_export_reconciliation_report", {
      result,
      outputPath,
    });
  }

  // Web simulation
  return {
    outputPath: outputPath || "C:\\Users\\User\\Downloads\\Bao_Cao_Doi_Chieu.xlsx",
    fileSizeBytes: 24500,
    totalGroupsExported: result.groups.length,
    createdAt: new Date().toISOString(),
  };
}

/**
 * Pre-configured standard scenarios with explicit roles and semantic comparison rules
 */
export const PRECONFIGURED_SCENARIOS: PreconfiguredScenario[] = [
  {
    id: "scenario_revenue_standard",
    name: "Đối chiếu Doanh thu (Hóa đơn ↔ Sổ cái TK 511)",
    description: "So khớp Doanh thu tiền hàng chưa thuế giữa Hóa đơn điện tử và Sổ cái TK 511",
    recommendedSources: [
      {
        kind: "e_invoice",
        role: "PRIMARY",
        title: "Bảng kê Hóa đơn điện tử bán ra",
        description: "File xuất từ Cổng Thuế hoặc Nhà cung cấp HĐĐT (VNPT, Viettel, MISA, BKAV)",
        required: true,
      },
      {
        kind: "ledger_511",
        role: "REQUIRED_SECONDARY",
        title: "Sổ cái TK 511 (Doanh thu bán hàng)",
        description: "Sổ chi tiết doanh thu từ phần mềm kế toán (Phát sinh Có)",
        required: true,
      },
    ],
    rules: [
      {
        semantic: "REVENUE",
        title: "Đối chiếu Doanh thu bán hàng",
        description: "Khớp Tiền chưa thuế (HĐĐT) với Phát sinh Có (TK 511)",
        primaryField: "pretaxAmount",
        secondaryField: "creditAmount",
      },
    ],
    defaultToleranceVnd: 10,
    defaultDateDays: 3,
    enableAggregate: true,
  },
  {
    id: "scenario_revenue_vat_receivables",
    name: "Đối chiếu Toàn diện Doanh thu, Thuế & Công nợ (HĐ ↔ 511, 3331, 131)",
    description: "Kiểm tra chu trình bán hàng khép kín: HĐĐT ↔ TK 511 ↔ TK 3331 ↔ Sổ công nợ TK 131",
    recommendedSources: [
      {
        kind: "e_invoice",
        role: "PRIMARY",
        title: "Bảng kê Hóa đơn điện tử",
        description: "HĐĐT bán ra (Nguồn chính)",
        required: true,
      },
      {
        kind: "ledger_511",
        role: "REQUIRED_SECONDARY",
        title: "Sổ cái TK 511",
        description: "Doanh thu bán hàng (Phát sinh Có)",
        required: true,
      },
      {
        kind: "ledger_3331",
        role: "REQUIRED_SECONDARY",
        title: "Sổ cái TK 3331",
        description: "Thuế GTGT phải nộp (Phát sinh Có)",
        required: true,
      },
      {
        kind: "ledger_131",
        role: "OPTIONAL_SECONDARY",
        title: "Sổ công nợ TK 131",
        description: "Công nợ phải thu khách hàng (Phát sinh Nợ)",
        required: false,
      },
    ],
    rules: [
      {
        semantic: "REVENUE",
        title: "Doanh thu chưa thuế",
        description: "Hóa đơn Pretax ↔ TK511 Phát sinh Có",
        primaryField: "pretaxAmount",
        secondaryField: "creditAmount",
      },
      {
        semantic: "VAT",
        title: "Thuế GTGT đầu ra",
        description: "Hóa đơn VAT ↔ TK3331 Phát sinh Có",
        primaryField: "vatAmount",
        secondaryField: "creditAmount",
      },
      {
        semantic: "RECEIVABLE",
        title: "Công nợ phải thu",
        description: "Hóa đơn Tổng thanh toán ↔ TK131 Phát sinh Nợ",
        primaryField: "totalAmount",
        secondaryField: "debitAmount",
      },
    ],
    defaultToleranceVnd: 10,
    defaultDateDays: 3,
    enableAggregate: true,
  },
  {
    id: "scenario_input_vat",
    name: "Đối chiếu Thuế GTGT Đầu vào (HĐ Đầu vào ↔ TK 133)",
    description: "So khớp Thuế GTGT được khấu trừ trên hóa đơn đầu vào với Sổ chi tiết TK 133",
    recommendedSources: [
      {
        kind: "e_invoice",
        role: "PRIMARY",
        title: "Bảng kê Hóa đơn đầu vào",
        description: "Bảng kê HĐ mua vào từ Cổng Thuế",
        required: true,
      },
      {
        kind: "ledger_133",
        role: "REQUIRED_SECONDARY",
        title: "Sổ cái TK 133 (Thuế GTGT đầu vào)",
        description: "Phát sinh Nợ TK 133",
        required: true,
      },
    ],
    rules: [
      {
        semantic: "VAT",
        title: "Thuế GTGT đầu vào",
        description: "Hóa đơn VAT ↔ TK133 Phát sinh Nợ",
        primaryField: "vatAmount",
        secondaryField: "debitAmount",
      },
    ],
    defaultToleranceVnd: 10,
    defaultDateDays: 3,
    enableAggregate: true,
  },
  {
    id: "scenario_bank_reconciliation",
    name: "Đối chiếu Thu tiền Khách hàng (TK 131 ↔ Sao kê Ngân hàng)",
    description: "So khớp tiền thanh toán trên sổ công nợ với dòng tiền ghi có trên sao kê ngân hàng",
    recommendedSources: [
      {
        kind: "ledger_131",
        role: "PRIMARY",
        title: "Sổ công nợ TK 131",
        description: "Phát sinh Có TK 131 (Khách thanh toán)",
        required: true,
      },
      {
        kind: "bank_statement",
        role: "REQUIRED_SECONDARY",
        title: "Sao kê ngân hàng",
        description: "Giao dịch phát sinh Có tài khoản ngân hàng",
        required: true,
      },
    ],
    rules: [
      {
        semantic: "BANK_PAYMENT",
        title: "Dòng tiền thanh toán",
        description: "TK131 Phát sinh Có ↔ Sao kê Phát sinh Có",
        primaryField: "creditAmount",
        secondaryField: "creditAmount",
      },
    ],
    defaultToleranceVnd: 0,
    defaultDateDays: 3,
    enableAggregate: false,
  },
  {
    id: "scenario_custom_multi_source",
    name: "Đối chiếu Tùy biến N-nguồn",
    description: "Tự do cấu hình vai trò từng file và các cột đối chiếu",
    recommendedSources: [],
    rules: [],
    defaultToleranceVnd: 10,
    defaultDateDays: 3,
    enableAggregate: true,
  },
];

/**
 * Returns synthetic demo datasets for instant in-app exploration
 */
export function getDemoDatasets() {
  return {
    eInvoices: eInvoicesSynthetic,
    ledger511: ledger511Synthetic,
  };
}

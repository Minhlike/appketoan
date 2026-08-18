import { invoke } from "@tauri-apps/api/core";
import type {
  ExcelFileMetadata,
  ReconciliationResult,
  ReconciliationSession,
  ExportSummary,
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
 * Pre-configured standard scenarios
 */
export const PRECONFIGURED_SCENARIOS: Array<import("../types/dataContract").PreconfiguredScenario> = [
  {
    id: "scenario_revenue_vat",
    name: "Đối chiếu Doanh thu & Thuế đầu ra (3 nguồn)",
    description: "Khớp Hóa đơn điện tử với Sổ chi tiết TK 511 và Bảng kê thuế GTGT TK 3331",
    recommendedSources: [
      {
        kind: "e_invoice",
        title: "Bảng kê Hóa đơn điện tử bán ra",
        description: "File xuất từ Cổng Thuế hoặc Nhà cung cấp HĐĐT (VNPT, Viettel, MISA, BKAV)",
        required: true,
      },
      {
        kind: "ledger_511",
        title: "Sổ cái TK 511 (Doanh thu bán hàng)",
        description: "Sổ chi tiết doanh thu từ phần mềm kế toán",
        required: true,
      },
      {
        kind: "ledger_3331",
        title: "Sổ cái TK 3331 (Thuế GTGT phải nộp)",
        description: "Sổ thuế GTGT đầu ra",
        required: false,
      },
    ],
    defaultToleranceVnd: 10,
    defaultDateDays: 3,
    enableAggregate: true,
  },
  {
    id: "scenario_revenue_receivables",
    name: "Đối chiếu Doanh thu, Thuế & Công nợ (4 nguồn)",
    description: "Kiểm tra chu trình bán hàng: HĐĐT ↔ TK 511 ↔ TK 3331 ↔ Sổ công nợ TK 131",
    recommendedSources: [
      { kind: "e_invoice", title: "Hóa đơn điện tử", description: "Bảng kê HĐ bán ra", required: true },
      { kind: "ledger_511", title: "Sổ cái TK 511", description: "Doanh thu", required: true },
      { kind: "ledger_3331", title: "Sổ cái TK 3331", description: "Thuế GTGT", required: true },
      { kind: "ledger_131", title: "Sổ công nợ TK 131", description: "Phát sinh Nợ TK 131", required: true },
    ],
    defaultToleranceVnd: 10,
    defaultDateDays: 3,
    enableAggregate: true,
  },
  {
    id: "scenario_bank_reconciliation",
    name: "Đối chiếu Tiền gửi & Sao kê Ngân hàng",
    description: "So khớp Sổ tiền gửi ngân hàng (TK 112) với Sao kê tài khoản từ Internet Banking",
    recommendedSources: [
      { kind: "ledger_511", title: "Sổ tiền gửi TK 112", description: "Sổ chi tiết tài khoản ngân hàng của kế toán", required: true },
      { kind: "bank_statement", title: "Sao kê ngân hàng", description: "File Excel sao kê giao dịch từ ngân hàng", required: true },
    ],
    defaultToleranceVnd: 0,
    defaultDateDays: 3,
    enableAggregate: false,
  },
  {
    id: "scenario_custom_multi_source",
    name: "Đối chiếu Tùy biến N-nguồn",
    description: "Tự do thêm nhiều file Excel bất kỳ và cấu hình cột đối chiếu linh hoạt",
    recommendedSources: [],
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

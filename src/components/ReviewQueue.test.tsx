import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { AuditWorkspaceReport } from "../types/dataContract";
import { ReviewQueue } from "./ReviewQueue";

const report: AuditWorkspaceReport = {
  sessionId: "review-session",
  accountingPeriod: { startDate: "2026-07-01", endDate: "2026-07-31" },
  sourceCatalog: { sources: [] },
  controlPlans: [{
    controlId: "BANK_LEDGER_RECONCILIATION",
    title: "Sổ ngân hàng ↔ Sao kê",
    status: "NEEDS_REVIEW",
    sourceIds: [],
    missingCapabilities: [],
    warnings: ["Bằng chứng ghép giao dịch chưa đủ mạnh."],
  }],
  controlResults: [{
    controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
    status: "FAILED",
    evidence: [],
    findings: [{
      code: "CONTROL_EXECUTION_FAILED",
      severity: "HIGH",
      message: "Không thể hoàn tất kiểm tra doanh thu.",
    }],
    sourceIds: [],
    missingCapabilities: [],
    elapsedMs: 4,
    summaryMetrics: {},
    limitations: [],
    error: {
      code: "CONTROL_EXECUTION_FAILED",
      scope: "CONTROL",
      controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
      safeUserMessage: "Kiểm tra doanh thu chưa hoàn tất.",
      recoverability: "CONTINUE_OTHER_CONTROLS",
      recommendedAction: "Rà soát nguồn doanh thu rồi chạy lại.",
    },
  }],
  sourceReuse: [],
  runStatus: "PARTIAL",
  errors: [{
    code: "INVALID_WORKBOOK",
    scope: "SOURCE",
    sourceId: "broken-source",
    safeUserMessage: "Không thể mở file nguồn.",
    recoverability: "CONTINUE_OTHER_CONTROLS",
    recommendedAction: "Kiểm tra file rồi thử lại.",
  }, {
    code: "CONTROL_EXECUTION_FAILED",
    scope: "CONTROL",
    controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
    safeUserMessage: "Kiểm tra doanh thu chưa hoàn tất.",
    recoverability: "CONTINUE_OTHER_CONTROLS",
    recommendedAction: "Rà soát nguồn doanh thu rồi chạy lại.",
  }],
  metrics: {
    stages: {
      fileReadMs: 0,
      excelParseMs: 0,
      normalizationMs: 0,
      periodFilteringMs: 0,
      capabilityDetectionMs: 0,
      indexConstructionMs: 0,
      controlPlanningMs: 0,
      resultSerializationMs: 0,
      totalBackendMs: 0,
    },
    controlExecution: [],
    sourceCount: 0,
    normalizedRecordCount: 0,
    cacheHits: 0,
    cacheMisses: 0,
    estimatedCacheBytes: 0,
  },
};

describe("ReviewQueue", () => {
  it("keeps source failures and review-only bank evidence actionable", () => {
    render(<ReviewQueue report={report} />);
    expect(screen.getByRole("heading", { name: "Việc cần rà soát" })).toBeDefined();
    expect(screen.getByText("Không thể mở file nguồn.")).toBeDefined();
    expect(screen.getByText("Bằng chứng ghép giao dịch chưa đủ mạnh.")).toBeDefined();
    expect(screen.getByText(/Kiểm tra file rồi thử lại/)).toBeDefined();
    expect(screen.getByText("Kiểm tra doanh thu chưa hoàn tất.")).toBeDefined();
    expect(screen.queryByText("Không thể hoàn tất kiểm tra doanh thu.")).toBeNull();
    expect(screen.queryByText(/đã chấp nhận/i)).toBeNull();
  });

  it("shows a partial loaded control but excludes unrelated controls with no source", () => {
    const partialReport: AuditWorkspaceReport = {
      ...report,
      controlPlans: [{
        controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
        title: "Hóa đơn ↔ Bảng kê ↔ TK511",
        status: "NEEDS_REVIEW",
        sourceIds: ["invoice", "register"],
        missingCapabilities: [{ kind: "LEDGER_ENTRY", account: "511" }],
        warnings: ["Đã đủ Hóa đơn Thuế và Bảng kê để đối chiếu; chưa có TK511."],
      }, {
        controlId: "BANK_LEDGER_RECONCILIATION",
        title: "Sổ ngân hàng ↔ Sao kê",
        status: "MISSING_SOURCE",
        sourceIds: [],
        missingCapabilities: [
          { kind: "LEDGER_ENTRY", account: "112" },
          { kind: "BANK_TRANSACTION" },
        ],
        warnings: [],
      }],
      controlResults: [],
      errors: [],
    };

    render(<ReviewQueue report={partialReport} />);
    expect(screen.getByText(/Đã đủ Hóa đơn Thuế và Bảng kê/)).toBeDefined();
    expect(screen.getByText(/Bổ sung Sổ kế toán TK 511/)).toBeDefined();
    expect(screen.queryByText("Sổ ngân hàng ↔ Sao kê")).toBeNull();
  });
});

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { AuditWorkspaceDashboard } from "./AuditWorkspaceDashboard";
import type { AuditWorkspaceReport } from "../types/dataContract";

describe("AuditWorkspaceDashboard", () => {
  it("shows capability-based ready and missing controls without engine roles", () => {
    const report: AuditWorkspaceReport = {
      sessionId: "audit-synthetic",
      accountingPeriod: { startDate: "2026-07-01", endDate: "2026-07-31" },
      sourceCatalog: {
        sources: [
          {
            sourceId: "invoice",
            sourceName: "Hóa đơn tháng 7",
            sourceKind: "e_invoice",
            capabilities: [{ kind: "INVOICE" }],
            recordCount: 46,
            mappingComplete: true,
            periodEvidence: {
              earliestDate: "2026-07-02",
              latestDate: "2026-07-31",
              recordsInPeriod: 46,
              recordsOutsidePeriod: 0,
              missingOrUnparseableDates: 0,
            },
            warnings: [],
          },
        ],
      },
      controlPlans: [
        {
          controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
          title: "Hóa đơn ↔ Bảng kê ↔ Sổ doanh thu",
          status: "READY",
          sourceIds: ["invoice"],
          missingCapabilities: [],
          warnings: [],
          effectivePeriod: { startDate: "2026-07-02", endDate: "2026-07-31" },
        },
        {
          controlId: "RECEIVABLE_CONTROL",
          title: "Kiểm soát công nợ phải thu",
          status: "MISSING_SOURCE",
          sourceIds: [],
          missingCapabilities: [{ kind: "LEDGER_ENTRY", account: "131" }],
          warnings: [],
        },
      ],
      controlResults: [],
      sourceReuse: [
        {
          sourceId: "invoice",
          readCount: 1,
          normalizeCount: 1,
          indexCount: 1,
          normalizedRecordCount: 46,
          cacheHit: false,
        },
      ],
      runStatus: "PARTIAL",
      errors: [{
        code: "MAPPING_INCOMPLETE",
        scope: "SOURCE",
        sourceId: "invoice",
        safeUserMessage: "Nguồn này cần xác định lại cột dữ liệu.",
        recoverability: "CONTINUE_OTHER_CONTROLS",
        recommendedAction: "Mở thiết lập cột và kiểm tra lại.",
      }],
      metrics: {
        stages: {
          fileReadMs: 1,
          excelParseMs: 1,
          normalizationMs: 1,
          periodFilteringMs: 1,
          capabilityDetectionMs: 1,
          indexConstructionMs: 1,
          controlPlanningMs: 1,
          resultSerializationMs: 1,
          totalBackendMs: 8,
        },
        controlExecution: [],
        sourceCount: 1,
        normalizedRecordCount: 46,
        cacheHits: 0,
        cacheMisses: 1,
        estimatedCacheBytes: 1024,
      },
    };

    render(
      <AuditWorkspaceDashboard
        sources={[{
          id: "invoice",
          source: {
            id: "invoice",
            name: "Hóa đơn tháng 7",
            filePath: "invoice.xlsx",
            sheetName: "Data",
            kind: "e_invoice",
            role: "PRIMARY",
            headerRow: 1,
            dataStartRow: 2,
            columnMapping: {},
          },
          fileMetadata: {
            filePath: "invoice.xlsx",
            fileName: "invoice.xlsx",
            fileSizeBytes: 1024,
            sheets: [{
              name: "Data",
              totalRows: 46,
              totalCols: 8,
              detectedHeaderRow: 1,
              detectedDataStartRow: 2,
              columns: [],
              suggestedMapping: {},
              suggestedKind: "e_invoice",
              confidenceScore: 0.95,
              previewRows: [],
            }],
          },
        }]}
        accountingPeriod={report.accountingPeriod}
        report={report}
      />
    );
    expect(screen.getByText(/Nguồn sử dụng: Hóa đơn tháng 7/)).toBeDefined();
    expect(screen.getByText("Sẵn sàng kiểm tra")).toBeDefined();
    expect(screen.getByText("Thiếu chứng từ / sổ cần thiết")).toBeDefined();
    expect(screen.getByText(/Sổ kế toán TK 131/)).toBeDefined();
    expect(screen.getByText("Nguồn này cần xác định lại cột dữ liệu.")).toBeDefined();
    expect(screen.getByText(/Mở thiết lập cột và kiểm tra lại/)).toBeDefined();
    expect(screen.queryByText(/PRIMARY|REQUIRED_SECONDARY/)).toBeNull();
  });

  it("renders cancelled workspace and control states without implying PASS", () => {
    const cancelledReport: AuditWorkspaceReport = {
      sessionId: "cancelled",
      accountingPeriod: { startDate: "2026-07-01", endDate: "2026-07-31" },
      sourceCatalog: { sources: [] },
      controlPlans: [{
        controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
        title: "Hóa đơn ↔ Bảng kê ↔ Sổ doanh thu",
        status: "READY",
        sourceIds: [],
        missingCapabilities: [],
        warnings: [],
        effectivePeriod: { startDate: "2026-07-01", endDate: "2026-07-31" },
      }],
      controlResults: [{
        controlId: "REVENUE_INVOICE_REGISTER_LEDGER",
        status: "CANCELLED",
        evidence: [],
        findings: [],
        sourceIds: [],
        missingCapabilities: [],
        effectivePeriod: { startDate: "2026-07-01", endDate: "2026-07-31" },
        elapsedMs: 0,
        summaryMetrics: {},
        limitations: [],
      }],
      sourceReuse: [],
      runStatus: "CANCELLED",
      errors: [],
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
    render(
      <AuditWorkspaceDashboard
        sources={[]}
        accountingPeriod={cancelledReport.accountingPeriod}
        report={cancelledReport}
      />
    );
    expect(screen.getAllByText("Đã dừng").length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText(/Không phát hiện sai lệch/)).toBeNull();
  });

  it("renders a legacy partial IPC result when empty collections are omitted", () => {
    const legacyWireReport = {
      sessionId: "legacy-partial-payload",
      accountingPeriod: { startDate: "2026-07-01", endDate: "2026-08-01" },
      sourceCatalog: { sources: [] },
      controlPlans: [{
        controlId: "RECEIVABLE_CONTROL",
        title: "Kiểm soát công nợ phải thu",
        status: "MISSING_SOURCE",
        sourceIds: [],
        missingCapabilities: [{ kind: "LEDGER_ENTRY", account: "131" }],
        warnings: [],
      }],
      controlResults: [{
        controlId: "RECEIVABLE_CONTROL",
        status: "NOT_RUN",
        evidence: [],
        findings: [],
        sourceIds: [],
        missingCapabilities: [{ kind: "LEDGER_ENTRY", account: "131" }],
        elapsedMs: 0,
      }],
      sourceReuse: [],
      runStatus: "COMPLETED",
      errors: [],
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
    } as unknown as AuditWorkspaceReport;

    expect(() => render(
      <AuditWorkspaceDashboard
        sources={[]}
        accountingPeriod={legacyWireReport.accountingPeriod}
        report={legacyWireReport}
      />
    )).not.toThrow();
    expect(screen.getAllByText("Chưa chạy").length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText(/Không phát hiện sai lệch/)).toBeNull();
  });
});

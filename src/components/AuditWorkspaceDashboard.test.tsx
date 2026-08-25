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
        },
      ],
    };

    render(
      <AuditWorkspaceDashboard
        sources={[]}
        accountingPeriod={report.accountingPeriod}
        report={report}
      />
    );
    expect(screen.getByText("Hóa đơn tháng 7")).toBeDefined();
    expect(screen.getByText("Sẵn sàng")).toBeDefined();
    expect(screen.getByText("Thiếu nguồn")).toBeDefined();
    expect(screen.getByText(/Sổ kế toán TK 131/)).toBeDefined();
    expect(screen.queryByText(/PRIMARY|REQUIRED_SECONDARY/)).toBeNull();
  });
});

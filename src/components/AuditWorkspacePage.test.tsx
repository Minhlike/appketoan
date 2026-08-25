import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { IngestedSourceItem } from "../types/auditWorkspace";
import { AuditWorkspacePage } from "./AuditWorkspacePage";

const source: IngestedSourceItem = {
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
};

const noop = () => undefined;

describe("AuditWorkspacePage", () => {
  it("shows a useful empty state and the four-step normal workflow", () => {
    render(
      <AuditWorkspacePage
        sources={[]}
        accountingPeriod={{ startDate: "", endDate: "" }}
        setAccountingPeriod={vi.fn()}
        report={null}
        operationState="IDLE"
        errorMessage={null}
        canRun={false}
        advancedMode={false}
        onAddSource={noop}
        onRemoveSource={noop}
        onUpdateSheet={noop}
        onUpdateKind={noop}
        onUpdateRole={noop}
        onOpenMapping={noop}
        onRun={noop}
        onCancel={noop}
      />
    );
    expect(screen.getByText("1. Nạp bộ hồ sơ")).toBeDefined();
    expect(screen.getByText("4. Rà soát kết quả")).toBeDefined();
    expect(screen.getByText(/Bắt đầu bằng cách kéo các file Excel/)).toBeDefined();
    expect(
      screen.getByRole("button", { name: "Chạy tất cả kiểm tra có thể" }).hasAttribute("disabled")
    ).toBe(true);
  });

  it("exposes cancel and locks mutable intake controls while running", () => {
    const cancel = vi.fn();
    render(
      <AuditWorkspacePage
        sources={[source]}
        accountingPeriod={{ startDate: "2026-07-01", endDate: "2026-07-31" }}
        setAccountingPeriod={vi.fn()}
        report={null}
        operationState="RUNNING"
        errorMessage={null}
        canRun={false}
        advancedMode={false}
        onAddSource={noop}
        onRemoveSource={noop}
        onUpdateSheet={noop}
        onUpdateKind={noop}
        onUpdateRole={noop}
        onOpenMapping={noop}
        onRun={noop}
        onCancel={cancel}
      />
    );
    fireEvent.click(screen.getByRole("button", { name: "Dừng kiểm tra" }));
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(screen.getByLabelText("Từ ngày").hasAttribute("disabled")).toBe(true);
    expect(screen.getByRole("status").textContent).toContain("Đang đọc, chuẩn hóa");
  });

  it("keeps internal source roles hidden by default and available in advanced mode", () => {
    const { rerender } = render(
      <AuditWorkspacePage
        sources={[source]}
        accountingPeriod={{ startDate: "2026-07-01", endDate: "2026-07-31" }}
        setAccountingPeriod={vi.fn()}
        report={null}
        operationState="READY"
        errorMessage={null}
        canRun
        advancedMode={false}
        onAddSource={noop}
        onRemoveSource={noop}
        onUpdateSheet={noop}
        onUpdateKind={noop}
        onUpdateRole={noop}
        onOpenMapping={noop}
        onRun={noop}
        onCancel={noop}
      />
    );
    expect(screen.queryByText("Vai trò trong kịch bản:")).toBeNull();
    rerender(
      <AuditWorkspacePage
        sources={[source]}
        accountingPeriod={{ startDate: "2026-07-01", endDate: "2026-07-31" }}
        setAccountingPeriod={vi.fn()}
        report={null}
        operationState="READY"
        errorMessage={null}
        canRun
        advancedMode
        onAddSource={noop}
        onRemoveSource={noop}
        onUpdateSheet={noop}
        onUpdateKind={noop}
        onUpdateRole={noop}
        onOpenMapping={noop}
        onRun={noop}
        onCancel={noop}
      />
    );
    expect(screen.getByText("Vai trò trong kịch bản:")).toBeDefined();
  });
});

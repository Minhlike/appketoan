import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { DocumentIntegrityResult } from "../types/dataContract";
import { DocumentIntegrityPanel } from "./DocumentIntegrityPanel";

const result: DocumentIntegrityResult = {
  summary: {
    invoiceRecords: 1,
    salesRegisterRecords: 1,
    ledger511Records: 1,
    invoiceSalesRegisterExact: 1,
    fullyMatched: 45,
    dateMismatch: 1,
    invoiceNumberMismatch: 0,
    pretaxMismatch: 0,
    vatMismatch: 0,
    totalMismatch: 0,
    missingInBk: 0,
    extraInBk: 0,
    duplicateInvoiceNumber: 0,
    ambiguousMatch: 0,
    invalidDate: 0,
    missingInvoiceNumber: 0,
    invalidAmount: 0,
    missingInTk511: 1,
    invoiceLifecycleNeedsReview: 0,
  },
  totals: [{
    field: "PRETAX",
    invoiceTotal: "100",
    salesRegisterTotal: "100",
    variance: "0",
    status: "EQUAL",
  }],
  totalsEqual: true,
  ledger511Revenue: {
    invoiceTotal: "100",
    ledgerTotal: "0",
    variance: "100",
    status: "MISMATCH",
  },
  ledger511Checked: true,
  documentsPass: false,
  documents: [{
    id: "invoice:233",
    status: "NEEDS_REVIEW",
    invoice: {
      provenance: {
        sourceId: "invoice",
        sourceName: "Hóa đơn Thuế",
        filePath: "invoice.xlsx",
        sheetName: "Data",
        recordId: "invoice-233",
        sourceRow: 48,
      },
      date: "2026-07-06",
      invoiceNumber: "233",
      pretaxAmount: "105000000",
      vatAmount: "10500000",
      totalAmount: "115500000",
    },
    salesRegister: {
      provenance: {
        sourceId: "register",
        sourceName: "Bảng kê",
        filePath: "register.xlsx",
        sheetName: "Data",
        recordId: "register-233",
        sourceRow: 48,
      },
      date: "2026-07-06",
      invoiceNumber: "233",
      pretaxAmount: "105000000",
      vatAmount: "10500000",
      totalAmount: "115500000",
    },
    fieldChecks: [{
      scope: "INVOICE_TO_LEDGER511",
      field: "VAT",
      status: "NOT_CHECKED",
      expectedValue: "10500000",
    }],
    errors: [{
      code: "MISSING_IN_TK511",
      severity: "HIGH",
      message: "HIGH: chứng từ #233 thiếu trong TK511.",
      provenance: [],
    }],
  }],
};

describe("DocumentIntegrityPanel", () => {
  it("shows field-level summary and opens side-by-side evidence with every error code", () => {
    render(<DocumentIntegrityPanel result={result} />);
    expect(screen.getByText("Thuế ↔ BK khớp")).toBeDefined();
    expect(screen.getByText("Khớp đủ 3 nguồn")).toBeDefined();
    expect(screen.getByText(/Tổng bằng nhau không thay thế/)).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "233 ↔ 233" }));
    expect(screen.getByRole("complementary", { name: "Chi tiết đối chiếu chứng từ" })).toBeDefined();
    expect(screen.getByRole("heading", { name: "Hóa đơn Thuế" })).toBeDefined();
    expect(screen.getByRole("heading", { name: "Bảng kê bán hàng" })).toBeDefined();
    expect(screen.getByText("CHƯA ĐỐI CHIẾU")).toBeDefined();
    expect(screen.getAllByText("MISSING_IN_TK511").length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText(/PASS/i)).toBeNull();
  });

  it("shows useful Thuế ↔ BK results without claiming PASS when TK511 is absent", () => {
    const partialResult: DocumentIntegrityResult = {
      ...result,
      summary: {
        ...result.summary,
        ledger511Records: 0,
        invoiceSalesRegisterExact: 1,
        fullyMatched: 0,
        missingInTk511: 0,
      },
      ledger511Checked: false,
      ledger511Revenue: {
        invoiceTotal: "100",
        ledgerTotal: "0",
        variance: "100",
        status: "NOT_VERIFIED",
      },
      documents: [{
        ...result.documents[0],
        errors: [],
        fieldChecks: [
          {
            scope: "INVOICE_TO_SALES_REGISTER",
            field: "DATE",
            status: "MATCH",
            expectedValue: "2026-07-06",
            actualValue: "2026-07-06",
          },
          {
            scope: "INVOICE_TO_LEDGER511",
            field: "DATE",
            status: "NOT_CHECKED",
            expectedValue: "2026-07-06",
          },
        ],
      }],
    };

    render(<DocumentIntegrityPanel result={partialResult} />);
    expect(screen.getByText(/Thuế ↔ BK đã được đối chiếu/)).toBeDefined();
    expect(screen.getByText(/Thuế ↔ BK khớp · TK511 chưa kiểm tra/)).toBeDefined();
    expect(screen.queryByText(/Chứng từ ba nguồn: đạt/)).toBeNull();
    expect(screen.queryByText(/PASS/i)).toBeNull();
  });

  it("renders every simultaneous document error including lifecycle review", () => {
    const errorCodes = [
      "DUPLICATE_INVOICE_NUMBER",
      "MISSING_IN_BK",
      "EXTRA_IN_BK",
      "DATE_MISMATCH",
      "INVOICE_NUMBER_MISMATCH",
      "PRETAX_MISMATCH",
      "VAT_MISMATCH",
      "TOTAL_MISMATCH",
      "INVOICE_LIFECYCLE_NEEDS_REVIEW",
    ] as const;
    const multiErrorResult: DocumentIntegrityResult = {
      ...result,
      summary: { ...result.summary, invoiceLifecycleNeedsReview: 1 },
      documents: [{
        ...result.documents[0],
        errors: errorCodes.map((code) => ({
          code,
          severity: "HIGH",
          message: `${code}: cần rà soát`,
          provenance: [],
        })),
      }],
    };
    render(<DocumentIntegrityPanel result={multiErrorResult} />);
    fireEvent.click(screen.getByRole("button", { name: "233 ↔ 233" }));
    for (const code of errorCodes) {
      expect(screen.getAllByText(code).length).toBeGreaterThanOrEqual(1);
    }
    expect(screen.queryByText(/PASS/i)).toBeNull();
  });
});

import { describe, it, expect } from "vitest";
import type {
  CanonicalRecord,
  ReconciliationResult,
  DataSource,
} from "./dataContract";

import eInvoicesFixture from "../../fixtures/synthetic/synthetic_e_invoices.json";
import ledgerFixture from "../../fixtures/synthetic/synthetic_ledger_511.json";
import expectedResultFixture from "../../fixtures/expected/expected_reconciliation_result.json";

describe("Data Contract & Synthetic Fixture Integrity", () => {
  it("validates synthetic e-invoice records against CanonicalRecord schema", () => {
    const records = eInvoicesFixture as CanonicalRecord[];
    expect(records.length).toBe(3);
    expect(records[0].docNo).toBe("00000012");
    expect(records[0].totalAmount).toBe(11000000);
    expect(records[0].partnerTaxId).toBe("0109999001");
  });

  it("validates synthetic ledger 511 records against CanonicalRecord schema", () => {
    const records = ledgerFixture as CanonicalRecord[];
    expect(records.length).toBe(3);
    expect(records[0].docNo).toBe("12");
    expect(records[0].debitAccount).toBe("131");
    expect(records[0].creditAccount).toBe("5111");
  });

  it("validates expected reconciliation result against ReconciliationResult schema", () => {
    const result = expectedResultFixture as unknown as ReconciliationResult;
    expect(result.sessionId).toBe("mock_session_202601");
    expect(result.summary.exactMatchesCount).toBe(1);
    expect(result.summary.mismatchesCount).toBe(1);
    expect(result.summary.missingInTargetCount).toBe(1);
    expect(result.summary.missingInSourceCount).toBe(1);
    expect(result.groups.length).toBe(4);
    expect(result.groups[0].status).toBe("MATCHED_EXACT");
    expect(result.groups[1].status).toBe("MISMATCH_AMOUNT");
    expect(result.groups[1].discrepancies?.length).toBe(2);
  });

  it("constructs and validates a DataSource configuration", () => {
    const dataSource: DataSource = {
      id: "ds_01",
      name: "Bảng kê HĐĐT Tháng 01/2026",
      filePath: "C:/test/e_invoices.xlsx",
      sheetName: "Sheet1",
      kind: "e_invoice",
      headerRow: 1,
      dataStartRow: 2,
      columnMapping: {
        dateColumn: "A",
        docNoColumn: "B",
        seriesColumn: "C",
        partnerTaxIdColumn: "D",
        partnerNameColumn: "E",
        pretaxAmountColumn: "F",
        vatAmountColumn: "G",
        totalAmountColumn: "H",
      },
    };

    expect(dataSource.kind).toBe("e_invoice");
    expect(dataSource.columnMapping.docNoColumn).toBe("B");
  });
});

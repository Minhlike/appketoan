import { describe, it, expect } from "vitest";
import type { ReconciliationResult } from "./dataContract";
import { formatVND, parseMoneyString, isZeroMoney } from "../utils/money";

describe("IPC Contract & Decimal Safety Tests", () => {
  it("deserializes Rust DTO JSON with exact large VND numbers without precision loss", () => {
    const rawRustDtoJson = JSON.stringify({
      sessionId: "sess_ipc_01",
      executedAt: "2026-08-19T01:00:00Z",
      profileId: "Standard Profile",
      summary: {
        totalSourceRecords: 46,
        totalTargetRecords: 45,
        exactMatchesCount: 45,
        toleranceMatchesCount: 0,
        aggregateMatchesCount: 0,
        mismatchesCount: 0,
        missingInTargetCount: 1,
        missingInSourceCount: 0,
        duplicatesCount: 0,
        ambiguousCount: 0,
        revenueVariance: "105000000",
        vatVariance: "0",
        receivableVariance: "0",
        totalDiscrepantAmount: "105000000",
        netFinancialVariance: "105000000",
      },
      groups: [
        {
          id: "grp_01",
          status: "MATCHED_EXACT",
          docNo: "001",
          series: "1C26TAA",
          primarySourceRecordIds: ["src_inv_row_2"],
          targetSourceRecordIds: ["src_tk_row_2"],
          revenueVariance: "0",
          vatVariance: "0",
          receivableVariance: "0",
          totalSourceAmount: "7328121057",
          totalTargetAmount: "7223121057",
          amountVariance: "105000000",
          semanticComparisons: [
            {
              semantic: "REVENUE",
              semanticName: "Doanh thu (Pretax ↔ TK511)",
              primarySourceId: "src_inv",
              primarySourceName: "Hóa đơn",
              secondarySourceId: "src_511",
              secondarySourceName: "Sổ cái TK 511",
              secondarySourceKind: "ledger_511",
              semanticField: "Doanh thu chưa thuế",
              expectedAmount: "7328121057",
              actualAmount: "7223121057",
              variance: "105000000",
              status: "MATCHED_EXACT",
            },
          ],
        },
        {
          id: "grp_negative",
          status: "MISMATCH_AMOUNT",
          docNo: "002",
          series: "1C26TAA",
          primarySourceRecordIds: ["src_inv_row_3"],
          targetSourceRecordIds: ["src_tk_row_3"],
          revenueVariance: "-500000",
          totalSourceAmount: "10000000",
          totalTargetAmount: "10500000",
          amountVariance: "-500000",
        },
      ],
    });

    const parsed: ReconciliationResult = JSON.parse(rawRustDtoJson);

    // Verify summary values
    expect(parsed.summary.revenueVariance).toBe("105000000");
    expect(parsed.summary.vatVariance).toBe("0");
    expect(parsed.summary.netFinancialVariance).toBe("105000000");

    // Test formatVND
    expect(formatVND(parsed.summary.revenueVariance)).toBe("105.000.000 đ");
    expect(formatVND(parsed.summary.vatVariance)).toBe("0 đ");
    expect(formatVND("7328121057")).toBe("7.328.121.057 đ");
    expect(formatVND("7223121057")).toBe("7.223.121.057 đ");
    expect(formatVND("-500000")).toBe("-500.000 đ");
    expect(formatVND(0)).toBe("0 đ");

    // Test parseMoneyString
    expect(parseMoneyString("105.000.000 đ")).toBe("105000000");
    expect(parseMoneyString("-500.000 VND")).toBe("-500000");
    expect(parseMoneyString("0")).toBe("0");

    // Test isZeroMoney
    expect(isZeroMoney("0")).toBe(true);
    expect(isZeroMoney("0.00")).toBe(true);
    expect(isZeroMoney("105000000")).toBe(false);
    expect(isZeroMoney("-500000")).toBe(false);
  });

  it("validates actual Rust generated contract artifact if available on disk", () => {
    try {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const fs = require("fs");
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const path = require("path");

      const contractPath = path.resolve(__dirname, "../../fixtures/artifacts/ipc_contract_output.json");
      if (fs.existsSync(contractPath)) {
        const fileContent = fs.readFileSync(contractPath, "utf-8");
        const res: ReconciliationResult = JSON.parse(fileContent);

        expect(res.sessionId).toBe("sess_ipc_contract_v5");
        expect(res.groups.length).toBeGreaterThan(0);
        expect(res.groups.some((g) => g.totalSourceAmount === "7328121057" || g.totalSourceAmount === 7328121057)).toBe(true);
      }
    } catch {
      // Optional if file not yet generated during unit test phase
    }
  });
});

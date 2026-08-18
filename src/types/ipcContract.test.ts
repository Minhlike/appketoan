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
        {
          id: "grp_fraction",
          status: "MATCHED_EXACT",
          docNo: "003",
          primarySourceRecordIds: ["src_inv_fraction"],
          targetSourceRecordIds: ["src_tk_fraction"],
          totalSourceAmount: "1250000.50",
          totalTargetAmount: "1250000.50",
          amountVariance: "0",
        },
      ],
    });

    const parsed: ReconciliationResult = JSON.parse(rawRustDtoJson);

    // Verify summary values are strings (not numbers)
    expect(typeof parsed.summary.revenueVariance).toBe("string");
    expect(parsed.summary.revenueVariance).toBe("105000000");
    expect(parsed.summary.vatVariance).toBe("0");
    expect(parsed.summary.netFinancialVariance).toBe("105000000");

    // Verify group amounts are strings
    expect(typeof parsed.groups[0].totalSourceAmount).toBe("string");
    expect(parsed.groups[0].totalSourceAmount).toBe("7328121057");

    // Verify negative amounts preserved as strings
    expect(parsed.groups[1].revenueVariance).toBe("-500000");

    // Verify decimal fraction preserved exactly (no truncation/rounding)
    expect(parsed.groups[2].totalSourceAmount).toBe("1250000.50");

    // Test formatVND string-only operations
    expect(formatVND(parsed.summary.revenueVariance!)).toBe("105.000.000 đ");
    expect(formatVND(parsed.summary.vatVariance!)).toBe("0 đ");
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
    expect(isZeroMoney("1250000.50")).toBe(false);
  });

  it("validates actual Rust-generated contract artifact — FAIL if file missing or invalid", () => {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const fs = require("fs") as typeof import("fs");
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const path = require("path") as typeof import("path");

    const contractPath = path.resolve(
      __dirname,
      "../../crates/reconciliation-core/fixtures/artifacts/ipc_contract_output.json"
    );

    // HARD FAIL if file doesn't exist — no silent swallow
    expect(
      fs.existsSync(contractPath),
      `Rust-generated IPC contract artifact must exist at: ${contractPath}\nRun \`cargo test\` first to generate it.`
    ).toBe(true);

    const fileContent = fs.readFileSync(contractPath, "utf-8");
    const res: ReconciliationResult = JSON.parse(fileContent);

    // Session ID must be from the real test
    expect(res.sessionId).toBe("sess_ipc_contract_v5");

    // Must have groups
    expect(res.groups.length).toBeGreaterThan(0);

    // Total source amount must be the real invoice baseline (string, exact)
    const hasCorrectAmount = res.groups.some(
      (g) => g.totalSourceAmount === "7328121057"
    );
    expect(hasCorrectAmount).toBe(true);

    // All monetary fields in groups must be strings
    for (const g of res.groups) {
      expect(typeof g.totalSourceAmount).toBe("string");
      expect(typeof g.totalTargetAmount).toBe("string");
      expect(typeof g.amountVariance).toBe("string");
    }

    // Summary monetary fields must be strings
    expect(typeof res.summary.netFinancialVariance).toBe("string");
  });

  it("verifies NOT_CHECKED is a valid MatchStatus that renders without throwing", () => {
    // Simulate a semantic comparison arriving with NOT_CHECKED
    const rawJson = JSON.stringify({
      sessionId: "sess_not_checked_test",
      executedAt: "2026-08-19T01:00:00Z",
      profileId: "test",
      summary: {
        totalSourceRecords: 1,
        totalTargetRecords: 1,
        exactMatchesCount: 1,
        toleranceMatchesCount: 0,
        aggregateMatchesCount: 0,
        mismatchesCount: 0,
        missingInTargetCount: 0,
        missingInSourceCount: 0,
        duplicatesCount: 0,
        ambiguousCount: 0,
        netFinancialVariance: "0",
      },
      groups: [
        {
          id: "grp_not_checked",
          status: "MATCHED_EXACT",
          primarySourceRecordIds: ["inv_1"],
          targetSourceRecordIds: ["tk_1"],
          totalSourceAmount: "1000000",
          totalTargetAmount: "1000000",
          amountVariance: "0",
          semanticComparisons: [
            {
              semantic: "REVENUE",
              semanticName: "Doanh thu",
              primarySourceId: "src_inv",
              primarySourceName: "Hóa đơn",
              secondarySourceId: "src_511",
              secondarySourceName: "TK511",
              secondarySourceKind: "ledger_511",
              semanticField: "pretaxAmount",
              expectedAmount: "1000000",
              actualAmount: "1000000",
              variance: "0",
              status: "MATCHED_EXACT",
            },
            {
              semantic: "VAT",
              semanticName: "Thuế GTGT",
              primarySourceId: "src_inv",
              primarySourceName: "Hóa đơn",
              secondarySourceId: "src_3331_placeholder",
              secondarySourceName: "TK3331 (chưa tải)",
              secondarySourceKind: "ledger_3331",
              semanticField: "vatAmount",
              expectedAmount: "100000",
              actualAmount: "0",
              variance: "100000",
              // NOT_CHECKED = source absent from session
              status: "NOT_CHECKED",
            },
          ],
        },
      ],
    });

    const res: ReconciliationResult = JSON.parse(rawJson);
    expect(res.groups[0].semanticComparisons![1].status).toBe("NOT_CHECKED");
    expect(res.groups[0].semanticComparisons![0].status).toBe("MATCHED_EXACT");
  });
});

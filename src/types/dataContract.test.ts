import { describe, it, expect } from "vitest";
import type {
  CanonicalRecord,
  ReconciliationResult,
  ReconciliationProfile,
} from "./dataContract";

import eInvoicesSynthetic from "../../fixtures/synthetic/einvoices_comprehensive_synthetic.json";
import ledger511Synthetic from "../../fixtures/synthetic/ledger_511_comprehensive_synthetic.json";
import ledger3331Synthetic from "../../fixtures/synthetic/ledger_3331_vat_comprehensive_synthetic.json";
import bankSynthetic from "../../fixtures/synthetic/bank_statement_comprehensive_synthetic.json";
import goldenComprehensiveResult from "../../fixtures/expected/golden_comprehensive_reconciliation_result.json";

describe("Data Contract & Comprehensive Fixtures", () => {
  it("validates comprehensive e-invoices fixture", () => {
    const records = eInvoicesSynthetic as CanonicalRecord[];
    expect(records.length).toBe(7);
    expect(records[0].docNo).toBe("00000101");
    expect(records[0].totalAmount).toBe(11000000);
    // Duplicate test check
    expect(records[5].docNo).toBe(records[6].docNo);
  });

  it("validates comprehensive ledger 511 fixture", () => {
    const records = ledger511Synthetic as CanonicalRecord[];
    expect(records.length).toBe(6);
    expect(records[0].debitAccount).toBe("131");
    expect(records[0].creditAccount).toBe("5111");
    // 1-to-N parts check
    expect(records[2].docNo).toBe("104");
    expect(records[3].docNo).toBe("104");
  });

  it("validates comprehensive ledger 3331 VAT fixture", () => {
    const records = ledger3331Synthetic as CanonicalRecord[];
    expect(records.length).toBe(3);
    expect(records[0].creditAccount).toBe("33311");
  });

  it("validates comprehensive bank statement fixture", () => {
    const records = bankSynthetic as CanonicalRecord[];
    expect(records.length).toBe(3);
    expect(records[0].bankAccount).toBe("0011001234567");
  });

  it("validates golden comprehensive reconciliation result", () => {
    const result = goldenComprehensiveResult as unknown as ReconciliationResult;
    expect(result.sessionId).toBe("session_golden_comprehensive_202601");
    expect(result.summary.totalSourceRecords).toBe(7);
    expect(result.summary.totalTargetRecords).toBe(6);
    expect(result.summary.exactMatchesCount).toBe(1);
    expect(result.summary.toleranceMatchesCount).toBe(1);
    expect(result.summary.aggregateMatchesCount).toBe(1);
    expect(result.summary.mismatchesCount).toBe(1);
    expect(result.summary.missingInTargetCount).toBe(1);
    expect(result.summary.missingInSourceCount).toBe(1);
    expect(result.summary.duplicatesCount).toBe(2);

    expect(result.groups.length).toBe(7);
    expect(result.groups[0].status).toBe("MATCHED_EXACT");
    expect(result.groups[1].status).toBe("MISMATCH_AMOUNT");
    expect(result.groups[2].status).toBe("MATCHED_AGGREGATE");
    expect(result.groups[3].status).toBe("MATCHED_WITH_TOLERANCE");
    expect(result.groups[4].status).toBe("UNMATCHED_MISSING_IN_TARGET");
    expect(result.groups[5].status).toBe("UNMATCHED_MISSING_IN_SOURCE");
    expect(result.groups[6].status).toBe("DUPLICATE_SUSPECT");
  });

  it("validates custom reconciliation profile structure", () => {
    const profile: ReconciliationProfile = {
      id: "profile_revenue_vat_3way",
      name: "Đối chiếu Doanh thu & Thuế đầu ra (3 nguồn)",
      sourceIds: ["src_e_invoice", "src_ledger_511", "src_ledger_3331"],
      rules: [
        {
          id: "rule_exact_invoice",
          name: "So khớp chính xác theo Số HĐ và MST",
          primaryKeys: ["series_and_doc_no", "tax_id_and_doc_no"],
          allowDateVarianceDays: 3,
          allowAmountToleranceVnd: 10.0,
          enableAggregateMatch: true,
          aggregateGroupingKeys: ["docNo", "partnerTaxId"],
        },
      ],
    };

    expect(profile.sourceIds.length).toBe(3);
    expect(profile.rules[0].enableAggregateMatch).toBe(true);
  });
});

import { normalizeMoneyInput, parseVietnameseMoneyInput } from "../utils/money";
import { formatDetailReason } from "../components/ResultTable";
import type { MatchGroup, IntakeAnalysisResult } from "./dataContract";

describe("Vietnamese Money Tolerance & Semantic Detail Explanations", () => {
  it("parses and normalizes Vietnamese money input formats exactly without float loss", () => {
    // Valid test cases per V14 policy
    expect(parseVietnameseMoneyInput("0")).toEqual({ success: true, value: "0" });
    expect(parseVietnameseMoneyInput("500")).toEqual({ success: true, value: "500" });
    expect(parseVietnameseMoneyInput("1.000")).toEqual({ success: true, value: "1000" });
    expect(parseVietnameseMoneyInput("10.000")).toEqual({ success: true, value: "10000" });
    expect(parseVietnameseMoneyInput("1.000.000")).toEqual({ success: true, value: "1000000" });
    expect(parseVietnameseMoneyInput("10.000,50")).toEqual({ success: true, value: "10000.50" });
    expect(parseVietnameseMoneyInput("1.000.000,1234")).toEqual({ success: true, value: "1000000.1234" });

    // normalizeMoneyInput helper works on valid inputs
    expect(normalizeMoneyInput("1.000")).toBe("1000");
    expect(normalizeMoneyInput("10.000")).toBe("10000");
    expect(normalizeMoneyInput("1.000.000")).toBe("1000000");
    expect(normalizeMoneyInput("10.000,50")).toBe("10000.50");
    expect(normalizeMoneyInput("1.000.000,1234")).toBe("1000000.1234");

    // Invalid thousands grouping & malformed inputs rejected strictly (NOT converted to 0)
    expect(parseVietnameseMoneyInput("1.00").success).toBe(false); // only 2 digits after dot
    expect(parseVietnameseMoneyInput("10.00.0").success).toBe(false); // malformed grouping
    expect(parseVietnameseMoneyInput("1..000").success).toBe(false); // consecutive dots
    expect(parseVietnameseMoneyInput("1.0000").success).toBe(false); // 4 digits after dot
    expect(parseVietnameseMoneyInput("10,2,3").success).toBe(false); // multiple commas
    expect(parseVietnameseMoneyInput("abc").success).toBe(false); // non-numeric
    expect(parseVietnameseMoneyInput("-500").success).toBe(false); // negative
    expect(parseVietnameseMoneyInput("10.000,12345").success).toBe(false); // > 4 decimals

    expect(() => normalizeMoneyInput("1.00")).toThrow();
    expect(() => normalizeMoneyInput("10.00.0")).toThrow();
    expect(() => normalizeMoneyInput("1..000")).toThrow();
    expect(() => normalizeMoneyInput("1.0000")).toThrow();
    expect(() => normalizeMoneyInput("10,2,3")).toThrow();
    expect(() => normalizeMoneyInput("abc")).toThrow();
    expect(() => normalizeMoneyInput("-500")).toThrow();
    expect(() => normalizeMoneyInput("10.000,12345")).toThrow();
  });

  it("renders detail explanation for Case 1: Revenue MATCHED, VAT & 131 NOT_CHECKED", () => {
    const group: MatchGroup = {
      id: "grp_inv_228",
      status: "MATCHED_EXACT",
      docNo: "228",
      primarySourceRecordIds: ["rec_1"],
      targetSourceRecordIds: ["rec_2"],
      sourceBreakdowns: {},
      discrepancies: [],
      semanticComparisons: [
        {
          semantic: "REVENUE",
          semanticName: "Doanh thu",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_511",
          secondarySourceName: "Sổ cái TK 511",
          secondarySourceKind: "ledger_511",
          semanticField: "Doanh thu",
          expectedAmount: "10000000",
          actualAmount: "10000000",
          variance: "0",
          status: "MATCHED_EXACT",
          primaryRecordIds: ["rec_1"],
          secondaryRecordIds: ["rec_2"],
          discrepancies: [],
        },
        {
          semantic: "VAT",
          semanticName: "Thuế GTGT",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_3331",
          secondarySourceName: "Sổ cái TK 3331",
          secondarySourceKind: "ledger_3331",
          semanticField: "Thuế GTGT",
          expectedAmount: "0",
          actualAmount: "0",
          variance: "0",
          status: "NOT_CHECKED",
          primaryRecordIds: [],
          secondaryRecordIds: [],
          discrepancies: [],
        },
        {
          semantic: "RECEIVABLE",
          semanticName: "Công nợ",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_131",
          secondarySourceName: "Sổ cái TK 131",
          secondarySourceKind: "ledger_131",
          semanticField: "Công nợ",
          expectedAmount: "0",
          actualAmount: "0",
          variance: "0",
          status: "NOT_CHECKED",
          primaryRecordIds: [],
          secondaryRecordIds: [],
          discrepancies: [],
        },
      ],
      revenueVariance: "0",
      vatVariance: "0",
      receivableVariance: "0",
      otherVariance: "0",
      totalSourceAmount: "10000000",
      totalTargetAmount: "10000000",
      amountVariance: "0",
    };

    const detail = formatDetailReason(group);
    expect(detail).toContain("Doanh thu TK511 khớp");
    expect(detail).toContain("Thuế GTGT và Công nợ chưa đối chiếu");
    expect(detail).not.toContain("Khớp hoàn toàn tất cả");
  });

  it("renders detail explanation for Case 2: Invoice #233 missing in TK511", () => {
    const group: MatchGroup = {
      id: "grp_inv_233",
      status: "UNMATCHED_MISSING_IN_TARGET",
      docNo: "233",
      primarySourceRecordIds: ["rec_233"],
      targetSourceRecordIds: [],
      sourceBreakdowns: {},
      discrepancies: [
        {
          fieldName: "docNo",
          message: "Chứng từ #233 không tìm thấy trong nguồn Sổ cái TK 511",
          amountDiff: "105000000",
        },
      ],
      semanticComparisons: [
        {
          semantic: "REVENUE",
          semanticName: "Doanh thu",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_511",
          secondarySourceName: "Sổ cái TK 511",
          secondarySourceKind: "ledger_511",
          semanticField: "Doanh thu",
          expectedAmount: "105000000",
          actualAmount: "0",
          variance: "105000000",
          status: "UNMATCHED_MISSING_IN_TARGET",
          primaryRecordIds: ["rec_233"],
          secondaryRecordIds: [],
          discrepancies: [],
        },
        {
          semantic: "VAT",
          semanticName: "Thuế GTGT",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_3331",
          secondarySourceName: "Sổ cái TK 3331",
          secondarySourceKind: "ledger_3331",
          semanticField: "Thuế GTGT",
          expectedAmount: "0",
          actualAmount: "0",
          variance: "0",
          status: "NOT_CHECKED",
          primaryRecordIds: [],
          secondaryRecordIds: [],
          discrepancies: [],
        },
        {
          semantic: "RECEIVABLE",
          semanticName: "Công nợ",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_131",
          secondarySourceName: "Sổ cái TK 131",
          secondarySourceKind: "ledger_131",
          semanticField: "Công nợ",
          expectedAmount: "0",
          actualAmount: "0",
          variance: "0",
          status: "NOT_CHECKED",
          primaryRecordIds: [],
          secondaryRecordIds: [],
          discrepancies: [],
        },
      ],
      revenueVariance: "105000000",
      vatVariance: "0",
      receivableVariance: "0",
      otherVariance: "0",
      totalSourceAmount: "105000000",
      totalTargetAmount: "0",
      amountVariance: "105000000",
    };

    const detail = formatDetailReason(group);
    expect(detail).toContain("Thiếu TK511: 105.000.000 đ");
    expect(detail).toContain("Thuế GTGT và Công nợ chưa đối chiếu");
    expect(detail).not.toContain("Khớp hoàn toàn");
  });

  it("renders detail explanation for Case 3: All checked semantics matched", () => {
    const group: MatchGroup = {
      id: "grp_all_matched",
      status: "MATCHED_EXACT",
      docNo: "300",
      primarySourceRecordIds: ["rec_1"],
      targetSourceRecordIds: ["rec_2", "rec_3", "rec_4"],
      sourceBreakdowns: {},
      discrepancies: [],
      semanticComparisons: [
        {
          semantic: "REVENUE",
          semanticName: "Doanh thu",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_511",
          secondarySourceName: "Sổ cái TK 511",
          secondarySourceKind: "ledger_511",
          semanticField: "Doanh thu",
          expectedAmount: "10000000",
          actualAmount: "10000000",
          variance: "0",
          status: "MATCHED_EXACT",
          primaryRecordIds: ["rec_1"],
          secondaryRecordIds: ["rec_2"],
          discrepancies: [],
        },
        {
          semantic: "VAT",
          semanticName: "Thuế GTGT",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_3331",
          secondarySourceName: "Sổ cái TK 3331",
          secondarySourceKind: "ledger_3331",
          semanticField: "Thuế GTGT",
          expectedAmount: "1000000",
          actualAmount: "1000000",
          variance: "0",
          status: "MATCHED_EXACT",
          primaryRecordIds: ["rec_1"],
          secondaryRecordIds: ["rec_3"],
          discrepancies: [],
        },
        {
          semantic: "RECEIVABLE",
          semanticName: "Công nợ",
          primarySourceId: "src_inv",
          primarySourceName: "Hóa đơn",
          secondarySourceId: "src_131",
          secondarySourceName: "Sổ cái TK 131",
          secondarySourceKind: "ledger_131",
          semanticField: "Công nợ",
          expectedAmount: "11000000",
          actualAmount: "11000000",
          variance: "0",
          status: "MATCHED_EXACT",
          primaryRecordIds: ["rec_1"],
          secondaryRecordIds: ["rec_4"],
          discrepancies: [],
        },
      ],
      revenueVariance: "0",
      vatVariance: "0",
      receivableVariance: "0",
      otherVariance: "0",
      totalSourceAmount: "11000000",
      totalTargetAmount: "11000000",
      amountVariance: "0",
    };

    const detail = formatDetailReason(group);
    expect(detail).toBe("✓ Khớp hoàn toàn tất cả các tiêu chí đã đối chiếu");
  });
});


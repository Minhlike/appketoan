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

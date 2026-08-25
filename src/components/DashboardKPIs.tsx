import React from "react";
import type { ComparisonSemantic, ReconciliationSummary } from "../types/dataContract";
import { formatVND, isZeroMoney } from "../utils/money";

interface DashboardKPIsProps {
  summary: ReconciliationSummary;
  activeFilter: string;
  onSelectFilter: (status: string) => void;
  /** Semantics that were actually checked in this reconciliation session */
  activeSemantics?: ComparisonSemantic[];
}

export const DashboardKPIs: React.FC<DashboardKPIsProps> = ({
  summary,
  activeFilter,
  onSelectFilter,
  activeSemantics,
}) => {
  const kpis = [
    {
      id: "ALL",
      label: "Tổng chứng từ",
      value: summary.totalSourceRecords + summary.totalTargetRecords,
      sub: `${summary.totalSourceRecords} nguồn chính • ${summary.totalTargetRecords} đối chiếu`,
      badgeClass: "kpi-neutral",
    },
    {
      id: "MATCHED_EXACT",
      label: "Khớp hoàn toàn (100%)",
      value: summary.exactMatchesCount,
      sub: "Khớp đúng số HĐ, ký hiệu & tiền",
      badgeClass: "kpi-success",
    },
    {
      id: "MATCHED_WITH_TOLERANCE",
      label: "Khớp có dung sai",
      value: summary.toleranceMatchesCount,
      sub: "Lệch làm tròn trong ngưỡng",
      badgeClass: "kpi-info",
    },
    {
      id: "MATCHED_AGGREGATE",
      label: "Khớp gộp (1-N / N-1)",
      value: summary.aggregateMatchesCount,
      sub: "1 hóa đơn khớp tổng nhiều dòng",
      badgeClass: "kpi-teal",
    },
    {
      id: "MISMATCH_AMOUNT",
      label: "Sai lệch tiền / Thuế",
      value: summary.mismatchesCount,
      sub: "Khác tiền hàng, thuế hoặc tổng",
      badgeClass: "kpi-danger",
    },
    {
      id: "UNMATCHED_MISSING_IN_TARGET",
      label: "Thiếu bên đối chiếu",
      value: summary.missingInTargetCount,
      sub: "Có ở HĐĐT, chưa vào Sổ cái",
      badgeClass: "kpi-danger",
    },
    {
      id: "UNMATCHED_MISSING_IN_SOURCE",
      label: "Thiếu bên nguồn chính",
      value: summary.missingInSourceCount,
      sub: "Có ở Sổ cái, không có trên Cổng",
      badgeClass: "kpi-warning",
    },
    {
      id: "DUPLICATE_SUSPECT",
      label: "Nghi ngờ trùng lặp",
      value: summary.duplicatesCount,
      sub: "Trùng số chứng từ trong cùng file",
      badgeClass: "kpi-purple",
    },
    {
      id: "AMBIGUOUS_MATCH",
      label: "Cần kiểm tra lại (Ambiguous)",
      value: summary.ambiguousCount,
      sub: "Trùng số HĐ hoặc nhiều tổ hợp gộp",
      badgeClass: "kpi-orange",
    },
    ...(summary.needsReviewCount && summary.needsReviewCount > 0
      ? [
          {
            id: "NEEDS_REVIEW",
            label: "Cần rà soát (Thiếu dữ kiện)",
            value: summary.needsReviewCount,
            sub: "Thiếu số chứng từ hoặc MST",
            badgeClass: "kpi-warning",
          },
        ]
      : []),
  ];

  // Determine which semantics were actually checked in this session
  const revenueChecked = !activeSemantics || activeSemantics.includes("REVENUE");
  const vatChecked = !activeSemantics || activeSemantics.includes("VAT");
  const receivableChecked = !activeSemantics || activeSemantics.includes("RECEIVABLE");

  // Show revenue variance badge ONLY if revenue was actually checked
  const showRevenueVariance =
    revenueChecked && summary.revenueVariance !== undefined;
  // Show VAT variance badge ONLY if VAT was actually checked
  const showVatVariance = vatChecked && summary.vatVariance !== undefined;
  // Show receivable variance badge ONLY if receivable was actually checked
  const showReceivableVariance =
    receivableChecked && summary.receivableVariance !== undefined;

  const hasAnyVariance =
    (showRevenueVariance && !isZeroMoney(summary.revenueVariance!)) ||
    (showVatVariance && !isZeroMoney(summary.vatVariance!)) ||
    (showReceivableVariance && !isZeroMoney(summary.receivableVariance!));


  return (
    <section className="dashboard-section">
      <div className="section-title-row">
        <h2 className="section-title">Kết quả đối chiếu tổng quan</h2>
        <div className="variance-badges-container" style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>

          {showRevenueVariance ? (
            <div className="net-variance-badge" title="Chênh lệch doanh thu trên các control đã chạy">
              <span className="var-label">Lệch Doanh thu:</span>
              <span
                className={`var-value ${
                  isZeroMoney(summary.revenueVariance!) ? "var-zero" : "var-pos"
                }`}
              >
                {formatVND(summary.revenueVariance!)}
              </span>
            </div>
          ) : !revenueChecked ? null : null}

          {vatChecked ? (
            showVatVariance ? (
              <div className="net-variance-badge" title="Chênh lệch thuế GTGT trên các control đã chạy">
                <span className="var-label">Lệch Thuế GTGT:</span>
                <span
                  className={`var-value ${
                    isZeroMoney(summary.vatVariance!) ? "var-zero" : "var-pos"
                  }`}
                >
                  {formatVND(summary.vatVariance!)}
                </span>
              </div>
            ) : null
          ) : (
            <div className="net-variance-badge not-checked-badge" title="Thuế GTGT chưa được đối chiếu trong phiên này">
              <span className="var-label">Thuế GTGT:</span>
              <span className="var-not-checked">CHƯA ĐỐI CHIẾU</span>
            </div>
          )}

          {receivableChecked ? (
            showReceivableVariance ? (
              <div className="net-variance-badge" title="Chênh lệch phải thu trên các control đã chạy">
                <span className="var-label">Lệch Công nợ:</span>
                <span
                  className={`var-value ${
                    isZeroMoney(summary.receivableVariance!) ? "var-zero" : "var-pos"
                  }`}
                >
                  {formatVND(summary.receivableVariance!)}
                </span>
              </div>
            ) : null
          ) : (
            <div className="net-variance-badge not-checked-badge" title="Công nợ phải thu chưa được đối chiếu trong phiên này">
              <span className="var-label">Công nợ:</span>
              <span className="var-not-checked">CHƯA ĐỐI CHIẾU</span>
            </div>
          )}

          {!hasAnyVariance && (
            <div className="net-variance-badge">
              <span className="var-label">Tổng quy mô sai lệch:</span>
              <span
                className={`var-value ${
                  isZeroMoney(summary.totalDiscrepantAmount)
                    ? "var-zero"
                    : "var-pos"
                }`}
              >
                {formatVND(summary.totalDiscrepantAmount)}
              </span>
            </div>
          )}
        </div>
      </div>

      <div className="kpis-grid">
        {kpis.map((kpi) => {
          const isSelected = activeFilter === kpi.id;
          return (
            <button
              type="button"
              key={kpi.id}
              className={`kpi-card ${kpi.badgeClass} ${isSelected ? "kpi-selected" : ""}`}
              onClick={() => onSelectFilter(kpi.id)}
            >
              <div className="kpi-label">{kpi.label}</div>
              <div className="kpi-value">{kpi.value.toLocaleString("vi-VN")}</div>
              <div className="kpi-sub">{kpi.sub}</div>
            </button>
          );
        })}
      </div>
    </section>
  );
};

import React from "react";
import type { ReconciliationSummary } from "../types/dataContract";
import { formatVND, isZeroMoney } from "../utils/money";

interface DashboardKPIsProps {
  summary: ReconciliationSummary;
  activeFilter: string;
  onSelectFilter: (status: string) => void;
}

export const DashboardKPIs: React.FC<DashboardKPIsProps> = ({
  summary,
  activeFilter,
  onSelectFilter,
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

  const hasSemanticBreakdowns =
    (summary.revenueVariance !== undefined && !isZeroMoney(summary.revenueVariance)) ||
    (summary.vatVariance !== undefined && !isZeroMoney(summary.vatVariance)) ||
    (summary.receivableVariance !== undefined && !isZeroMoney(summary.receivableVariance));

  const isNetZero = isZeroMoney(summary.netFinancialVariance);

  return (
    <section className="dashboard-section">
      <div className="section-title-row">
        <h2 className="section-title">Kết quả đối chiếu tổng quan</h2>
        <div className="variance-badges-container" style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap" }}>
          {summary.revenueVariance !== undefined && (
            <div className="net-variance-badge" title="Chênh lệch Doanh thu (Pretax ↔ TK 511)">
              <span className="var-label">Lệch Doanh thu:</span>
              <span
                className={`var-value ${
                  isZeroMoney(summary.revenueVariance) ? "var-zero" : "var-pos"
                }`}
              >
                {formatVND(summary.revenueVariance)}
              </span>
            </div>
          )}

          {summary.vatVariance !== undefined && (
            <div className="net-variance-badge" title="Chênh lệch Thuế GTGT (VAT ↔ TK 3331)">
              <span className="var-label">Lệch Thuế GTGT:</span>
              <span
                className={`var-value ${
                  isZeroMoney(summary.vatVariance) ? "var-zero" : "var-pos"
                }`}
              >
                {formatVND(summary.vatVariance)}
              </span>
            </div>
          )}

          {summary.receivableVariance !== undefined && (
            <div className="net-variance-badge" title="Chênh lệch Công nợ (Total ↔ TK 131)">
              <span className="var-label">Lệch Công nợ:</span>
              <span
                className={`var-value ${
                  isZeroMoney(summary.receivableVariance) ? "var-zero" : "var-pos"
                }`}
              >
                {formatVND(summary.receivableVariance)}
              </span>
            </div>
          )}

          {!hasSemanticBreakdowns && (
            <div className="net-variance-badge">
              <span className="var-label">Chênh lệch tài chính:</span>
              <span
                className={`var-value ${
                  isNetZero
                    ? "var-zero"
                    : String(summary.netFinancialVariance).startsWith("-")
                    ? "var-neg"
                    : "var-pos"
                }`}
              >
                {formatVND(summary.netFinancialVariance)}
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

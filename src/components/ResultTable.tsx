import React, { useMemo, useState } from "react";
import type { MatchGroup, MatchStatus } from "../types/dataContract";
import { formatVND, isZeroMoney } from "../utils/money";

interface ResultTableProps {
  groups: MatchGroup[];
  activeStatusFilter: string;
  onSelectStatusFilter: (status: string) => void;
  onOpenDetail: (group: MatchGroup) => void;
}

export const ResultTable: React.FC<ResultTableProps> = ({
  groups,
  activeStatusFilter,
  onSelectStatusFilter,
  onOpenDetail,
}) => {
  const [searchQuery, setSearchQuery] = useState("");
  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(25);

  const getStatusBadge = (status: MatchStatus) => {
    switch (status) {
      case "MATCHED_EXACT":
        return <span className="status-badge badge-exact">✓ Khớp 100%</span>;
      case "MATCHED_WITH_TOLERANCE":
        return <span className="status-badge badge-tolerance">≈ Dung sai</span>;
      case "MATCHED_AGGREGATE":
        return <span className="status-badge badge-aggregate">∑ Khớp gộp</span>;
      case "MATCHED_WITH_MISSING_SOURCE":
        return <span className="status-badge badge-missing-target">⚠️ Thiếu nguồn bổ trợ</span>;
      case "MISMATCH_AMOUNT":
        return <span className="status-badge badge-mismatch">⚠️ Lệch tiền</span>;
      case "MISMATCH_METADATA":
        return <span className="status-badge badge-mismatch">⚠️ Lệch thông tin</span>;
      case "UNMATCHED_MISSING_IN_TARGET":
        return <span className="status-badge badge-missing-target">✕ Thiếu bên đối chiếu</span>;
      case "UNMATCHED_MISSING_IN_SOURCE":
        return <span className="status-badge badge-missing-source">✕ Thiếu bên nguồn chính</span>;
      case "DUPLICATE_SUSPECT":
        return <span className="status-badge badge-duplicate">⚇ Trùng lặp</span>;
      case "AMBIGUOUS_MATCH":
        return <span className="status-badge badge-ambiguous">? Cần kiểm tra</span>;
      case "NEEDS_REVIEW":
      case "INSUFFICIENT_MATCHING_EVIDENCE":
        return <span className="status-badge badge-mismatch">🔍 Cần rà soát</span>;
      default:
        return <span className="status-badge">{status}</span>;
    }
  };

  const filteredGroups = useMemo(() => {
    return groups.filter((g) => {
      // 1. Status Filter
      if (activeStatusFilter !== "ALL" && g.status !== activeStatusFilter) {
        return false;
      }

      // 2. Search Query Filter
      if (searchQuery.trim().length > 0) {
        const query = searchQuery.toLowerCase().trim();
        const idMatch = g.id.toLowerCase().includes(query);
        const docMatch = g.docNo?.toLowerCase().includes(query) || false;
        const seriesMatch = g.series?.toLowerCase().includes(query) || false;
        const dateMatch = g.date?.toLowerCase().includes(query) || false;
        const partnerMatch = g.partnerName?.toLowerCase().includes(query) || false;
        const refMatch =
          g.primarySourceRecordIds.some((id) => id.toLowerCase().includes(query)) ||
          g.targetSourceRecordIds.some((id) => id.toLowerCase().includes(query));
        const discMatch = g.discrepancies?.some(
          (d) =>
            d.message.toLowerCase().includes(query) ||
            (d.sourceValue && d.sourceValue.toLowerCase().includes(query)) ||
            (d.targetValue && d.targetValue.toLowerCase().includes(query))
        );

        if (!idMatch && !docMatch && !seriesMatch && !dateMatch && !partnerMatch && !refMatch && !discMatch) {
          return false;
        }
      }

      return true;
    });
  }, [groups, activeStatusFilter, searchQuery]);

  const totalPages = Math.max(1, Math.ceil(filteredGroups.length / pageSize || 1));
  const validPage = Math.min(currentPage, totalPages);

  const paginatedGroups = useMemo(() => {
    const start = (validPage - 1) * pageSize;
    return filteredGroups.slice(start, start + pageSize);
  }, [filteredGroups, validPage, pageSize]);

  const renderSemanticStatus = (
    variance: string | number | undefined,
    status: MatchStatus,
    missingLabel: string
  ) => {
    if (variance === undefined || isZeroMoney(variance)) {
      if (status === "MATCHED_EXACT") {
        return <span style={{ color: "#16a34a", fontSize: "0.85rem" }}>✓ Khớp</span>;
      }
      return <span style={{ color: "#94a3b8", fontSize: "0.85rem" }}>-</span>;
    }
    if (status === "UNMATCHED_MISSING_IN_TARGET") {
      return (
        <span style={{ color: "#dc2626", fontWeight: 600, fontSize: "0.85rem" }}>
          ✕ {missingLabel}: {formatVND(variance)}
        </span>
      );
    }
    const isNeg = String(variance).startsWith("-");
    return (
      <span style={{ color: isNeg ? "#dc2626" : "#d97706", fontWeight: 600, fontSize: "0.85rem" }}>
        {formatVND(variance)}
      </span>
    );
  };

  return (
    <section className="results-table-section">
      <div className="table-controls-bar">
        <div className="search-box">
          <span className="search-icon">🔍</span>
          <input
            type="text"
            className="search-input"
            placeholder="Tìm theo số HĐ, ký hiệu, ngày, MST, tên khách hàng..."
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              setCurrentPage(1);
            }}
          />
          {searchQuery && (
            <button
              type="button"
              className="btn-clear-search"
              onClick={() => {
                setSearchQuery("");
                setCurrentPage(1);
              }}
            >
              ✕
            </button>
          )}
        </div>

        <div className="filter-status-pills">
          {[
            { id: "ALL", label: "Tất cả" },
            { id: "MATCHED_EXACT", label: "Khớp 100%" },
            { id: "MISMATCH_AMOUNT", label: "Lệch tiền/thuế" },
            { id: "UNMATCHED_MISSING_IN_TARGET", label: "Thiếu bên đối chiếu" },
            { id: "UNMATCHED_MISSING_IN_SOURCE", label: "Thiếu bên nguồn chính" },
            { id: "MATCHED_AGGREGATE", label: "Khớp gộp" },
            { id: "AMBIGUOUS_MATCH", label: "Cần kiểm tra" },
            { id: "DUPLICATE_SUSPECT", label: "Trùng lặp" },
          ].map((tab) => (
            <button
              type="button"
              key={tab.id}
              className={`filter-pill ${activeStatusFilter === tab.id ? "filter-pill-active" : ""}`}
              onClick={() => {
                onSelectStatusFilter(tab.id);
                setCurrentPage(1);
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <div className="table-wrapper">
        <table className="reconciliation-table">
          <thead>
            <tr>
              <th style={{ width: "45px" }}>STT</th>
              <th style={{ width: "135px" }}>Trạng thái</th>
              <th style={{ width: "150px" }}>Số CT / Ký hiệu</th>
              <th style={{ width: "100px" }}>Ngày</th>
              <th style={{ width: "130px", textAlign: "right" }}>Doanh thu (511)</th>
              <th style={{ width: "130px", textAlign: "right" }}>Thuế GTGT (3331)</th>
              <th style={{ width: "130px", textAlign: "right" }}>Công nợ (131)</th>
              <th>Chi tiết & Lý do sai lệch</th>
              <th style={{ width: "70px", textAlign: "center" }}>Xem</th>
            </tr>
          </thead>
          <tbody>
            {paginatedGroups.length > 0 ? (
              paginatedGroups.map((g, idx) => {
                const rowIndex = (validPage - 1) * pageSize + idx + 1;
                const reasonText =
                  g.discrepancies && g.discrepancies.length > 0
                    ? g.discrepancies.map((d) => d.message).join(" • ")
                    : "✓ Khớp hoàn toàn tất cả các tiêu chí";

                return (
                  <tr
                    key={g.id}
                    className={`table-row ${
                      g.status === "MATCHED_EXACT" ? "row-exact" : "row-attention"
                    }`}
                    onClick={() => onOpenDetail(g)}
                  >
                    <td>{rowIndex}</td>
                    <td>{getStatusBadge(g.status)}</td>
                    <td className="cell-id">
                      <div><strong>#{g.docNo || g.id}</strong></div>
                      {g.series && <div style={{ fontSize: "0.75rem", color: "#64748b" }}>Ký hiệu: {g.series}</div>}
                    </td>
                    <td>{g.date || "-"}</td>
                    <td className="cell-num" style={{ textAlign: "right" }}>
                      {renderSemanticStatus(g.revenueVariance, g.status, "Thiếu TK511")}
                    </td>
                    <td className="cell-num" style={{ textAlign: "right" }}>
                      {renderSemanticStatus(g.vatVariance, g.status, "Thiếu TK3331")}
                    </td>
                    <td className="cell-num" style={{ textAlign: "right" }}>
                      {renderSemanticStatus(g.receivableVariance, g.status, "Thiếu TK131")}
                    </td>
                    <td className="cell-reason" title={reasonText}>
                      {reasonText}
                    </td>
                    <td style={{ textAlign: "center" }}>
                      <button
                        type="button"
                        className="btn-inspect"
                        onClick={(e) => {
                          e.stopPropagation();
                          onOpenDetail(g);
                        }}
                      >
                        🔍 Xem
                      </button>
                    </td>
                  </tr>
                );
              })
            ) : (
              <tr>
                <td colSpan={9} className="empty-table-cell">
                  Không tìm thấy chứng từ nào phù hợp với bộ lọc hiện tại.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination Bar */}
      <div className="pagination-bar">
        <div className="pagination-info">
          Hiển thị{" "}
          <strong>
            {filteredGroups.length === 0 ? 0 : (validPage - 1) * pageSize + 1} -{" "}
            {Math.min(validPage * pageSize, filteredGroups.length)}
          </strong>{" "}
          trên tổng số <strong>{filteredGroups.length}</strong> nhóm đối chiếu
        </div>

        <div className="pagination-actions">
          <label className="page-size-label">
            Số dòng:
            <select
              className="select-control select-sm"
              value={pageSize}
              onChange={(e) => {
                setPageSize(parseInt(e.target.value, 10));
                setCurrentPage(1);
              }}
            >
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
              <option value={250}>250</option>
            </select>
          </label>

          <button
            type="button"
            className="btn btn-sm btn-outline"
            disabled={validPage <= 1}
            onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
          >
            ◀ Trang trước
          </button>

          <span className="page-number-display">
            Trang {validPage} / {totalPages}
          </span>

          <button
            type="button"
            className="btn btn-sm btn-outline"
            disabled={validPage >= totalPages}
            onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
          >
            Trang sau ▶
          </button>
        </div>
      </div>
    </section>
  );
};

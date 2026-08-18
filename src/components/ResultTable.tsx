import React, { useMemo, useState } from "react";
import type { MatchGroup, MatchStatus } from "../types/dataContract";

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

  const formatMoney = (val: number) => {
    return Math.round(val).toLocaleString("vi-VN") + " đ";
  };

  const getStatusBadge = (status: MatchStatus) => {
    switch (status) {
      case "MATCHED_EXACT":
        return <span className="status-badge badge-exact">✓ Khớp 100%</span>;
      case "MATCHED_WITH_TOLERANCE":
        return <span className="status-badge badge-tolerance">≈ Dung sai</span>;
      case "MATCHED_AGGREGATE":
        return <span className="status-badge badge-aggregate">∑ Khớp gộp</span>;
      case "MISMATCH_AMOUNT":
        return <span className="status-badge badge-mismatch">⚠️ Lệch tiền</span>;
      case "MISMATCH_METADATA":
        return <span className="status-badge badge-mismatch">⚠️ Lệch thông tin</span>;
      case "UNMATCHED_MISSING_IN_TARGET":
        return <span className="status-badge badge-missing-target">✕ Thiếu bên B</span>;
      case "UNMATCHED_MISSING_IN_SOURCE":
        return <span className="status-badge badge-missing-source">✕ Thiếu bên A</span>;
      case "DUPLICATE_SUSPECT":
        return <span className="status-badge badge-duplicate">⚇ Trùng lặp</span>;
      case "AMBIGUOUS_MATCH":
        return <span className="status-badge badge-ambiguous">? Kiểm tra</span>;
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
        const refMatch =
          g.primarySourceRecordIds.some((id) => id.toLowerCase().includes(query)) ||
          g.targetSourceRecordIds.some((id) => id.toLowerCase().includes(query));
        const discMatch = g.discrepancies?.some(
          (d) =>
            d.message.toLowerCase().includes(query) ||
            (d.sourceValue && d.sourceValue.toLowerCase().includes(query)) ||
            (d.targetValue && d.targetValue.toLowerCase().includes(query))
        );

        if (!idMatch && !refMatch && !discMatch) {
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

  return (
    <section className="results-table-section">
      <div className="table-controls-bar">
        <div className="search-box">
          <span className="search-icon">🔍</span>
          <input
            type="text"
            className="search-input"
            placeholder="Tìm theo số HĐ, MST, tên khách hàng, mã nhóm..."
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
              <th style={{ width: "60px" }}>STT</th>
              <th style={{ width: "150px" }}>Trạng thái</th>
              <th style={{ width: "160px" }}>Mã nhóm</th>
              <th style={{ width: "140px", textAlign: "right" }}>Tiền Nguồn A</th>
              <th style={{ width: "140px", textAlign: "right" }}>Tiền Nguồn B</th>
              <th style={{ width: "140px", textAlign: "right" }}>Chênh lệch</th>
              <th>Chi tiết & Lý do sai lệch</th>
              <th style={{ width: "90px", textAlign: "center" }}>Xem</th>
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
                    <td className="cell-id" title={g.id}>
                      {g.id}
                    </td>
                    <td className="cell-num">{formatMoney(g.totalSourceAmount)}</td>
                    <td className="cell-num">{formatMoney(g.totalTargetAmount)}</td>
                    <td
                      className={`cell-num ${
                        g.amountVariance === 0
                          ? "var-zero"
                          : g.amountVariance > 0
                          ? "var-pos"
                          : "var-neg"
                      }`}
                    >
                      {formatMoney(g.amountVariance)}
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
                <td colSpan={8} className="empty-table-cell">
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

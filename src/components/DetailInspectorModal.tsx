import React from "react";
import type { MatchGroup, MatchStatus, SemanticFieldComparison } from "../types/dataContract";
import { formatVND, isZeroMoney } from "../utils/money";

interface DetailInspectorModalProps {
  group: MatchGroup | null;
  isOpen: boolean;
  onClose: () => void;
}

export const DetailInspectorModal: React.FC<DetailInspectorModalProps> = ({
  group,
  isOpen,
  onClose,
}) => {
  if (!isOpen || !group) return null;

  const getStatusBadge = (status: MatchStatus) => {
    switch (status) {
      case "MATCHED_EXACT":
        return <span className="status-badge badge-exact">✓ Khớp hoàn toàn</span>;
      case "MATCHED_WITH_TOLERANCE":
        return <span className="status-badge badge-tolerance">≈ Khớp có dung sai</span>;
      case "MATCHED_AGGREGATE":
        return <span className="status-badge badge-aggregate">∑ Khớp gộp (1-N)</span>;
      case "MATCHED_WITH_MISSING_SOURCE":
        return <span className="status-badge badge-missing-target">⚠️ Thiếu nguồn bổ trợ</span>;
      case "MISMATCH_AMOUNT":
        return <span className="status-badge badge-mismatch">⚠️ Sai lệch số tiền / thuế</span>;
      case "MISMATCH_METADATA":
        return <span className="status-badge badge-mismatch">⚠️ Khác thông tin đối tác/ngày</span>;
      case "UNMATCHED_MISSING_IN_TARGET":
        return <span className="status-badge badge-missing-target">✕ Thiếu bên đối chiếu</span>;
      case "UNMATCHED_MISSING_IN_SOURCE":
        return <span className="status-badge badge-missing-source">✕ Thiếu bên nguồn chính</span>;
      case "DUPLICATE_SUSPECT":
        return <span className="status-badge badge-duplicate">⚇ Nghi ngờ trùng lặp</span>;
      case "AMBIGUOUS_MATCH":
        return <span className="status-badge badge-ambiguous">? Cần kiểm tra lại (Ambiguous)</span>;
      default:
        return <span className="status-badge">{status}</span>;
    }
  };

  const comparisons: SemanticFieldComparison[] = group.semanticComparisons || [];

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-container modal-lg" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div>
            <div className="modal-tag-row">
              {getStatusBadge(group.status)}
              <span className="group-id-tag">Mã nhóm: {group.id}</span>
              {group.docNo && <span className="group-id-tag">Số HĐ: #{group.docNo}</span>}
              {group.series && <span className="group-id-tag">Ký hiệu: {group.series}</span>}
            </div>
            <h3 className="modal-title">Kiểm tra chi tiết đối chiếu theo bản chất kế toán</h3>
          </div>
          <button type="button" className="btn-close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="modal-body">
          {/* Document Identity Info */}
          <div className="doc-identity-card" style={{ background: "#f8fafc", padding: "0.75rem 1rem", borderRadius: "6px", marginBottom: "1rem", border: "1px solid #e2e8f0" }}>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: "0.5rem" }}>
              <div><strong>Số chứng từ:</strong> {group.docNo || "N/A"}</div>
              <div><strong>Ký hiệu:</strong> {group.series || "(Trống)"}</div>
              <div><strong>Ngày lập:</strong> {group.date || "N/A"}</div>
              <div><strong>Khách hàng / Đối tác:</strong> {group.partnerName || "N/A"}</div>
            </div>
          </div>

          {/* Semantic Comparison Sections */}
          {comparisons.length > 0 ? (
            <div className="semantic-comparisons-list" style={{ display: "flex", flexDirection: "column", gap: "1rem", marginBottom: "1.5rem" }}>
              {comparisons.map((comp, idx) => {
                const isExact = comp.status === "MATCHED_EXACT";
                const isZero = isZeroMoney(comp.variance);
                return (
                  <div
                    key={idx}
                    className="semantic-card"
                    style={{
                      border: "1px solid #cbd5e1",
                      borderRadius: "8px",
                      padding: "1rem",
                      background: isExact ? "#ffffff" : "#fff7ed",
                    }}
                  >
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.75rem", borderBottom: "1px solid #e2e8f0", paddingBottom: "0.5rem" }}>
                      <div>
                        <strong style={{ fontSize: "1rem", color: "#1e293b" }}>{comp.semanticName}</strong>
                        <span style={{ marginLeft: "0.5rem", color: "#64748b", fontSize: "0.875rem" }}>
                          ({comp.primarySourceName} ↔ {comp.secondarySourceName})
                        </span>
                      </div>
                      {getStatusBadge(comp.status)}
                    </div>

                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "1rem", marginBottom: "0.5rem" }}>
                      <div className="val-box val-source" style={{ background: "#f1f5f9", padding: "0.5rem 0.75rem", borderRadius: "4px" }}>
                        <div style={{ fontSize: "0.8rem", color: "#64748b" }}>Giá trị Nguồn chính (A):</div>
                        <div style={{ fontSize: "1.1rem", fontWeight: "bold" }}>{formatVND(comp.expectedAmount)}</div>
                      </div>
                      <div className="val-box val-target" style={{ background: "#f1f5f9", padding: "0.5rem 0.75rem", borderRadius: "4px" }}>
                        <div style={{ fontSize: "0.8rem", color: "#64748b" }}>Giá trị Đối chiếu (B):</div>
                        <div style={{ fontSize: "1.1rem", fontWeight: "bold" }}>{formatVND(comp.actualAmount)}</div>
                      </div>
                      <div className="val-box" style={{ background: isZero ? "#f1f5f9" : "#fee2e2", padding: "0.5rem 0.75rem", borderRadius: "4px" }}>
                        <div style={{ fontSize: "0.8rem", color: "#64748b" }}>Chênh lệch (A - B):</div>
                        <div style={{ fontSize: "1.1rem", fontWeight: "bold", color: isZero ? "#059669" : "#dc2626" }}>
                          {formatVND(comp.variance)}
                        </div>
                      </div>
                    </div>

                    {comp.discrepancies && comp.discrepancies.length > 0 && (
                      <div style={{ marginTop: "0.5rem", fontSize: "0.875rem", color: "#b91c1c" }}>
                        {comp.discrepancies.map((d, dIdx) => (
                          <div key={dIdx}>⚠️ {d.message}</div>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ) : (
            /* Fallback single summary bar if no semantic comparisons */
            <div className="inspector-summary-bar">
              <div className="summary-item">
                <span className="summary-lbl">Tổng tiền Nguồn chính (A):</span>
                <strong className="summary-val">{formatVND(group.totalSourceAmount)}</strong>
              </div>
              <div className="summary-item">
                <span className="summary-lbl">Tổng tiền Đối chiếu (B):</span>
                <strong className="summary-val">{formatVND(group.totalTargetAmount)}</strong>
              </div>
              <div className="summary-item">
                <span className="summary-lbl">Chênh lệch:</span>
                <strong className="summary-val">{formatVND(group.amountVariance)}</strong>
              </div>
            </div>
          )}

          {/* Group-level Discrepancies list */}
          {group.discrepancies && group.discrepancies.length > 0 && comparisons.length === 0 && (
            <div className="inspector-diff-section">
              <h4 className="section-subtitle">Danh sách các điểm sai lệch phát hiện được:</h4>
              <div className="diff-cards-list">
                {group.discrepancies.map((d, i) => (
                  <div key={i} className="diff-card">
                    <div className="diff-card-header">
                      <span className="diff-field-name">Trường dữ liệu: {d.fieldName}</span>
                      {d.amountDiff !== undefined && d.amountDiff !== null && (
                        <span className="diff-amount-tag">Lệch: {formatVND(d.amountDiff)}</span>
                      )}
                    </div>
                    <div className="diff-msg">{d.message}</div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Record references */}
          <div className="inspector-refs-section">
            <h4 className="section-subtitle">Bản ghi tham gia đối chiếu:</h4>
            <div className="refs-grid">
              <div className="ref-col">
                <span className="ref-col-title">Nguồn chính ({group.primarySourceRecordIds.length} bản ghi):</span>
                <ul className="ref-list">
                  {group.primarySourceRecordIds.length > 0 ? (
                    group.primarySourceRecordIds.map((id) => <li key={id}>{id}</li>)
                  ) : (
                    <li className="text-muted">(Không có bản ghi)</li>
                  )}
                </ul>
              </div>
              <div className="ref-col">
                <span className="ref-col-title">Nguồn đối chiếu ({group.targetSourceRecordIds.length} bản ghi):</span>
                <ul className="ref-list">
                  {group.targetSourceRecordIds.length > 0 ? (
                    group.targetSourceRecordIds.map((id) => <li key={id}>{id}</li>)
                  ) : (
                    <li className="text-muted">(Không có bản ghi)</li>
                  )}
                </ul>
              </div>
            </div>
          </div>
        </div>

        <div className="modal-footer">
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
};

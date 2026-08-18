import React from "react";
import type { MatchGroup, MatchStatus } from "../types/dataContract";

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

  const formatMoney = (val: number) => {
    return Math.round(val).toLocaleString("vi-VN") + " đ";
  };

  const getStatusBadge = (status: MatchStatus) => {
    switch (status) {
      case "MATCHED_EXACT":
        return <span className="status-badge badge-exact">✓ Khớp hoàn toàn</span>;
      case "MATCHED_WITH_TOLERANCE":
        return <span className="status-badge badge-tolerance">≈ Khớp có dung sai</span>;
      case "MATCHED_AGGREGATE":
        return <span className="status-badge badge-aggregate">∑ Khớp gộp (1-N)</span>;
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
        return <span className="status-badge badge-ambiguous">? Cần kiểm tra lại</span>;
      default:
        return <span className="status-badge">{status}</span>;
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-container modal-lg" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div>
            <div className="modal-tag-row">
              {getStatusBadge(group.status)}
              <span className="group-id-tag">Mã nhóm: {group.id}</span>
            </div>
            <h3 className="modal-title">Kiểm tra chi tiết đối chiếu & Bằng chứng kiểm toán</h3>
          </div>
          <button type="button" className="btn-close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="modal-body">
          {/* Summary financial comparison bar */}
          <div className="inspector-summary-bar">
            <div className="summary-item">
              <span className="summary-lbl">Tổng tiền Nguồn chính (A):</span>
              <strong className="summary-val">{formatMoney(group.totalSourceAmount)}</strong>
            </div>
            <div className="summary-item">
              <span className="summary-lbl">Tổng tiền Nguồn đối chiếu (B):</span>
              <strong className="summary-val">{formatMoney(group.totalTargetAmount)}</strong>
            </div>
            <div className="summary-item">
              <span className="summary-lbl">Chênh lệch tài chính:</span>
              <strong
                className={`summary-val ${
                  group.amountVariance === 0
                    ? "var-zero"
                    : group.amountVariance > 0
                    ? "var-pos"
                    : "var-neg"
                }`}
              >
                {formatMoney(group.amountVariance)}
              </strong>
            </div>
          </div>

          {/* Discrepancies list */}
          {group.discrepancies && group.discrepancies.length > 0 ? (
            <div className="inspector-diff-section">
              <h4 className="section-subtitle">Danh sách các điểm sai lệch phát hiện được:</h4>
              <div className="diff-cards-list">
                {group.discrepancies.map((d, i) => (
                  <div key={i} className="diff-card">
                    <div className="diff-card-header">
                      <span className="diff-field-name">Trường dữ liệu: {d.fieldName}</span>
                      {d.amountDiff !== undefined && d.amountDiff !== null && (
                        <span className="diff-amount-tag">
                          Lệch: {formatMoney(d.amountDiff)}
                        </span>
                      )}
                    </div>
                    <div className="diff-msg">{d.message}</div>
                    <div className="diff-values-row">
                      <div className="val-box val-source">
                        <span className="val-lbl">Giá trị Nguồn A:</span>
                        <span className="val-text">{d.sourceValue || "(Trống)"}</span>
                      </div>
                      <div className="val-box val-target">
                        <span className="val-lbl">Giá trị Nguồn B:</span>
                        <span className="val-text">{d.targetValue || "(Trống)"}</span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="inspector-exact-banner">
              ✓ Nhóm chứng từ này khớp chính xác 100% tất cả các trường dữ liệu và số tiền.
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

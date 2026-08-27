import { useEffect, useMemo, useState } from "react";

import type {
  DocumentIntegrityCase,
  DocumentIntegrityResult,
  DocumentSnapshot,
} from "../types/dataContract";
import { formatVND } from "../utils/money";
import {
  documentCheckLabel,
  documentFilterItems,
  documentStatusLabel,
  documentTotalLabel,
  matchesDocumentFilter,
  notCheckedReason,
  totalStatusLabel,
  type DocumentFilterId,
} from "./documentIntegrityView";

const valuePair = (left?: string, right?: string, money = false) => {
  const format = (value?: string) => value === undefined ? "—" : money ? formatVND(value) : value;
  return `${format(left)} ↔ ${format(right)}`;
};

type SnapshotKind = "INVOICE" | "SALES_REGISTER" | "LEDGER_511";

function snapshotFields(record: DocumentSnapshot | undefined, kind: SnapshotKind) {
  const labels = kind === "INVOICE"
    ? ["Tiền chưa thuế", "VAT", "Tổng thanh toán"]
    : kind === "SALES_REGISTER"
      ? ["Tiền (doanh thu)", "Thuế", "Phải thu"]
      : ["Phát sinh Có (TK511)", "VAT (không thuộc TK511)", "Phải thu (không thuộc TK511)"];
  return [
    ["Nguồn", record?.provenance.sourceName || "—"],
    ["File / sheet / dòng", record
      ? `${record.provenance.filePath} / ${record.provenance.sheetName} / ${record.provenance.sourceRow}`
      : "—"],
    ["Ngày", record?.date || "—"],
    ["Số chứng từ", record?.invoiceNumber || "—"],
    [labels[0], record?.pretaxAmount ? formatVND(record.pretaxAmount) : "—"],
    [labels[1], record?.vatAmount ? formatVND(record.vatAmount) : "—"],
    [labels[2], record?.totalAmount ? formatVND(record.totalAmount) : "—"],
  ];
}

function DocumentDetail({
  document,
  ledger511Checked,
}: {
  document: DocumentIntegrityCase;
  ledger511Checked: boolean;
}) {
  return (
    <aside className="document-detail" aria-label="Chi tiết đối chiếu chứng từ">
      <div className="section-title-row">
        <h3>Chi tiết chứng từ</h3>
        <span className={`document-status status-${document.status.toLowerCase()}`}>
          {document.status === "FULLY_MATCHED" ? "Khớp hoàn toàn" : "Cần rà soát"}
        </span>
      </div>
      <div className="document-side-by-side">
        {[
          ["Hóa đơn Thuế", document.invoice, "INVOICE"],
          ["Bảng kê bán hàng (Tiền / Thuế / Phải thu)", document.salesRegister, "SALES_REGISTER"],
          ["Sổ cái TK511 (nguồn riêng)", document.ledger511, "LEDGER_511"],
        ].map(([title, record, kind]) => (
          <section key={String(title)}>
            <h4>{String(title)}</h4>
            <dl>
              {snapshotFields(record as DocumentSnapshot | undefined, kind as SnapshotKind).map(([label, value]) => (
                <div key={label}>
                  <dt>{label}</dt>
                  <dd>{value}</dd>
                </div>
              ))}
            </dl>
          </section>
        ))}
      </div>
      <div className="document-field-checks">
        {document.fieldChecks.map((check, index) => (
          <div className={`field-check check-${check.status.toLowerCase()}`} key={`${check.scope}:${check.field}:${index}`}>
            <span aria-label={check.status === "MATCH" ? "Khớp" : check.status === "NOT_CHECKED" ? "Chưa đối chiếu" : "Không khớp"}>
              {check.status === "MATCH" ? "✓" : check.status === "NOT_CHECKED" ? "—" : "✗"}
            </span>
            <strong>{documentCheckLabel(check)}</strong>
            <small>
              {check.status === "NOT_CHECKED"
                ? notCheckedReason(check, ledger511Checked)
                : `${check.expectedValue || "—"} ↔ ${check.actualValue || "—"}`}
            </small>
          </div>
        ))}
      </div>
      {document.errors.length > 0 && (
        <div className="document-error-codes" role="alert">
          <strong>Toàn bộ lỗi</strong>
          <ul>
            {document.errors.map((error, index) => (
              <li key={`${error.code}:${index}`}>
                <code>{error.code}</code> — {error.message}
              </li>
            ))}
          </ul>
        </div>
      )}
    </aside>
  );
}

export function DocumentIntegrityPanel({ result }: { result: DocumentIntegrityResult }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [activeFilter, setActiveFilter] = useState<DocumentFilterId>("ALL");
  useEffect(() => {
    setSelectedId(null);
    setActiveFilter("ALL");
  }, [result]);
  const filteredDocuments = useMemo(
    () => result.documents.filter((document) => matchesDocumentFilter(document, activeFilter)),
    [activeFilter, result.documents]
  );
  const selected = useMemo(
    () => filteredDocuments.find((document) => document.id === selectedId),
    [filteredDocuments, selectedId]
  );
  const summary = result.summary;
  const ledger511Checked = result.ledger511Checked ?? (summary.ledger511Records > 0);
  const summaryItems = useMemo(() => documentFilterItems(result), [result]);
  const activeFilterLabel = activeFilter === "ALL"
    ? "Tất cả chứng từ"
    : summaryItems.find((item) => item.id === activeFilter)?.label || "Chứng từ đã lọc";

  const selectFilter = (filterId: DocumentFilterId) => {
    setSelectedId(null);
    setActiveFilter((current) => current === filterId ? "ALL" : filterId);
  };

  return (
    <section className="document-integrity-panel" aria-label="Kiểm soát chi tiết chứng từ">
      <div className="document-summary-grid">
        {summaryItems.map((item) => (
          <button
            type="button"
            className="document-summary-filter"
            key={item.id}
            aria-label={`Lọc ${item.label}: ${item.count} chứng từ`}
            aria-pressed={activeFilter === item.id}
            onClick={() => selectFilter(item.id)}
          >
            <span>{item.label}</span>
            <strong>{item.count}</strong>
          </button>
        ))}
      </div>
      <div className="document-total-note" role="status">
        Tổng Thuế ↔ BK: {result.totalsEqual ? "bằng nhau" : "có chênh lệch hoặc chưa đủ dữ liệu"} ·
        {ledger511Checked
          ? ` Chứng từ ba nguồn: ${result.documentsPass ? "đạt" : "không đạt"}.`
          : " Thuế ↔ BK đã được đối chiếu; chưa có TK511 nên kết quả tổng thể CHƯA ĐỦ BẰNG CHỨNG."}
        {result.totalsEqual && !result.documentsPass && " Tổng bằng nhau không thay thế kiểm tra từng chứng từ."}
      </div>
      <section className="document-totals-section" aria-labelledby="document-totals-title">
        <div className="document-totals-heading">
          <div>
            <h4 id="document-totals-title">Tổng cộng trong kỳ đã chọn</h4>
            <p>Tính lại từ các dòng chi tiết hợp lệ trong kỳ; không cộng dòng “Tổng cộng” sẵn có trong workbook và không thay đổi khi lọc bảng.</p>
          </div>
        </div>
        <div className="document-totals-wrap">
          <table className="document-totals-table">
            <thead>
              <tr>
                <th>Chỉ tiêu</th>
                <th>Hóa đơn Thuế</th>
                <th>Bảng kê bán hàng</th>
                <th>Chênh lệch Thuế − BK</th>
                <th>Trạng thái</th>
              </tr>
            </thead>
            <tbody>
              {(result.totals || []).map((total) => (
                <tr key={total.field}>
                  <th scope="row">{documentTotalLabel[total.field as "PRETAX" | "VAT" | "TOTAL"]}</th>
                  <td>{formatVND(total.invoiceTotal)}</td>
                  <td>{formatVND(total.salesRegisterTotal)}</td>
                  <td>{formatVND(total.variance)}</td>
                  <td>
                    <span className={`document-total-status is-${total.status.toLowerCase()}`}>
                      {totalStatusLabel[total.status]}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="document-ledger-total" aria-label="Tổng đối chiếu Sổ cái TK511">
          <strong>Doanh thu Thuế ↔ Sổ cái TK511:</strong>
          <span>Chưa thuế Thuế {formatVND(result.ledger511Revenue.invoiceTotal)}</span>
          <span>Phát sinh Có TK511 {formatVND(result.ledger511Revenue.ledgerTotal)}</span>
          <span>Chênh lệch {formatVND(result.ledger511Revenue.variance)}</span>
          <span className={`document-total-status is-${result.ledger511Revenue.status.toLowerCase()}`}>
            {ledger511Checked ? totalStatusLabel[result.ledger511Revenue.status] : "Chưa tải Sổ cái TK511"}
          </span>
        </div>
        <p className="document-totals-warning">
          Tổng bằng nhau chỉ là kiểm tra cộng dọc; kết luận chứng từ vẫn phụ thuộc đối chiếu từng dòng và bằng chứng TK511.
        </p>
      </section>
      <div className="document-source-scope-note">
        <strong>Phạm vi đang đối chiếu:</strong> Tiền chưa thuế trên hóa đơn Thuế ↔ cột Tiền của BK
        (doanh thu); VAT Thuế ↔ cột Thuế BK; Tổng thanh toán ↔ cột Phải thu BK.
        {!ledger511Checked && " BK chứng minh bảng kê bán hàng, không thay thế file Sổ cái TK511; muốn xác nhận đã ghi sổ phải tải thêm nguồn TK511."}
      </div>
      <div className="document-filter-status" role="status" aria-label="Bộ lọc chứng từ">
        <span><strong>{activeFilterLabel}:</strong> hiển thị {filteredDocuments.length}/{result.documents.length} chứng từ.</span>
        {activeFilter !== "ALL" && (
          <button type="button" onClick={() => selectFilter("ALL")}>Hiển thị tất cả</button>
        )}
      </div>
      <div className="document-table-wrap">
        <table className="document-table">
          <thead>
            <tr>
              <th>Ngày Thuế</th>
              <th>Ngày BK</th>
              <th>Số HĐ / Số CT</th>
              <th>Doanh thu (Chưa thuế Thuế ↔ Tiền BK)</th>
              <th>VAT (Thuế ↔ BK)</th>
              <th>Phải thu (Tổng thanh toán ↔ BK)</th>
              <th>Trạng thái</th>
            </tr>
          </thead>
          <tbody>
            {filteredDocuments.map((document) => (
              <tr key={document.id} className={selectedId === document.id ? "is-selected" : ""}>
                <td>{document.invoice?.date || "—"}</td>
                <td>{document.salesRegister?.date || "—"}</td>
                <td>
                  <button type="button" className="document-open-button" onClick={() => setSelectedId(document.id)}>
                    {valuePair(document.invoice?.invoiceNumber, document.salesRegister?.invoiceNumber)}
                  </button>
                </td>
                <td>{valuePair(document.invoice?.pretaxAmount, document.salesRegister?.pretaxAmount, true)}</td>
                <td>{valuePair(document.invoice?.vatAmount, document.salesRegister?.vatAmount, true)}</td>
                <td>{valuePair(document.invoice?.totalAmount, document.salesRegister?.totalAmount, true)}</td>
                <td>{documentStatusLabel(document, ledger511Checked)}</td>
              </tr>
            ))}
            {filteredDocuments.length === 0 && (
              <tr>
                <td colSpan={7} className="document-filter-empty">Không có chứng từ thuộc nhóm này.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      {selected && <DocumentDetail document={selected} ledger511Checked={ledger511Checked} />}
    </section>
  );
}

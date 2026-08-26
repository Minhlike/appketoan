import { useMemo, useState } from "react";

import type {
  DocumentField,
  DocumentIntegrityCase,
  DocumentIntegrityResult,
  DocumentSnapshot,
} from "../types/dataContract";
import { formatVND } from "../utils/money";

const fieldLabel: Record<DocumentField, string> = {
  DATE: "Ngày",
  INVOICE_NUMBER: "Số HĐ / Số CT",
  PRETAX: "Chưa thuế",
  VAT: "VAT",
  TOTAL: "Phải thu",
};

const valuePair = (left?: string, right?: string, money = false) => {
  const format = (value?: string) => value === undefined ? "—" : money ? formatVND(value) : value;
  return `${format(left)} ↔ ${format(right)}`;
};

function snapshotFields(record?: DocumentSnapshot) {
  return [
    ["Nguồn", record?.provenance.sourceName || "—"],
    ["File / sheet / dòng", record
      ? `${record.provenance.filePath} / ${record.provenance.sheetName} / ${record.provenance.sourceRow}`
      : "—"],
    ["Ngày", record?.date || "—"],
    ["Số chứng từ", record?.invoiceNumber || "—"],
    ["Chưa thuế", record?.pretaxAmount ? formatVND(record.pretaxAmount) : "—"],
    ["VAT", record?.vatAmount ? formatVND(record.vatAmount) : "—"],
    ["Phải thu", record?.totalAmount ? formatVND(record.totalAmount) : "—"],
  ];
}

function DocumentDetail({ document }: { document: DocumentIntegrityCase }) {
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
          ["Hóa đơn Thuế", document.invoice],
          ["Bảng kê bán hàng", document.salesRegister],
          ["TK511", document.ledger511],
        ].map(([title, record]) => (
          <section key={String(title)}>
            <h4>{String(title)}</h4>
            <dl>
              {snapshotFields(record as DocumentSnapshot | undefined).map(([label, value]) => (
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
            <strong>{check.scope === "INVOICE_TO_SALES_REGISTER" ? "Thuế ↔ BK" : "Thuế ↔ TK511"}: {fieldLabel[check.field]}</strong>
            <small>
              {check.status === "NOT_CHECKED"
                ? "CHƯA ĐỐI CHIẾU"
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
  const selected = useMemo(
    () => result.documents.find((document) => document.id === selectedId),
    [result.documents, selectedId]
  );
  const summary = result.summary;
  const summaryItems = [
    ["Khớp hoàn toàn", summary.fullyMatched],
    ["Sai ngày", summary.dateMismatch],
    ["Sai số HĐ", summary.invoiceNumberMismatch],
    ["Sai tiền", summary.pretaxMismatch],
    ["Sai VAT", summary.vatMismatch],
    ["Sai phải thu", summary.totalMismatch],
    ["Thiếu BK", summary.missingInBk],
    ["Thừa BK", summary.extraInBk],
    ["Trùng Số ct", summary.duplicateInvoiceNumber],
    ["Mơ hồ", summary.ambiguousMatch],
  ] as const;

  return (
    <section className="document-integrity-panel" aria-label="Kiểm soát chi tiết chứng từ">
      <div className="document-summary-grid">
        {summaryItems.map(([label, value]) => (
          <div key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
      <div className="document-total-note" role="status">
        Tổng Thuế ↔ BK: {result.totalsEqual ? "bằng nhau" : "có chênh lệch hoặc chưa đủ dữ liệu"} ·
        Chứng từ: {result.documentsPass ? "đạt" : "không đạt"}.
        {result.totalsEqual && !result.documentsPass && " Tổng bằng nhau không thay thế kiểm tra từng chứng từ."}
      </div>
      <div className="document-table-wrap">
        <table className="document-table">
          <thead>
            <tr>
              <th>Ngày Thuế</th>
              <th>Ngày BK</th>
              <th>Số HĐ / Số CT</th>
              <th>Chưa thuế</th>
              <th>VAT</th>
              <th>Phải thu</th>
              <th>Trạng thái</th>
            </tr>
          </thead>
          <tbody>
            {result.documents.map((document) => (
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
                <td>{document.status === "FULLY_MATCHED" ? "Khớp hoàn toàn" : document.errors.map((error) => error.code).join(", ") || "Cần rà soát"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {selected && <DocumentDetail document={selected} />}
    </section>
  );
}

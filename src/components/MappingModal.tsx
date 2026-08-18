import React, { useState } from "react";
import type { DataSource, ExcelFileMetadata, ColumnMapping, DataSourceKind } from "../types/dataContract";

interface MappingModalProps {
  source: DataSource;
  fileMetadata: ExcelFileMetadata;
  isOpen: boolean;
  onClose: () => void;
  onSave: (updatedSource: DataSource) => void;
}

export const MappingModal: React.FC<MappingModalProps> = ({
  source,
  fileMetadata,
  isOpen,
  onClose,
  onSave,
}) => {
  if (!isOpen) return null;

  const currentSheet =
    fileMetadata.sheets.find((s) => s.name === source.sheetName) ||
    fileMetadata.sheets[0];
  const availableColumns = currentSheet ? currentSheet.columns : [];

  const [mapping, setMapping] = useState<ColumnMapping>({ ...source.columnMapping });
  const [sourceKind, setSourceKind] = useState<DataSourceKind>(source.kind);
  const [sourceName, setSourceName] = useState<string>(source.name);
  const [headerRow, setHeaderRow] = useState<number>(source.headerRow);
  const [dataStartRow, setDataStartRow] = useState<number>(source.dataStartRow);

  const handleFieldChange = (field: keyof ColumnMapping, val: string) => {
    setMapping((prev) => ({
      ...prev,
      [field]: val === "__none__" ? undefined : val,
    }));
  };

  const handleSave = () => {
    onSave({
      ...source,
      name: sourceName,
      kind: sourceKind,
      headerRow,
      dataStartRow,
      columnMapping: mapping,
    });
    onClose();
  };

  const mappingFields: Array<{
    key: keyof ColumnMapping;
    label: string;
    description: string;
    required?: boolean;
  }> = [
    { key: "docNoColumn", label: "Số hóa đơn / Số chứng từ", description: "Số chứng từ để đối chiếu (Số ct / Số HĐ)", required: true },
    { key: "docCodeColumn", label: "Mã chứng từ", description: "Loại / Mã CT (ví dụ: HĐ, PKT, PC)" },
    { key: "seriesColumn", label: "Ký hiệu hóa đơn", description: "Ví dụ: 1C26TAA, C24TBB" },
    { key: "templateCodeColumn", label: "Ký hiệu mẫu số", description: "Ví dụ: 1, 2, 01GTKT" },
    { key: "dateColumn", label: "Ngày lập / Ngày chứng từ", description: "Định dạng DD/MM/YYYY hoặc ngày Excel" },
    { key: "buyerTaxIdColumn", label: "MST người mua", description: "Mã số thuế bên mua / đối tác" },
    { key: "sellerTaxIdColumn", label: "MST người bán", description: "Mã số thuế đơn vị phát hành" },
    { key: "partnerTaxIdColumn", label: "Mã số thuế đối tác", description: "MST đối tác chung" },
    { key: "partnerNameColumn", label: "Tên khách hàng / Đơn vị", description: "Tên đối tác hoặc người mua" },
    { key: "pretaxAmountColumn", label: "Doanh thu chưa thuế", description: "Tiền hàng trước thuế" },
    { key: "vatAmountColumn", label: "Tiền thuế GTGT", description: "Số tiền thuế GTGT" },
    { key: "discountAmountColumn", label: "Tiền chiết khấu thương mại", description: "Số tiền chiết khấu (nếu có)" },
    { key: "feeAmountColumn", label: "Tiền phí / Lệ phí", description: "Số tiền phí (nếu có)" },
    { key: "totalAmountColumn", label: "Tổng tiền thanh toán", description: "Tổng tiền sau thuế thanh toán" },
    { key: "creditAmountColumn", label: "Phát sinh Có", description: "Dành cho sổ cái TK 511, TK 3331" },
    { key: "debitAmountColumn", label: "Phát sinh Nợ", description: "Dành cho sổ cái TK 131, TK 133" },
    { key: "vatRateColumn", label: "Thuế suất", description: "Ví dụ: 8%, 10%, 0%" },
    { key: "debitAccountColumn", label: "Tài khoản Nợ", description: "Ví dụ: 131, 111, 112" },
    { key: "creditAccountColumn", label: "Tài khoản Có", description: "Ví dụ: 5111, 33311" },
    { key: "voucherNoColumn", label: "Số phiếu / Số CT sổ cái", description: "Ví dụ: PKT-00101" },
    { key: "descriptionColumn", label: "Diễn giải / Nội dung", description: "Nội dung giao dịch kế toán" },
    { key: "bankAccountColumn", label: "Số tài khoản ngân hàng", description: "Dành cho sao kê ngân hàng" },
  ];

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-container" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div>
            <h3 className="modal-title">⚙️ Cấu hình loại nguồn & cột dữ liệu: {source.name}</h3>
            <p className="modal-subtitle">
              Sheet: <strong>{source.sheetName}</strong> • File: {fileMetadata.fileName}
            </p>
          </div>
          <button type="button" className="btn-close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="modal-body">
          {/* Source classification & row config */}
          <div className="mapping-row-config" style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: "1rem", marginBottom: "1.5rem" }}>
            <div className="form-group">
              <label className="field-label">Tên nguồn dữ liệu:</label>
              <input
                type="text"
                className="input-control"
                value={sourceName}
                onChange={(e) => setSourceName(e.target.value)}
              />
            </div>
            <div className="form-group">
              <label className="field-label">Loại nguồn đối chiếu:</label>
              <select
                className="select-control"
                value={sourceKind}
                onChange={(e) => setSourceKind(e.target.value as DataSourceKind)}
              >
                <option value="e_invoice">Hóa đơn điện tử</option>
                <option value="ledger_511">TK 511 - Doanh thu</option>
                <option value="ledger_3331">TK 3331 - Thuế GTGT</option>
                <option value="ledger_131">TK 131 - Công nợ (Phải thu)</option>
                <option value="ledger_133">TK 133 - Thuế đầu vào</option>
                <option value="bank_statement">Sao kê ngân hàng</option>
                <option value="cash_book">Sổ quỹ tiền mặt</option>
                <option value="branch_ledger">Sổ chi nhánh</option>
                <option value="custom">Khác / Tùy chỉnh</option>
              </select>
            </div>
            <div className="form-group">
              <label className="field-label">Dòng Tiêu đề (Header):</label>
              <input
                type="number"
                min="1"
                className="input-control"
                value={headerRow}
                onChange={(e) => setHeaderRow(parseInt(e.target.value, 10) || 1)}
              />
            </div>
            <div className="form-group">
              <label className="field-label">Dòng Dữ liệu bắt đầu:</label>
              <input
                type="number"
                min="1"
                className="input-control"
                value={dataStartRow}
                onChange={(e) => setDataStartRow(parseInt(e.target.value, 10) || 2)}
              />
            </div>
          </div>

          <div className="mapping-fields-grid">
            {mappingFields.map((f) => {
              const currentValue = mapping[f.key] || "__none__";
              return (
                <div key={f.key} className="mapping-field-card">
                  <div className="mapping-label-row">
                    <span className="field-label">
                      {f.label} {f.required && <span className="req-star">*</span>}
                    </span>
                    <span className="field-hint">{f.description}</span>
                  </div>
                  <select
                    className="select-control"
                    value={currentValue}
                    onChange={(e) => handleFieldChange(f.key, e.target.value)}
                  >
                    <option value="__none__">-- Chưa ánh xạ (Bỏ qua) --</option>
                    {availableColumns.map((col, idx) => (
                      <option key={idx} value={col}>
                        {col}
                      </option>
                    ))}
                  </select>
                </div>
              );
            })}
          </div>
        </div>

        <div className="modal-footer">
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            Hủy bỏ
          </button>
          <button type="button" className="btn btn-primary" onClick={handleSave}>
            Lưu cấu hình nguồn & cột
          </button>
        </div>
      </div>
    </div>
  );
};

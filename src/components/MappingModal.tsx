import React, { useState } from "react";
import type { DataSource, ExcelFileMetadata, ColumnMapping } from "../types/dataContract";

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
    { key: "docNoColumn", label: "Số hóa đơn / Số chứng từ", description: "Cột chứa số HĐ hoặc mã chứng từ", required: true },
    { key: "seriesColumn", label: "Ký hiệu hóa đơn", description: "Ví dụ: 1C26TAA, C24TBB" },
    { key: "dateColumn", label: "Ngày lập / Ngày chứng từ", description: "Định dạng DD/MM/YYYY hoặc ngày Excel" },
    { key: "partnerTaxIdColumn", label: "Mã số thuế (MST)", description: "MST người mua hoặc đối tác" },
    { key: "partnerNameColumn", label: "Tên khách hàng / Đơn vị", description: "Tên đối tác hoặc người mua" },
    { key: "pretaxAmountColumn", label: "Doanh thu chưa thuế", description: "Tiền hàng trước thuế" },
    { key: "vatAmountColumn", label: "Tiền thuế GTGT", description: "Số tiền thuế GTGT" },
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
            <h3 className="modal-title">⚙️ Cấu hình cột dữ liệu: {source.name}</h3>
            <p className="modal-subtitle">
              Sheet: <strong>{source.sheetName}</strong> • File: {fileMetadata.fileName}
            </p>
          </div>
          <button type="button" className="btn-close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="modal-body">
          {/* Row config */}
          <div className="mapping-row-config">
            <div className="form-group">
              <label className="field-label">Dòng Tiêu đề (Header Row):</label>
              <input
                type="number"
                min="1"
                className="input-control"
                value={headerRow}
                onChange={(e) => setHeaderRow(parseInt(e.target.value, 10) || 1)}
              />
            </div>
            <div className="form-group">
              <label className="field-label">Dòng Bắt đầu Dữ liệu:</label>
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
            Lưu cấu hình cột
          </button>
        </div>
      </div>
    </div>
  );
};

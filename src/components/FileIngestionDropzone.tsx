import React, { useRef, useState } from "react";
import type { DataSource, ExcelFileMetadata, DataSourceKind, SourceRole } from "../types/dataContract";
import { inspectExcelBytes } from "../services/api";

interface IngestedSourceItem {
  id: string;
  source: DataSource;
  fileMetadata: ExcelFileMetadata;
  rawBytes?: Uint8Array;
}

interface FileIngestionDropzoneProps {
  sources: IngestedSourceItem[];
  onAddSource: (item: IngestedSourceItem) => void;
  onRemoveSource: (id: string) => void;
  onUpdateSheet: (id: string, newSheetName: string) => void;
  onUpdateKind?: (id: string, newKind: DataSourceKind) => void;
  onUpdateRole?: (id: string, newRole: SourceRole) => void;
  onOpenMapping: (item: IngestedSourceItem) => void;
  disabled?: boolean;
  advancedMode?: boolean;
}

const sourceKindLabel = (kind: DataSourceKind): string =>
  ({
    e_invoice: "Hóa đơn điện tử",
    ledger_511: "Sổ doanh thu (TK 511)",
    ledger_3331: "Sổ thuế GTGT (TK 3331)",
    ledger_133: "Sổ thuế đầu vào (TK 133)",
    ledger_131: "Sổ công nợ phải thu (TK 131)",
    ledger_112: "Sổ tiền gửi ngân hàng (TK 112)",
    partner_master: "Danh mục khách hàng / nhà cung cấp",
    sales_register: "Bảng kê bán hàng",
    sales_analysis_report: "Báo cáo phân tích bán hàng",
    bank_statement: "Sao kê ngân hàng",
    cash_book: "Sổ quỹ tiền mặt",
    branch_ledger: "Sổ chi nhánh",
    custom: "Chưa xác định",
  })[kind];

export const FileIngestionDropzone: React.FC<FileIngestionDropzoneProps> = ({
  sources,
  onAddSource,
  onRemoveSource,
  onUpdateSheet,
  onUpdateKind,
  onUpdateRole,
  onOpenMapping,
  disabled = false,
  advancedMode = false,
}) => {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [loading, setLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const processFile = async (file: File) => {
    try {
      setLoading(true);
      setErrorMessage(null);
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);

      const metadata = await inspectExcelBytes(bytes, file.name);
      if (!metadata.sheets || metadata.sheets.length === 0) {
        throw new Error(`File ${file.name} không chứa sheet dữ liệu nào.`);
      }

      const defaultSheet = metadata.sheets[0];
      const sourceId = `src_${Date.now()}_${Math.random().toString(36).substr(2, 4)}`;

      // Default role: first source is Primary, subsequent are RequiredSecondary
      const defaultRole: SourceRole = sources.length === 0 ? "PRIMARY" : "REQUIRED_SECONDARY";

      const dataSource: DataSource = {
        id: sourceId,
        name: file.name.replace(/\.[^/.]+$/, ""),
        filePath: file.name,
        sheetName: defaultSheet.name,
        kind: defaultSheet.suggestedKind,
        role: defaultRole,
        headerRow: defaultSheet.detectedHeaderRow,
        dataStartRow: defaultSheet.detectedDataStartRow,
        columnMapping: defaultSheet.suggestedMapping,
      };

      onAddSource({
        id: sourceId,
        source: dataSource,
        fileMetadata: metadata,
        rawBytes: bytes,
      });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Không thể đọc file Excel: ${msg}`);
    } finally {
      setLoading(false);
    }
  };

  const processFiles = async (fileList: File[]) => {
    for (const file of fileList) {
      await processFile(file);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      const fileList = Array.from(e.target.files);
      void processFiles(fileList);
      e.target.value = "";
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    if (!disabled) setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    if (disabled) return;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const validFiles: File[] = [];
      Array.from(e.dataTransfer.files).forEach((file) => {
        if (file.name.endsWith(".xlsx") || file.name.endsWith(".xls") || file.name.endsWith(".xlsb")) {
          validFiles.push(file);
        } else {
          setErrorMessage("Vui lòng chỉ tải lên file định dạng Excel (.xlsx, .xls, .xlsb)");
        }
      });
      if (validFiles.length > 0) {
        void processFiles(validFiles);
      }
    }
  };

  return (
    <section className="ingestion-section">
      <div className="section-title-row">
        <h2 className="section-title">Danh sách nguồn dữ liệu Excel</h2>
        <span className="source-count-badge">{sources.length} nguồn</span>
      </div>

      {errorMessage && (
        <div className="alert-banner alert-error" role="alert">
          <span>⚠️ {errorMessage}</span>
          <button type="button" className="btn-close" onClick={() => setErrorMessage(null)}>
            ✕
          </button>
        </div>
      )}

      {/* Dropzone area */}
      <div
        className={`dropzone-container ${isDragging ? "dragging" : ""} ${disabled ? "disabled" : ""}`}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={() => {
          if (!disabled && fileInputRef.current) fileInputRef.current.click();
        }}
      >
        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept=".xlsx,.xls,.xlsb"
          style={{ display: "none" }}
          onChange={handleFileChange}
          disabled={disabled}
        />

        <div className="dropzone-content">
          <div className="dropzone-icon">📥</div>
          <div className="dropzone-text">
            <strong>Kéo thả các file Excel vào đây</strong> hoặc bấm để chọn từ máy tính
          </div>
          <div className="dropzone-hint">
            Hỗ trợ .xlsx, .xls, .xlsb • Hỗ trợ đối chiếu đồng thời nhiều file/sheet không giới hạn
          </div>
        </div>
      </div>

      {loading && <div className="loading-bar">Đang phân tích cấu trúc file Excel...</div>}

      {/* Ingested Sources Cards */}
      {sources.length > 0 && (
        <div className="sources-grid">
          {sources.map((item, index) => {
            const currentSheetMeta =
              item.fileMetadata.sheets.find((s) => s.name === item.source.sheetName) ||
              item.fileMetadata.sheets[0];
            const confidencePercent = Math.round((currentSheetMeta?.confidenceScore || 0) * 100);
            const role = item.source.role || (index === 0 ? "PRIMARY" : "REQUIRED_SECONDARY");

            return (
              <div key={item.id} className="source-card">
                <div className="source-card-header">
                  <div className="source-index-badge">
                    Nguồn #{index + 1}
                  </div>
                  <div className="source-meta-info">
                    <span className="source-filename" title={item.fileMetadata.fileName}>
                      📄 {item.fileMetadata.fileName}
                    </span>
                    <span className="source-filesize">
                      ({(item.fileMetadata.fileSizeBytes / 1024).toFixed(1)} KB)
                    </span>
                  </div>
                  <button
                    type="button"
                    className="btn-remove-source"
                    title="Xóa nguồn này"
                    onClick={() => onRemoveSource(item.id)}
                    disabled={disabled}
                  >
                    ✕
                  </button>
                </div>

                <div className="source-card-body">
                  {advancedMode && <div className="source-field-row">
                    <label className="field-label">Tên phân loại:</label>
                    <input
                      type="text"
                      className="input-control"
                      value={item.source.name}
                      disabled={disabled}
                      onChange={(e) => {
                        item.source.name = e.target.value;
                      }}
                    />
                  </div>}

                  <div className="source-field-row">
                    <label className="field-label">Vai trò trong kịch bản:</label>
                    {advancedMode ? <select
                      className="select-control"
                      value={role}
                      disabled={disabled}
                      onChange={(e) => {
                        const newRole = e.target.value as SourceRole;
                        if (onUpdateRole) {
                          onUpdateRole(item.id, newRole);
                        } else {
                          item.source.role = newRole;
                        }
                      }}
                    >
                      <option value="PRIMARY">🔵 Nguồn chính (PRIMARY)</option>
                      <option value="REQUIRED_SECONDARY">🟠 Nguồn bắt buộc (REQUIRED)</option>
                      <option value="OPTIONAL_SECONDARY">⚪ Nguồn bổ trợ (OPTIONAL)</option>
                      <option value="REFERENCE_MASTER">🟣 Danh mục tham chiếu</option>
                    </select> : <strong>{sourceKindLabel(item.source.kind)}</strong>}
                  </div>

                  <div className="source-field-row">
                    <label className="field-label">Loại dữ liệu:</label>
                    <select
                      className="select-control"
                      value={item.source.kind}
                      disabled={disabled}
                      onChange={(e) => {
                        const newKind = e.target.value as DataSourceKind;
                        if (onUpdateKind) {
                          onUpdateKind(item.id, newKind);
                        } else {
                          item.source.kind = newKind;
                        }
                      }}
                    >
                      <option value="e_invoice">Hóa đơn điện tử</option>
                      <option value="ledger_511">TK 511 - Doanh thu</option>
                      <option value="ledger_3331">TK 3331 - Thuế GTGT</option>
                      <option value="ledger_131">TK 131 - Công nợ (Phải thu)</option>
                      <option value="ledger_112">TK 112 - Tiền gửi ngân hàng</option>
                      <option value="sales_register">Bảng kê bán hàng</option>
                      <option value="partner_master">Danh mục khách hàng / NCC</option>
                      <option value="sales_analysis_report">Báo cáo phân tích bán hàng</option>
                      <option value="ledger_133">TK 133 - Thuế đầu vào</option>
                      <option value="bank_statement">Sao kê ngân hàng</option>
                      <option value="cash_book">Sổ quỹ tiền mặt</option>
                      <option value="branch_ledger">Sổ chi nhánh</option>
                      <option value="custom">Khác / Tùy chỉnh</option>
                    </select>
                  </div>

                  <div className="source-field-row">
                    <label className="field-label">Sheet đối chiếu:</label>
                    <select
                      className="select-control"
                      value={item.source.sheetName}
                      disabled={disabled}
                      onChange={(e) => onUpdateSheet(item.id, e.target.value)}
                    >
                      {item.fileMetadata.sheets.map((s) => (
                        <option key={s.name} value={s.name}>
                          {s.name} ({s.totalRows} dòng, {s.totalCols} cột)
                        </option>
                      ))}
                    </select>
                  </div>

                  <div className="source-stats-row">
                    <span className="stat-pill">
                      📊 {currentSheetMeta?.totalRows || 0} dòng
                    </span>
                    <span
                      className={`confidence-pill ${
                        confidencePercent >= 80
                          ? "conf-high"
                          : confidencePercent >= 50
                          ? "conf-med"
                          : "conf-low"
                      }`}
                    >
                      {confidencePercent >= 80 ? "✓ Tự nhận diện (" : "⚠️ Cần kiểm tra ("}
                      {confidencePercent}%)
                    </span>
                  </div>
                </div>

                <div className="source-card-footer">
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary"
                    onClick={() => onOpenMapping(item)}
                    disabled={disabled}
                  >
                    ⚙️ Thiết lập nâng cao ({Object.values(item.source.columnMapping).filter(Boolean).length} cột đã gán)
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
};

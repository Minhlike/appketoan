import React, { useRef, useState } from "react";
import type { DataSource, ExcelFileMetadata } from "../types/dataContract";
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
  onOpenMapping: (item: IngestedSourceItem) => void;
  disabled?: boolean;
}

export const FileIngestionDropzone: React.FC<FileIngestionDropzoneProps> = ({
  sources,
  onAddSource,
  onRemoveSource,
  onUpdateSheet,
  onOpenMapping,
  disabled = false,
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

      const dataSource: DataSource = {
        id: sourceId,
        name: file.name.replace(/\.[^/.]+$/, ""),
        filePath: file.name,
        sheetName: defaultSheet.name,
        kind: defaultSheet.suggestedKind,
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

            return (
              <div key={item.id} className="source-card">
                <div className="source-card-header">
                  <div className="source-index-badge">Nguồn #{index + 1}</div>
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
                  <div className="source-field-row">
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
                      {confidencePercent >= 80 ? "✓ Tự map cột (" : "⚠️ Cần xem ("}
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
                    ⚙️ Cấu hình cột ({Object.values(item.source.columnMapping).filter(Boolean).length} cột đã gán)
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

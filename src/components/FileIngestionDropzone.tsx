import React, { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { DataSource, DataSourceKind, SourceRole } from "../types/dataContract";
import type { IngestedSourceItem } from "../types/auditWorkspace";
import {
  inspectExcelBytes,
  inspectExcelFile,
  isTauriRuntime,
  safeUserError,
} from "../services/api";

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

async function sha256(bytes: Uint8Array): Promise<string | undefined> {
  if (!globalThis.crypto?.subtle) return undefined;
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes as BufferSource);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

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

  const processFile = async (
    file: File,
    seenHashes: Set<string>,
    sourcePosition: number
  ) => {
    try {
      setLoading(true);
      setErrorMessage(null);
      if (file.name.startsWith("~$")) {
        setErrorMessage("Đã bỏ qua tệp tạm do Microsoft Office tạo.");
        return;
      }
      const buffer = await file.arrayBuffer();
      const bytes = new Uint8Array(buffer);
      const rawSha256 = await sha256(bytes);
      if (rawSha256 && seenHashes.has(rawSha256)) {
        setErrorMessage("Tệp này trùng hoàn toàn với nguồn đã nạp.");
        return;
      }

      const metadata = await inspectExcelBytes(bytes, file.name);
      if (!metadata.sheets || metadata.sheets.length === 0) {
        throw new Error(`File ${file.name} không chứa sheet dữ liệu nào.`);
      }

      const defaultSheet = metadata.sheets[0];
      const sourceId = `src_${Date.now()}_${Math.random().toString(36).substr(2, 4)}`;

      // Default role: first source is Primary, subsequent are RequiredSecondary
      const defaultRole: SourceRole =
        sourcePosition === 0 ? "PRIMARY" : "REQUIRED_SECONDARY";

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
        rawSha256,
      });
      if (rawSha256) seenHashes.add(rawSha256);
    } catch (err: unknown) {
      setErrorMessage(`Không thể đọc file Excel: ${safeUserError(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const processPath = async (filePath: string, sourcePosition: number) => {
    const fileName = filePath.split(/[\\/]/).pop() || filePath;
    try {
      setLoading(true);
      setErrorMessage(null);
      if (fileName.startsWith("~$")) {
        setErrorMessage("Đã bỏ qua tệp tạm do Microsoft Office tạo.");
        return;
      }
      if (!/\.(xlsx|xls|xlsb)$/i.test(fileName)) {
        setErrorMessage("Vui lòng chỉ nạp file Excel (.xlsx, .xls, .xlsb).");
        return;
      }
      const metadata = await inspectExcelFile(filePath);
      const defaultSheet = metadata.sheets[0];
      if (!defaultSheet) throw new Error("File không chứa sheet dữ liệu nào.");
      const sourceId = `src_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
      onAddSource({
        id: sourceId,
        source: {
          id: sourceId,
          name: fileName.replace(/\.[^/.]+$/, ""),
          filePath,
          sheetName: defaultSheet.name,
          kind: defaultSheet.suggestedKind,
          role: sourcePosition === 0 ? "PRIMARY" : "REQUIRED_SECONDARY",
          headerRow: defaultSheet.detectedHeaderRow,
          dataStartRow: defaultSheet.detectedDataStartRow,
          columnMapping: defaultSheet.suggestedMapping,
        },
        fileMetadata: metadata,
      });
    } catch (error: unknown) {
      setErrorMessage(`Không thể đọc file Excel: ${safeUserError(error)}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (disposed || disabled || event.payload.type !== "drop") return;
        const paths = event.payload.paths;
        void (async () => {
          for (const [index, path] of paths.entries()) {
            await processPath(path, sources.length + index);
          }
        })();
      })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [disabled, sources.length]);

  const processFiles = async (fileList: File[]) => {
    const seenHashes = new Set(
      sources.flatMap((source) => (source.rawSha256 ? [source.rawSha256] : []))
    );
    for (const [index, file] of fileList.entries()) {
      await processFile(file, seenHashes, sources.length + index);
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
    if (isTauriRuntime()) return;

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
                  {advancedMode && (
                    <div className="source-field-row">
                      <label className="field-label">Vai trò trong kịch bản:</label>
                      <select
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
                      </select>
                    </div>
                  )}

                  <div className="source-field-row">
                    <label className="field-label">
                      {advancedMode ? "Loại dữ liệu:" : "Loại dữ liệu nhận dạng:"}
                    </label>
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

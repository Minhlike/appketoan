import type {
  AccountingPeriod,
  AuditWorkspaceReport,
  ControlExecutionStatus,
  SourceCapability,
} from "../types/dataContract";
import type { IngestedSourceItem } from "../types/auditWorkspace";

interface Props {
  sources: IngestedSourceItem[];
  accountingPeriod: AccountingPeriod;
  report: AuditWorkspaceReport | null;
}

const capabilityLabel = (capability: SourceCapability) =>
  capability.kind === "LEDGER_ENTRY"
    ? `Sổ kế toán TK ${capability.account}`
    : {
        INVOICE: "Hóa đơn bán ra",
        BANK_TRANSACTION: "Sao kê ngân hàng",
        PARTNER_MASTER: "Danh mục đối tác",
        SALES_TRANSACTION: "Bảng kê bán hàng",
        ANALYTICAL_SALES_REPORT: "Báo cáo phân tích bán hàng",
      }[capability.kind];

const planLabel = (status: string) =>
  ({
    READY: "Sẵn sàng kiểm tra",
    MISSING_SOURCE: "Thiếu chứng từ / sổ cần thiết",
    NEEDS_MAPPING: "Cần xác định dữ liệu",
    NEEDS_REVIEW: "Cần người dùng rà soát",
    NOT_APPLICABLE: "Chưa thể áp dụng",
  })[status] || status;

const resultLabel = (status: ControlExecutionStatus) =>
  ({
    PASS: "Không phát hiện sai lệch theo kiểm tra này",
    NEEDS_REVIEW: "Có phát hiện cần rà soát",
    NOT_RUN: "Chưa chạy",
    FAILED: "Kiểm tra chưa hoàn tất",
    CANCELLED: "Đã dừng",
  })[status];

const resultSummary = (controlId: string, metrics: Record<string, number>) => {
  if (controlId === "BANK_LEDGER_RECONCILIATION") {
    return [
      `Khớp chắc chắn: ${metrics.strongAccepted || 0}`,
      `Gợi ý cần duyệt: ${metrics.suggestedReviewLinked || 0}`,
      `Mơ hồ cần duyệt: ${metrics.ambiguousReviewLinked || 0}`,
      `Chỉ có trên ngân hàng: ${metrics.trueBankOnly || 0}`,
      `Chỉ có trên sổ TK112: ${metrics.trueLedgerOnly || 0}`,
    ];
  }
  if (controlId === "PARTNER_IDENTITY") {
    return [`Đối tác đã kiểm tra: ${metrics.recordsChecked || 0}`, "Không tự ghép gần đúng"];
  }
  if (controlId === "SALES_ANALYSIS_CONTROL") {
    return [
      `Dòng nhóm: ${metrics.groupRecords || 0}`,
      `Dòng chi tiết: ${metrics.detailRecords || 0}`,
      "Dòng nhóm không cộng trùng vào tổng chi tiết",
    ];
  }
  return [
    `Khớp chính xác: ${metrics.exactMatches || 0}`,
    `Cần rà soát: ${metrics.needsReview || 0}`,
    `Thiếu ở nguồn đối chiếu: ${metrics.missingInTarget || 0}`,
    `Chỉ có ở nguồn đối chiếu: ${metrics.missingInSource || 0}`,
  ];
};

export function AuditWorkspaceDashboard({ sources, accountingPeriod, report }: Props) {
  const catalog = report?.sourceCatalog.sources || [];
  return (
    <section className="audit-workspace" aria-label="Bộ hồ sơ kế toán">
      <div className="section-title-row">
        <div>
          <h1 className="workspace-title">Bộ hồ sơ kế toán</h1>
          <p className="audit-workspace-subtitle">
            Nạp nguồn một lần, giới hạn đúng kỳ và chạy các kiểm tra độc lập.
          </p>
        </div>
        <span className="source-count-badge">
          {accountingPeriod.startDate && accountingPeriod.endDate
            ? `${accountingPeriod.startDate} → ${accountingPeriod.endDate}`
            : "Chưa chọn kỳ"}
        </span>
      </div>

      {sources.length === 0 ? (
        <div className="empty-state">
          <strong>Chưa có nguồn dữ liệu</strong>
          <p>Bắt đầu bằng cách kéo các file Excel của kỳ kế toán vào khu vực bên dưới.</p>
        </div>
      ) : (
        <div className="audit-source-list">
          {sources.map((item) => {
            const selectedSheet = item.fileMetadata.sheets.find(
              (sheet) => sheet.name === item.source.sheetName
            ) || item.fileMetadata.sheets[0];
            const source = catalog.find((entry) => entry.sourceId === item.id);
            const sourceError = report?.errors.find(
              (error) => error.scope === "SOURCE" && error.sourceId === item.id
            );
            const valid = source?.recordCount;
            const skipped = valid === undefined ? undefined : Math.max(0, selectedSheet.totalRows - valid);
            const confidence = Math.round((selectedSheet.confidenceScore || 0) * 100);
            return (
              <article className="audit-source-row" id={`source-${item.id}`} key={item.id}>
                <div className="source-summary">
                  <strong>{item.fileMetadata.fileName}</strong>
                  <span>Sheet: {item.source.sheetName}</span>
                  <span>
                    {valid === undefined
                      ? `${selectedSheet.totalRows} dòng trước khi kiểm tra`
                      : `${valid} bản ghi hợp lệ${skipped ? ` · ${skipped} dòng bỏ qua` : ""}`}
                  </span>
                </div>
                <div className="audit-capabilities">
                  {(source?.capabilities || []).length > 0
                    ? source?.capabilities.map((capability) => (
                        <span key={JSON.stringify(capability)}>{capabilityLabel(capability)}</span>
                      ))
                    : <span>{confidence >= 80 ? "Đang chờ lập kế hoạch" : "Cần xác định loại dữ liệu"}</span>}
                  <span className={confidence >= 80 ? "confidence-good" : "confidence-review"}>
                    {confidence >= 80 ? "Nhận dạng tốt" : "Có vẻ đúng, cần kiểm tra"} · {confidence}%
                  </span>
                </div>
                {source?.periodEvidence.earliestDate && source.periodEvidence.latestDate && (
                  <small>
                    Kỳ dữ liệu: {source.periodEvidence.earliestDate} → {source.periodEvidence.latestDate}
                  </small>
                )}
                {(source?.warnings.length || 0) > 0 && (
                  <small className="audit-warning">{source?.warnings.join(" ")}</small>
                )}
                {sourceError && (
                  <div className="source-recovery" role="alert">
                    <strong>{sourceError.safeUserMessage}</strong>
                    {sourceError.recommendedAction && (
                      <small>Hướng xử lý: {sourceError.recommendedAction}</small>
                    )}
                  </div>
                )}
              </article>
            );
          })}
        </div>
      )}

      {report && (
        <div className="control-plan-list">
          <div className="section-title-row">
            <h2 className="section-title">Kế hoạch kiểm tra</h2>
            <span className={`control-status status-${report.runStatus.toLowerCase()}`}>
              {report.runStatus === "PARTIAL" ? "Hoàn tất một phần" : "Đã lập kế hoạch"}
            </span>
          </div>
          {report.controlPlans.map((control) => {
            const result = report.controlResults.find(
              (candidate) => candidate.controlId === control.controlId
            );
            const sourceNames = control.sourceIds
              .map((id) => catalog.find((source) => source.sourceId === id)?.sourceName)
              .filter(Boolean)
              .join(", ");
            return (
              <article className="control-plan-row" id={`control-${control.controlId}`} key={control.controlId}>
                <div>
                  <strong>{control.title}</strong>
                  {sourceNames && <small>Nguồn sử dụng: {sourceNames}</small>}
                  {control.effectivePeriod && (
                    <small>
                      Kỳ kiểm tra: {control.effectivePeriod.startDate} → {control.effectivePeriod.endDate}
                    </small>
                  )}
                  {control.missingCapabilities.length > 0 && (
                    <small>Thiếu: {control.missingCapabilities.map(capabilityLabel).join(", ")}</small>
                  )}
                  {result && <small>{resultLabel(result.status)}</small>}
                  {result && result.findings.length > 0 && (
                    <small className="audit-warning">
                      {result.findings.length} phát hiện cần kiểm tra
                    </small>
                  )}
                  {result && Object.keys(result.summaryMetrics).length > 0 && (
                    <ul className="control-result-summary" aria-label="Tóm tắt kết quả">
                      {resultSummary(result.controlId, result.summaryMetrics).map((item) => (
                        <li key={item}>{item}</li>
                      ))}
                    </ul>
                  )}
                  {result?.limitations.includes("TK112_RUNNING_BALANCE_NOT_VERIFIED") && (
                    <small className="audit-warning">
                      Số dư chạy TK112: chưa đủ dữ liệu để xác minh.
                    </small>
                  )}
                </div>
                <span
                  className={`control-status status-${(result?.status || control.status).toLowerCase()}`}
                  aria-label={result ? resultLabel(result.status) : planLabel(control.status)}
                >
                  {result ? resultLabel(result.status) : planLabel(control.status)}
                </span>
              </article>
            );
          })}
          <div className="execution-metrics" aria-label="Thông tin thực thi">
            <span>Backend: {report.metrics.stages.totalBackendMs} ms</span>
            {report.metrics.ipcRoundTripOverheadMs !== undefined && (
              <span>IPC/vòng gọi: {report.metrics.ipcRoundTripOverheadMs} ms</span>
            )}
            {report.metrics.frontendRenderMs !== undefined && (
              <span>Hiển thị: {report.metrics.frontendRenderMs} ms</span>
            )}
            <span>Dùng lại cache: {report.metrics.cacheHits}</span>
            <span>Chuẩn hóa mới: {report.metrics.cacheMisses}</span>
          </div>
        </div>
      )}
    </section>
  );
}

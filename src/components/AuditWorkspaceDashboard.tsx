import type {
  AccountingPeriod,
  AuditWorkspaceReport,
  DataSource,
  ExcelFileMetadata,
  SourceCapability,
} from "../types/dataContract";

interface SourcePreview {
  source: DataSource;
  fileMetadata: ExcelFileMetadata;
}

interface AuditWorkspaceDashboardProps {
  sources: SourcePreview[];
  accountingPeriod: AccountingPeriod;
  report: AuditWorkspaceReport | null;
}

const capabilityLabel = (capability: SourceCapability) =>
  capability.kind === "LEDGER_ENTRY"
    ? `Sổ kế toán TK ${capability.account}`
    : {
        INVOICE: "Hóa đơn",
        BANK_TRANSACTION: "Giao dịch ngân hàng",
        PARTNER_MASTER: "Danh mục đối tác",
        SALES_TRANSACTION: "Bảng kê bán hàng",
        ANALYTICAL_SALES_REPORT: "Báo cáo phân tích bán hàng",
      }[capability.kind];

const statusLabel = (status: string) =>
  ({
    READY: "Sẵn sàng",
    MISSING_SOURCE: "Thiếu nguồn",
    NEEDS_MAPPING: "Cần xác định cột",
    NEEDS_REVIEW: "Cần rà soát",
    NOT_APPLICABLE: "Không áp dụng",
  }[status] || status);

export function AuditWorkspaceDashboard({
  sources,
  accountingPeriod,
  report,
}: AuditWorkspaceDashboardProps) {
  return (
    <section className="audit-workspace" aria-label="Bộ hồ sơ kế toán">
      <div className="section-title-row">
        <div>
          <h2 className="section-title">Bộ hồ sơ kế toán</h2>
          <p className="audit-workspace-subtitle">
            Nhận dạng nguồn, giới hạn đúng kỳ và lập kế hoạch kiểm tra tự động.
          </p>
        </div>
        <span className="source-count-badge">
          {accountingPeriod.startDate && accountingPeriod.endDate
            ? `${accountingPeriod.startDate} → ${accountingPeriod.endDate}`
            : "Chưa chọn kỳ"}
        </span>
      </div>

      <div className="audit-source-list">
        {(report?.sourceCatalog.sources || sources.map((item) => {
          const sheet = item.fileMetadata.sheets.find(
            (candidate) => candidate.name === item.source.sheetName
          );
          return {
            sourceId: item.source.id,
            sourceName: item.source.name,
            sourceKind: item.source.kind,
            capabilities: [] as SourceCapability[],
            recordCount: sheet?.totalRows || 0,
            mappingComplete: item.source.kind !== "custom",
            periodEvidence: {
              recordsInPeriod: 0,
              recordsOutsidePeriod: 0,
              missingOrUnparseableDates: 0,
            },
            warnings: [],
          };
        })).map((source) => (
          <div className="audit-source-row" key={source.sourceId}>
            <div>
              <strong>{source.sourceName}</strong>
              <span>{source.recordCount} record</span>
            </div>
            <div className="audit-capabilities">
              {source.capabilities.length > 0
                ? source.capabilities.map((capability) => (
                    <span key={JSON.stringify(capability)}>{capabilityLabel(capability)}</span>
                  ))
                : <span>Đang chờ lập kế hoạch</span>}
            </div>
            {source.warnings.length > 0 && (
              <small className="audit-warning">{source.warnings.join(" ")}</small>
            )}
          </div>
        ))}
      </div>

      {report && (
        <div className="control-plan-list">
          <h3>Kế hoạch kiểm tra</h3>
          {report.controlPlans.map((control) => {
            const result = report.controlResults.find(
              (candidate) => candidate.controlId === control.controlId
            );
            return (
              <article className="control-plan-row" key={control.controlId}>
                <div>
                  <strong>{control.title}</strong>
                  {control.effectivePeriod && (
                    <small>
                      Kỳ kiểm tra: {control.effectivePeriod.startDate} → {control.effectivePeriod.endDate}
                    </small>
                  )}
                  {control.missingCapabilities.length > 0 && (
                    <small>
                      Thiếu: {control.missingCapabilities.map(capabilityLabel).join(", ")}
                    </small>
                  )}
                  {result && result.findings.length > 0 && (
                    <small className="audit-warning">
                      {result.findings.length} phát hiện cần kiểm tra
                    </small>
                  )}
                </div>
                <span className={`control-status status-${control.status.toLowerCase()}`}>
                  {statusLabel(control.status)}
                </span>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}

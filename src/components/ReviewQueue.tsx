import type {
  AuditExecutionError,
  AuditWorkspaceReport,
  ControlFinding,
  ControlPlan,
} from "../types/dataContract";

interface ReviewItem {
  id: string;
  severity: "HIGH" | "MEDIUM" | "LOW";
  title: string;
  reason: string;
  action?: string;
  targetId?: string;
}

function severity(value?: string): ReviewItem["severity"] {
  return value === "HIGH" ? "HIGH" : value === "LOW" ? "LOW" : "MEDIUM";
}

function errorItem(error: AuditExecutionError): ReviewItem {
  return {
    id: `error:${error.scope}:${error.sourceId || error.controlId || error.code}`,
    severity: error.recoverability === "FATAL" ? "HIGH" : "MEDIUM",
    title: error.scope === "SOURCE" ? "Nguồn cần xử lý" : "Kiểm tra chưa hoàn tất",
    reason: error.safeUserMessage,
    action: error.recommendedAction,
    targetId: error.sourceId || error.controlId,
  };
}

function missingItem(plan: ControlPlan): ReviewItem {
  const missing = plan.missingCapabilities
    .map((capability) => capability.kind === "LEDGER_ENTRY"
      ? `Sổ kế toán TK ${capability.account}`
      : capability.kind === "INVOICE"
        ? "Hóa đơn Thuế"
        : capability.kind === "SALES_TRANSACTION"
          ? "Bảng kê bán hàng"
          : capability.kind)
    .join(", ");
  return {
    id: `plan:${plan.controlId}:${plan.status}`,
    severity: plan.status === "NEEDS_REVIEW" ? "HIGH" : "MEDIUM",
    title: plan.title,
    reason:
      plan.warnings[0] ||
      (plan.status === "MISSING_SOURCE"
        ? `Thiếu nguồn: ${missing || "chứng từ hoặc sổ cần thiết"}.`
        : "Cần xác định dữ liệu trước khi chạy."),
    action: missing ? `Bổ sung ${missing} để hoàn tất kiểm tra.` : "Bổ sung hoặc xác định lại nguồn dữ liệu.",
    targetId: plan.controlId,
  };
}

function findingItem(controlId: string, finding: ControlFinding): ReviewItem {
  return {
    id: `finding:${controlId}:${finding.code}:${finding.severity}:${finding.message}`,
    severity: severity(finding.severity),
    title: `Phát hiện từ ${controlId}`,
    reason: finding.message,
    action: "Mở chi tiết kiểm tra và đối chiếu chứng từ gốc.",
    targetId: controlId,
  };
}

export function reviewItems(report: AuditWorkspaceReport | null): ReviewItem[] {
  if (!report) return [];
  const errors = report.errors.map(errorItem);
  const plans = report.controlPlans
    .filter((plan) =>
      plan.status === "NEEDS_REVIEW"
      || plan.status === "NEEDS_MAPPING"
      || (plan.status === "MISSING_SOURCE" && plan.sourceIds.length > 0)
    )
    .map(missingItem);
  const findings = report.controlResults.flatMap((result) =>
    result.error
      ? []
      : result.findings.map((finding) => findingItem(result.controlId, finding))
  );
  return [...errors, ...plans, ...findings];
}

export function ReviewQueue({ report }: { report: AuditWorkspaceReport | null }) {
  const items = reviewItems(report);
  if (!report) return null;
  return (
    <section className="review-queue" aria-labelledby="review-queue-title">
      <div className="section-title-row">
        <div>
          <h2 id="review-queue-title" className="section-title">Việc cần rà soát</h2>
          <p className="audit-workspace-subtitle">
            Các mục này chưa phải kết luận kế toán và cần được kiểm tra bằng chứng.
          </p>
        </div>
        <span className="source-count-badge">{items.length} mục</span>
      </div>
      {items.length === 0 ? (
        <div className="empty-state compact" role="status">
          Không có mục rà soát phát sinh từ các kiểm tra đã chạy.
        </div>
      ) : (
        <div className="review-list">
          {items.map((item) => (
            <article className={`review-item severity-${item.severity.toLowerCase()}`} key={item.id}>
              <span className="severity-shape" aria-label={`Mức độ ${item.severity}`} />
              <div>
                <strong>{item.title}</strong>
                <p>{item.reason}</p>
                {item.action && <small>Hướng xử lý: {item.action}</small>}
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

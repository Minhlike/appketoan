import type { Dispatch, SetStateAction } from "react";

import type {
  AccountingPeriod,
  AuditWorkspaceReport,
  DataSource,
  DataSourceKind,
  SourceRole,
} from "../types/dataContract";
import type { AuditOperationState, IngestedSourceItem } from "../types/auditWorkspace";
import { AccountingPeriodPanel } from "./AccountingPeriodPanel";
import { AuditWorkspaceDashboard } from "./AuditWorkspaceDashboard";
import { ReviewQueue } from "./ReviewQueue";
import { SourceIntakePanel } from "./SourceIntakePanel";

interface Props {
  sources: IngestedSourceItem[];
  accountingPeriod: AccountingPeriod;
  setAccountingPeriod: Dispatch<SetStateAction<AccountingPeriod>>;
  report: AuditWorkspaceReport | null;
  operationState: AuditOperationState;
  errorMessage: string | null;
  canRun: boolean;
  advancedMode: boolean;
  onAddSource: (item: IngestedSourceItem) => void;
  onRemoveSource: (id: string) => void;
  onUpdateSheet: (id: string, sheet: string) => void;
  onUpdateKind: (id: string, kind: DataSourceKind) => void;
  onUpdateRole: (id: string, role: SourceRole) => void;
  onOpenMapping: (item: IngestedSourceItem) => void;
  onRun: () => void;
  onCancel: () => void;
}

export function AuditWorkspacePage(props: Props) {
  const busy = props.operationState === "RUNNING" || props.operationState === "CANCELLING";
  return (
    <section className="workspace-page" aria-label="Quy trình bộ hồ sơ kế toán">
      <nav className="workflow-steps" aria-label="Các bước kiểm tra">
        {[
          "1. Nạp bộ hồ sơ",
          "2. Chọn kỳ",
          "3. Kiểm tra",
          "4. Rà soát kết quả",
        ].map((step) => <span key={step}>{step}</span>)}
      </nav>

      {props.errorMessage && (
        <div className="alert-banner alert-error" role="alert">
          {props.errorMessage}
        </div>
      )}

      <AuditWorkspaceDashboard
        sources={props.sources}
        accountingPeriod={props.accountingPeriod}
        report={props.report}
      />

      <SourceIntakePanel
        sources={props.sources}
        onAddSource={props.onAddSource}
        onRemoveSource={props.onRemoveSource}
        onUpdateSheet={props.onUpdateSheet}
        onUpdateKind={props.onUpdateKind}
        onUpdateRole={props.onUpdateRole}
        onOpenMapping={props.onOpenMapping}
        disabled={busy}
        advancedMode={props.advancedMode}
      />

      <AccountingPeriodPanel
        accountingPeriod={props.accountingPeriod}
        setAccountingPeriod={props.setAccountingPeriod}
        operationState={props.operationState}
        canRun={props.canRun}
        busy={busy}
        onRun={props.onRun}
        onCancel={props.onCancel}
      />

      <ReviewQueue report={props.report} />
    </section>
  );
}

export type { IngestedSourceItem, DataSource };

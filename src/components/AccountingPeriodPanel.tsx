import type { Dispatch, SetStateAction } from "react";

import type { AccountingPeriod } from "../types/dataContract";
import type { AuditOperationState } from "../types/auditWorkspace";

interface Props {
  accountingPeriod: AccountingPeriod;
  setAccountingPeriod: Dispatch<SetStateAction<AccountingPeriod>>;
  operationState: AuditOperationState;
  canRun: boolean;
  busy: boolean;
  onRun: () => void;
  onCancel: () => void;
}

const operationLabel = (state: AuditOperationState) =>
  ({
    IDLE: "Sẵn sàng nạp hồ sơ",
    READY: "Sẵn sàng kiểm tra",
    RUNNING: "Đang đọc, chuẩn hóa và thực hiện các kiểm tra",
    CANCELLING: "Đang dừng an toàn",
    COMPLETED: "Đã hoàn tất kiểm tra",
    PARTIAL: "Đã hoàn tất một phần; có nguồn hoặc kiểm tra cần xử lý",
    ERROR: "Phiên kiểm tra cần xử lý",
  })[state];

export function AccountingPeriodPanel(props: Props) {
  return (
    <section className="period-panel" aria-labelledby="period-title">
      <div>
        <h2 id="period-title" className="section-title">Kỳ kế toán</h2>
        <p>Chỉ các bản ghi trong kỳ an toàn chung của từng kiểm tra mới được đối chiếu.</p>
      </div>
      <div className="period-controls">
        <label>
          Từ ngày
          <input
            className="input-control"
            type="date"
            value={props.accountingPeriod.startDate}
            onChange={(event) =>
              props.setAccountingPeriod((period) => ({
                ...period,
                startDate: event.target.value,
              }))
            }
            disabled={props.busy}
          />
        </label>
        <label>
          Đến ngày
          <input
            className="input-control"
            type="date"
            value={props.accountingPeriod.endDate}
            onChange={(event) =>
              props.setAccountingPeriod((period) => ({
                ...period,
                endDate: event.target.value,
              }))
            }
            disabled={props.busy}
          />
        </label>
        <button
          className="btn btn-primary btn-lg"
          type="button"
          onClick={props.onRun}
          disabled={!props.canRun}
        >
          {props.operationState === "RUNNING"
            ? "Đang kiểm tra…"
            : "Chạy tất cả kiểm tra có thể"}
        </button>
        {props.operationState === "RUNNING" && (
          <button className="btn btn-secondary" type="button" onClick={props.onCancel}>
            Dừng kiểm tra
          </button>
        )}
      </div>
      <p className="operation-status" role="status" aria-live="polite">
        {operationLabel(props.operationState)}
      </p>
    </section>
  );
}

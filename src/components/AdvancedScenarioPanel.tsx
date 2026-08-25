import type { MoneyValue, PreconfiguredScenario } from "../types/dataContract";
import { parseVietnameseMoneyInput } from "../utils/money";
import { ScenarioSelector } from "./ScenarioSelector";

interface Props {
  selectedScenario: PreconfiguredScenario;
  isRunning: boolean;
  sourceCount: number;
  toleranceInput: string;
  toleranceError: string | null;
  dateToleranceDays: number;
  enableAggregate: boolean;
  hasResult: boolean;
  onToggle: (open: boolean) => void;
  onSelectScenario: (scenario: PreconfiguredScenario) => void;
  onToleranceChange: (raw: string, value?: MoneyValue, error?: string) => void;
  onDateToleranceChange: (days: number) => void;
  onEnableAggregateChange: (enabled: boolean) => void;
  onRun: () => void;
  onExport: () => void;
}

export function AdvancedScenarioPanel(props: Props) {
  const singleSourceControl =
    props.selectedScenario.id === "scenario_partner_identity_control" ||
    props.selectedScenario.id === "scenario_sales_analysis_report_control";
  const runDisabled =
    props.isRunning ||
    (props.sourceCount < 2 && !singleSourceControl) ||
    props.toleranceError !== null;

  return (
    <details
      className="advanced-workflow"
      onToggle={(event) => props.onToggle(event.currentTarget.open)}
    >
      <summary>Thiết lập nâng cao theo kịch bản</summary>
      <ScenarioSelector
        selectedScenarioId={props.selectedScenario.id}
        onSelectScenario={props.onSelectScenario}
        disabled={props.isRunning}
      />

      <section className="action-toolbar-card" aria-label="Chạy kịch bản nâng cao">
        <div className="toolbar-options">
          <label className="option-control option-control-stacked">
            <span className="option-inline">
              <span>Dung sai số tiền:</span>
              <input
                type="text"
                className={`input-control input-sm ${props.toleranceError ? "input-error" : ""}`}
                value={props.toleranceInput}
                onChange={(event) => {
                  const raw = event.target.value;
                  const parsed = parseVietnameseMoneyInput(raw);
                  props.onToleranceChange(
                    raw,
                    parsed.success ? parsed.value : undefined,
                    parsed.success ? undefined : parsed.error
                  );
                }}
                disabled={props.isRunning}
                placeholder="0"
                aria-invalid={Boolean(props.toleranceError)}
              />
              <span className="unit-label">VND</span>
            </span>
            {props.toleranceError && (
              <span className="field-error" role="alert">{props.toleranceError}</span>
            )}
          </label>

          <label className="option-control">
            <span>Dung sai ngày:</span>
            <input
              type="number"
              min="0"
              className="input-control input-sm"
              value={props.dateToleranceDays}
              onChange={(event) =>
                props.onDateToleranceChange(Number.parseInt(event.target.value, 10) || 0)
              }
              disabled={props.isRunning}
            />
            <span className="unit-label">ngày</span>
          </label>

          <label className="checkbox-control">
            <input
              type="checkbox"
              checked={props.enableAggregate}
              onChange={(event) => props.onEnableAggregateChange(event.target.checked)}
              disabled={props.isRunning}
            />
            <span>Tự động khớp gộp tổng (1 hóa đơn = nhiều dòng sổ cái)</span>
          </label>
        </div>

        <div className="toolbar-buttons">
          <button
            type="button"
            className={`btn btn-primary btn-lg ${props.isRunning ? "btn-loading" : ""}`}
            onClick={props.onRun}
            disabled={runDisabled}
          >
            {props.isRunning ? "Đang đối chiếu…" : "CHẠY ĐỐI CHIẾU"}
          </button>

          {props.hasResult && (
            <button type="button" className="btn btn-success btn-lg" onClick={props.onExport}>
              Xuất báo cáo Excel (.xlsx)
            </button>
          )}
        </div>
      </section>
    </details>
  );
}

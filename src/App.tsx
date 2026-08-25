import { useState } from "react";
import "./App.css";

import type {
  ComparisonSemantic,
  DataSource,
  DataSourceKind,
  MoneyValue,
  SourceRole,
  ExcelFileMetadata,
  MatchGroup,
  PreconfiguredScenario,
  ReconciliationResult,
  ReconciliationSession,
  AccountingPeriod,
  AuditWorkspaceReport,
} from "./types/dataContract";
import {
  exportReport,
  getDemoDatasets,
  PRECONFIGURED_SCENARIOS,
  runReconciliation,
  runAuditWorkspace,
} from "./services/api";
import { parseVietnameseMoneyInput } from "./utils/money";

import { Header } from "./components/Header";
import { ScenarioSelector } from "./components/ScenarioSelector";
import { FileIngestionDropzone } from "./components/FileIngestionDropzone";
import { MappingModal } from "./components/MappingModal";
import { DashboardKPIs } from "./components/DashboardKPIs";
import { ResultTable } from "./components/ResultTable";
import { DetailInspectorModal } from "./components/DetailInspectorModal";
import { AuditWorkspaceDashboard } from "./components/AuditWorkspaceDashboard";

interface IngestedSourceItem {
  id: string;
  source: DataSource;
  fileMetadata: ExcelFileMetadata;
  rawBytes?: Uint8Array;
}

export function App() {
  const [selectedScenario, setSelectedScenario] = useState<PreconfiguredScenario>(
    PRECONFIGURED_SCENARIOS[0]
  );
  const [sources, setSources] = useState<IngestedSourceItem[]>([]);
  const [activeMappingItem, setActiveMappingItem] = useState<IngestedSourceItem | null>(null);

  // Settings
  const [toleranceVnd, setToleranceVnd] = useState<MoneyValue>(selectedScenario.defaultToleranceVnd);
  const [toleranceInput, setToleranceInput] = useState<string>(selectedScenario.defaultToleranceVnd);
  const [toleranceError, setToleranceError] = useState<string | null>(null);
  const [dateToleranceDays, setDateToleranceDays] = useState<number>(selectedScenario.defaultDateDays);
  const [enableAggregate, setEnableAggregate] = useState<boolean>(selectedScenario.enableAggregate);

  // Reconciliation state
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [result, setResult] = useState<ReconciliationResult | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>("ALL");
  const [inspectingGroup, setInspectingGroup] = useState<MatchGroup | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [auditReport, setAuditReport] = useState<AuditWorkspaceReport | null>(null);
  const [accountingPeriod, setAccountingPeriod] = useState<AccountingPeriod>({
    startDate: "",
    endDate: "",
  });
  const [advancedMode, setAdvancedMode] = useState(false);

  // Scenario change: automatically assign roles to current sources if kinds match
  const handleSelectScenario = (scenario: PreconfiguredScenario) => {
    setSelectedScenario(scenario);
    setToleranceVnd(scenario.defaultToleranceVnd);
    setToleranceInput(scenario.defaultToleranceVnd);
    setToleranceError(null);
    setDateToleranceDays(scenario.defaultDateDays);
    setEnableAggregate(scenario.enableAggregate);
    setErrorMessage(null);

    setSources((prev) =>
      prev.map((item, idx) => {
        const matchingRec = scenario.recommendedSources.find((r) => r.kind === item.source.kind);
        const assignedRole: SourceRole = matchingRec
          ? matchingRec.role
          : idx === 0
          ? "PRIMARY"
          : "REQUIRED_SECONDARY";
        return {
          ...item,
          source: {
            ...item.source,
            role: assignedRole,
          },
        };
      })
    );
    setAuditReport(null);
  };

  // Add source
  const handleAddSource = (item: IngestedSourceItem) => {
    // Automatically match role from selected scenario if kind matches
    const matchingRec = selectedScenario.recommendedSources.find(
      (r) => r.kind === item.source.kind
    );
    const assignedRole: SourceRole = matchingRec
      ? matchingRec.role
      : sources.length === 0
      ? "PRIMARY"
      : "REQUIRED_SECONDARY";

    const newItem: IngestedSourceItem = {
      ...item,
      source: {
        ...item.source,
        role: assignedRole,
      },
    };

    setSources((prev) => [...prev, newItem]);
    setAuditReport(null);
    setErrorMessage(null);
  };

  // Remove source
  const handleRemoveSource = (id: string) => {
    setSources((prev) => prev.filter((s) => s.id !== id));
    setAuditReport(null);
  };

  // Update sheet: re-runs detection and reclassifies source.kind
  const handleUpdateSheet = (id: string, newSheetName: string) => {
    setSources((prev) =>
      prev.map((item) => {
        if (item.id === id) {
          const sheetMeta = item.fileMetadata.sheets.find((s) => s.name === newSheetName);
          return {
            ...item,
            source: {
              ...item.source,
              sheetName: newSheetName,
              kind: sheetMeta?.suggestedKind || item.source.kind,
              headerRow: sheetMeta?.detectedHeaderRow || 1,
              dataStartRow: sheetMeta?.detectedDataStartRow || 2,
              columnMapping: sheetMeta?.suggestedMapping || item.source.columnMapping,
            },
          };
        }
        return item;
      })
    );
  };

  // Update source kind
  const handleUpdateKind = (id: string, newKind: DataSourceKind) => {
    setSources((prev) =>
      prev.map((item) =>
        item.id === id
          ? {
              ...item,
              source: {
                ...item.source,
                kind: newKind,
              },
            }
          : item
      )
    );
    setAuditReport(null);
  };

  // Update source role
  const handleUpdateRole = (id: string, newRole: SourceRole) => {
    setSources((prev) =>
      prev.map((item) =>
        item.id === id
          ? {
              ...item,
              source: {
                ...item.source,
                role: newRole,
              },
            }
          : item
      )
    );
  };

  // Save updated mapping
  const handleSaveMapping = (updatedSource: DataSource) => {
    setSources((prev) =>
      prev.map((item) => (item.id === updatedSource.id ? { ...item, source: updatedSource } : item))
    );
    setAuditReport(null);
  };

  // Load demo data
  const handleLoadDemoData = () => {
    const demo = getDemoDatasets();
    const demoEInvoices = demo.eInvoices;
    const demoLedger = demo.ledger511;

    const source1: IngestedSourceItem = {
      id: "src_demo_einvoice",
      source: {
        id: "src_demo_einvoice",
        name: "Bảng kê Hóa đơn điện tử bán ra (Demo)",
        filePath: "einvoices_demo.xlsx",
        sheetName: "Sheet1",
        kind: "e_invoice",
        role: "PRIMARY",
        headerRow: 1,
        dataStartRow: 2,
        columnMapping: {
          docNoColumn: "docNo",
          seriesColumn: "series",
          dateColumn: "date",
          partnerTaxIdColumn: "partnerTaxId",
          partnerNameColumn: "partnerName",
          pretaxAmountColumn: "pretaxAmount",
          vatAmountColumn: "vatAmount",
          totalAmountColumn: "totalAmount",
          vatRateColumn: "vatRate",
        },
      },
      fileMetadata: {
        filePath: "einvoices_demo.xlsx",
        fileName: "einvoices_demo.xlsx",
        fileSizeBytes: 24500,
        sheets: [
          {
            name: "Sheet1",
            totalRows: demoEInvoices.length,
            totalCols: 10,
            detectedHeaderRow: 1,
            detectedDataStartRow: 2,
            columns: ["docNo", "series", "date", "partnerTaxId", "partnerName", "pretaxAmount", "vatAmount", "totalAmount"],
            suggestedMapping: {},
            suggestedKind: "e_invoice",
            confidenceScore: 1.0,
            previewRows: [],
          },
        ],
      },
    };

    const source2: IngestedSourceItem = {
      id: "src_demo_ledger511",
      source: {
        id: "src_demo_ledger511",
        name: "Sổ cái TK 511 Doanh thu (Demo)",
        filePath: "ledger511_demo.xlsx",
        sheetName: "Sheet1",
        kind: "ledger_511",
        role: "REQUIRED_SECONDARY",
        headerRow: 1,
        dataStartRow: 2,
        columnMapping: {
          docNoColumn: "docNo",
          seriesColumn: "series",
          dateColumn: "date",
          partnerTaxIdColumn: "partnerTaxId",
          partnerNameColumn: "partnerName",
          pretaxAmountColumn: "pretaxAmount",
          vatAmountColumn: "vatAmount",
          totalAmountColumn: "totalAmount",
          debitAccountColumn: "debitAccount",
          creditAccountColumn: "creditAccount",
          voucherNoColumn: "voucherNo",
          descriptionColumn: "description",
        },
      },
      fileMetadata: {
        filePath: "ledger511_demo.xlsx",
        fileName: "ledger511_demo.xlsx",
        fileSizeBytes: 22100,
        sheets: [
          {
            name: "Sheet1",
            totalRows: demoLedger.length,
            totalCols: 11,
            detectedHeaderRow: 1,
            detectedDataStartRow: 2,
            columns: ["docNo", "series", "date", "partnerTaxId", "partnerName", "pretaxAmount", "vatAmount", "totalAmount", "voucherNo"],
            suggestedMapping: {},
            suggestedKind: "ledger_511",
            confidenceScore: 1.0,
            previewRows: [],
          },
        ],
      },
    };

    setSources([source1, source2]);
    setErrorMessage(null);
    setAuditReport(null);
  };

  const handleRunAuditWorkspace = async () => {
    if (sources.length === 0) {
      setErrorMessage("Vui lòng nạp ít nhất một nguồn vào bộ hồ sơ kế toán.");
      return;
    }
    if (!accountingPeriod.startDate || !accountingPeriod.endDate) {
      setErrorMessage("Vui lòng chọn đầy đủ ngày bắt đầu và kết thúc kỳ kế toán.");
      return;
    }
    if (accountingPeriod.startDate > accountingPeriod.endDate) {
      setErrorMessage("Ngày bắt đầu kỳ kế toán không được sau ngày kết thúc.");
      return;
    }
    try {
      setIsRunning(true);
      setErrorMessage(null);
      const fileBytesMap: Record<string, number[]> = {};
      for (const source of sources) {
        if (source.rawBytes) fileBytesMap[source.id] = Array.from(source.rawBytes);
      }
      const report = await runAuditWorkspace(
        `audit_${Date.now()}`,
        accountingPeriod,
        sources.map((item) => item.source),
        String(toleranceVnd),
        dateToleranceDays,
        Object.keys(fileBytesMap).length > 0 ? fileBytesMap : undefined
      );
      setAuditReport(report);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Không thể lập và chạy kế hoạch kiểm tra: ${msg}`);
    } finally {
      setIsRunning(false);
    }
  };

  // Reset
  const handleResetSession = () => {
    setSources([]);
    setResult(null);
    setStatusFilter("ALL");
    setExportMessage(null);
    setErrorMessage(null);
    setAuditReport(null);
  };

  // Run Reconciliation with strict validation
  const handleRunReconciliation = async () => {
    const isSingleSourceControl =
      selectedScenario.id === "scenario_partner_identity_control" ||
      selectedScenario.id === "scenario_sales_analysis_report_control";
    if (sources.length < 2 && !isSingleSourceControl) {
      setErrorMessage("Vui lòng tải lên ít nhất 2 nguồn dữ liệu Excel để thực hiện đối chiếu.");
      return;
    }

    // Enforce required sources for scenario
    const requiredScenarioSources = selectedScenario.recommendedSources.filter((r) => r.required);
    for (const req of requiredScenarioSources) {
      const isPresent = sources.some(
        (s) => s.source.kind === req.kind || (s.source.role === "PRIMARY" && req.role === "PRIMARY")
      );
      if (!isPresent && selectedScenario.id !== "scenario_custom_multi_source") {
        setErrorMessage(
          `Kịch bản "${selectedScenario.name}" yêu cầu bắt buộc có nguồn: "${req.title}". Vui lòng tải lên file dữ liệu tương ứng.`
        );
        return;
      }
    }

    try {
      setIsRunning(true);
      setErrorMessage(null);
      setExportMessage(null);

      const primarySource =
        sources.find((s) => s.source.role === "PRIMARY") ||
        sources.find((s) => s.source.kind === "e_invoice") ||
        sources[0];

      const requiredSourceIds = sources
        .filter((s) => s.source.role === "REQUIRED_SECONDARY" || s.source.role === "PRIMARY")
        .map((s) => s.source.id);

      const optionalSourceIds = sources
        .filter((s) => s.source.role === "OPTIONAL_SECONDARY")
        .map((s) => s.source.id);

      const expectedPrimaryKind = selectedScenario.recommendedSources.find(
        (r) => r.role === "PRIMARY"
      )?.kind;

      const comparisonRules = selectedScenario.rules.map((r, idx) => {
        const secKind: DataSourceKind =
          r.secondarySourceKind ||
          (r.semantic === "VAT"
            ? selectedScenario.id === "scenario_input_vat"
              ? "ledger_133"
              : "ledger_3331"
            : r.semantic === "RECEIVABLE"
            ? "ledger_131"
            : r.semantic === "BANK_PAYMENT"
            ? "bank_statement"
            : "ledger_511");

        return {
          id: `rule_${idx}_${r.semantic.toLowerCase()}`,
          name: r.title,
          semantic: r.semantic,
          primarySourceKind: expectedPrimaryKind || "e_invoice",
          primaryField: r.primaryField,
          secondarySourceKind: secKind,
          secondaryField: r.secondaryField,
          isRequired: true,
          toleranceVnd: String(r.toleranceVnd !== undefined ? r.toleranceVnd : toleranceVnd),
          dateToleranceDays: r.dateToleranceDays !== undefined ? r.dateToleranceDays : dateToleranceDays,
        };
      });

      const session: ReconciliationSession = {
        sessionId: `sess_${Date.now()}`,
        scenarioName: selectedScenario.name,
        primarySourceId: primarySource?.id,
        expectedPrimaryKind: expectedPrimaryKind,
        requiredSourceIds: requiredSourceIds.length > 0 ? requiredSourceIds : undefined,
        optionalSourceIds: optionalSourceIds.length > 0 ? optionalSourceIds : undefined,
        dataSources: sources.map((s) => s.source),
        comparisonRules,
        matchingToleranceVnd: String(toleranceVnd),
        dateToleranceDays: dateToleranceDays,
        enableAggregateMatch: enableAggregate,
      };

      const fileBytesMap: Record<string, number[]> = {};
      sources.forEach((s) => {
        if (s.rawBytes) {
          fileBytesMap[s.id] = Array.from(s.rawBytes);
        }
      });

      const res = await runReconciliation(
        session,
        Object.keys(fileBytesMap).length > 0 ? fileBytesMap : undefined
      );

      setResult(res);
      setStatusFilter("ALL");
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Lỗi khi thực hiện đối chiếu: ${msg}`);
    } finally {
      setIsRunning(false);
    }
  };

  // Export
  const handleExport = async () => {
    if (!result) return;
    try {
      setExportMessage("Đang tạo file Excel báo cáo...");
      const summary = await exportReport(result);
      setExportMessage(`✓ Đã xuất báo cáo thành công vào: ${summary.outputPath}`);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorMessage(`Không thể xuất báo cáo: ${msg}`);
    }
  };

  return (
    <div className="app-layout">
      <Header
        onLoadDemoData={handleLoadDemoData}
        onResetSession={handleResetSession}
        hasResults={Boolean(result)}
      />

      <main className="app-main-content">
        {errorMessage && (
          <div className="alert-banner alert-error" role="alert">
            <span>⚠️ {errorMessage}</span>
            <button type="button" className="btn-close" onClick={() => setErrorMessage(null)}>
              ✕
            </button>
          </div>
        )}

        {exportMessage && (
          <div className="alert-banner alert-success" role="status">
            <span>{exportMessage}</span>
            <button type="button" className="btn-close" onClick={() => setExportMessage(null)}>
              ✕
            </button>
          </div>
        )}

        <AuditWorkspaceDashboard
          sources={sources}
          accountingPeriod={accountingPeriod}
          report={auditReport}
        />

        <FileIngestionDropzone
          sources={sources}
          onAddSource={handleAddSource}
          onRemoveSource={handleRemoveSource}
          onUpdateSheet={handleUpdateSheet}
          onUpdateKind={handleUpdateKind}
          onUpdateRole={handleUpdateRole}
          onOpenMapping={(item) => setActiveMappingItem(item)}
          disabled={isRunning}
          advancedMode={advancedMode}
        />

        <div className="period-controls" aria-label="Kỳ kế toán">
          <label>
            Từ ngày
            <input
              className="input-control"
              type="date"
              value={accountingPeriod.startDate}
              onChange={(event) => {
                setAccountingPeriod((period) => ({ ...period, startDate: event.target.value }));
                setAuditReport(null);
              }}
              disabled={isRunning}
            />
          </label>
          <label>
            Đến ngày
            <input
              className="input-control"
              type="date"
              value={accountingPeriod.endDate}
              onChange={(event) => {
                setAccountingPeriod((period) => ({ ...period, endDate: event.target.value }));
                setAuditReport(null);
              }}
              disabled={isRunning}
            />
          </label>
          <button
            type="button"
            className={`btn btn-primary btn-lg ${isRunning ? "btn-loading" : ""}`}
            onClick={() => void handleRunAuditWorkspace()}
            disabled={
              isRunning ||
              sources.length === 0 ||
              !accountingPeriod.startDate ||
              !accountingPeriod.endDate
            }
          >
            {isRunning ? "⏳ Đang kiểm tra..." : "▶ Chạy tất cả kiểm tra có thể"}
          </button>
        </div>

        <details
          className="advanced-workflow"
          onToggle={(event) => setAdvancedMode(event.currentTarget.open)}
        >
          <summary>Thiết lập nâng cao theo kịch bản</summary>
          <ScenarioSelector
            selectedScenarioId={selectedScenario.id}
            onSelectScenario={handleSelectScenario}
            disabled={isRunning}
          />

        {/* Legacy scenario execution remains available as an advanced workflow. */}
        <section className="action-toolbar-card">
          <div className="toolbar-options">
            <label className="option-control" style={{ display: "flex", flexDirection: "column", alignItems: "flex-start" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                <span>Dung sai số tiền:</span>
                <input
                  type="text"
                  className={`input-control input-sm ${toleranceError ? "input-error" : ""}`}
                  value={toleranceInput}
                  onChange={(e) => {
                    const raw = e.target.value;
                    setToleranceInput(raw);
                    const parsed = parseVietnameseMoneyInput(raw);
                    if (parsed.success) {
                      setToleranceVnd(parsed.value);
                      setToleranceError(null);
                    } else {
                      setToleranceError(parsed.error);
                    }
                  }}
                  disabled={isRunning}
                  placeholder="0"
                  style={{
                    borderColor: toleranceError ? "#ef4444" : undefined,
                  }}
                />
                <span className="unit-label">VND</span>
              </div>
              {toleranceError && (
                <span
                  style={{
                    color: "#dc2626",
                    fontSize: "0.75rem",
                    marginTop: "0.25rem",
                  }}
                >
                  ⚠️ {toleranceError}
                </span>
              )}
            </label>

            <label className="option-control">
              <span>Dung sai ngày:</span>
              <input
                type="number"
                min="0"
                className="input-control input-sm"
                value={dateToleranceDays}
                onChange={(e) => setDateToleranceDays(parseInt(e.target.value, 10) || 0)}
                disabled={isRunning}
              />
              <span className="unit-label">ngày</span>
            </label>

            <label className="checkbox-control">
              <input
                type="checkbox"
                checked={enableAggregate}
                onChange={(e) => setEnableAggregate(e.target.checked)}
                disabled={isRunning}
              />
              <span>Tự động khớp gộp tổng (1 hóa đơn = nhiều dòng sổ cái)</span>
            </label>
          </div>

          <div className="toolbar-buttons">
            <button
              type="button"
              className={`btn btn-primary btn-lg ${isRunning ? "btn-loading" : ""}`}
              onClick={() => void handleRunReconciliation()}
              disabled={isRunning || (sources.length < 2 && !(
                selectedScenario.id === "scenario_partner_identity_control" ||
                selectedScenario.id === "scenario_sales_analysis_report_control"
              )) || toleranceError !== null}
            >
              {isRunning ? "⏳ Đang đối chiếu..." : "▶ CHẠY ĐỐI CHIẾU"}
            </button>

            {result && (
              <button
                type="button"
                className="btn btn-success btn-lg"
                onClick={() => void handleExport()}
              >
                📊 Xuất Báo Cáo Excel (.xlsx)
              </button>
            )}
          </div>
        </section>
        </details>

        {/* Results Area */}
        {result && (
          <>
            {result.intakeAnalysis &&
              result.intakeAnalysis.totalPhysicalSources >
                result.intakeAnalysis.uniqueDatasetsCount && (
                <div
                  className="alert-banner alert-warning intake-summary-banner"
                  role="alert"
                  style={{
                    marginBottom: "1.5rem",
                    backgroundColor: "#fffbeb",
                    border: "1px solid #fef3c7",
                    borderRadius: "8px",
                    padding: "1rem",
                    color: "#92400e",
                  }}
                >
                  <div>
                    <strong style={{ fontSize: "1rem" }}>
                      🛡️ Phân tích tệp nguồn ({result.intakeAnalysis.totalPhysicalSources} tệp vật lý → {result.intakeAnalysis.uniqueDatasetsCount} tập dữ liệu logic):
                    </strong>
                    <div style={{ marginTop: "0.5rem", fontSize: "0.875rem" }}>
                      {result.intakeAnalysis.sourceAnalyses
                        .filter((a) => !a.isEligibleForReconciliation)
                        .map((a) => (
                          <div key={a.sourceId} style={{ marginTop: "0.25rem" }}>
                            • <strong>{a.sourceName}</strong>: {a.diagnosticMessage}
                          </div>
                        ))}
                    </div>
                  </div>
                </div>
              )}

            <DashboardKPIs
              summary={result.summary}
              activeFilter={statusFilter}
              onSelectFilter={(st) => setStatusFilter(st)}
              activeSemantics={(() => {
                const presentKinds = new Set(sources.map((s) => s.source.kind));
                const checked = new Set<ComparisonSemantic>();
                for (const rule of selectedScenario.rules) {
                  const secKind: DataSourceKind =
                    rule.secondarySourceKind ||
                    (rule.semantic === "VAT"
                      ? selectedScenario.id === "scenario_input_vat"
                        ? "ledger_133"
                        : "ledger_3331"
                      : rule.semantic === "RECEIVABLE"
                      ? "ledger_131"
                      : "ledger_511");
                  if (presentKinds.has(secKind)) {
                    checked.add(rule.semantic as ComparisonSemantic);
                  }
                }
                return Array.from(checked);
              })()}
            />

            {result.referenceControls && result.referenceControls.length > 0 && (
              <section className="results-section" aria-label="Kết quả kiểm soát nguồn tham chiếu">
                <h2>Kiểm soát nguồn tham chiếu</h2>
                {result.referenceControls.map((control) => (
                  <div className="alert-banner" role="status" key={control.sourceId}>
                    <strong>{control.sourceKind}</strong> · {control.recordCount} dòng · {control.status}
                    <div>{control.message}</div>
                  </div>
                ))}
              </section>
            )}

            <ResultTable
              groups={result.groups}
              activeStatusFilter={statusFilter}
              onSelectStatusFilter={(st) => setStatusFilter(st)}
              onOpenDetail={(g) => setInspectingGroup(g)}
            />
          </>
        )}
      </main>

      {/* Mapping modal */}
      {activeMappingItem && (
        <MappingModal
          source={activeMappingItem.source}
          fileMetadata={activeMappingItem.fileMetadata}
          isOpen={Boolean(activeMappingItem)}
          onClose={() => setActiveMappingItem(null)}
          onSave={handleSaveMapping}
        />
      )}

      {/* Detail Inspector Modal */}
      {inspectingGroup && (
        <DetailInspectorModal
          group={inspectingGroup}
          isOpen={Boolean(inspectingGroup)}
          onClose={() => setInspectingGroup(null)}
        />
      )}
    </div>
  );
}

export default App;

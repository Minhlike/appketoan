import { useState } from "react";
import "./App.css";

import type {
  DataSource,
  DataSourceKind,
  ExcelFileMetadata,
  MatchGroup,
  PreconfiguredScenario,
  ReconciliationResult,
  ReconciliationSession,
} from "./types/dataContract";
import {
  exportReport,
  getDemoDatasets,
  PRECONFIGURED_SCENARIOS,
  runReconciliation,
} from "./services/api";

import { Header } from "./components/Header";
import { ScenarioSelector } from "./components/ScenarioSelector";
import { FileIngestionDropzone } from "./components/FileIngestionDropzone";
import { MappingModal } from "./components/MappingModal";
import { DashboardKPIs } from "./components/DashboardKPIs";
import { ResultTable } from "./components/ResultTable";
import { DetailInspectorModal } from "./components/DetailInspectorModal";

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
  const [toleranceVnd, setToleranceVnd] = useState<number>(selectedScenario.defaultToleranceVnd);
  const [dateToleranceDays, setDateToleranceDays] = useState<number>(selectedScenario.defaultDateDays);
  const [enableAggregate, setEnableAggregate] = useState<boolean>(selectedScenario.enableAggregate);

  // Reconciliation state
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [result, setResult] = useState<ReconciliationResult | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>("ALL");
  const [inspectingGroup, setInspectingGroup] = useState<MatchGroup | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Scenario change
  const handleSelectScenario = (scenario: PreconfiguredScenario) => {
    setSelectedScenario(scenario);
    setToleranceVnd(scenario.defaultToleranceVnd);
    setDateToleranceDays(scenario.defaultDateDays);
    setEnableAggregate(scenario.enableAggregate);
  };

  // Add source
  const handleAddSource = (item: IngestedSourceItem) => {
    setSources((prev) => [...prev, item]);
    setErrorMessage(null);
  };

  // Remove source
  const handleRemoveSource = (id: string) => {
    setSources((prev) => prev.filter((s) => s.id !== id));
  };

  // Update sheet
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
  };

  // Save updated mapping
  const handleSaveMapping = (updatedSource: DataSource) => {
    setSources((prev) =>
      prev.map((item) => (item.id === updatedSource.id ? { ...item, source: updatedSource } : item))
    );
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
  };

  // Reset
  const handleResetSession = () => {
    setSources([]);
    setResult(null);
    setStatusFilter("ALL");
    setExportMessage(null);
    setErrorMessage(null);
  };

  // Run Reconciliation
  const handleRunReconciliation = async () => {
    if (sources.length < 2) {
      setErrorMessage("Vui lòng tải lên ít nhất 2 nguồn dữ liệu Excel để thực hiện đối chiếu.");
      return;
    }

    try {
      setIsRunning(true);
      setErrorMessage(null);
      setExportMessage(null);

      const primarySource =
        sources.find((s) => s.source.kind === "e_invoice") || sources[0];

      const session: ReconciliationSession = {
        sessionId: `sess_${Date.now()}`,
        scenarioName: selectedScenario.name,
        primarySourceId: primarySource?.id,
        dataSources: sources.map((s) => s.source),
        matchingToleranceVnd: toleranceVnd,
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

        {/* Step 1: Scenario selection */}
        <ScenarioSelector
          selectedScenarioId={selectedScenario.id}
          onSelectScenario={handleSelectScenario}
          disabled={isRunning}
        />

        {/* Step 2: Multi-file ingestion */}
        <FileIngestionDropzone
          sources={sources}
          onAddSource={handleAddSource}
          onRemoveSource={handleRemoveSource}
          onUpdateSheet={handleUpdateSheet}
          onUpdateKind={handleUpdateKind}
          onOpenMapping={(item) => setActiveMappingItem(item)}
          disabled={isRunning}
        />

        {/* Options & Action Bar */}
        <section className="action-toolbar-card">
          <div className="toolbar-options">
            <label className="option-control">
              <span>Dung sai số tiền:</span>
              <input
                type="number"
                min="0"
                step="1"
                className="input-control input-sm"
                value={toleranceVnd}
                onChange={(e) => setToleranceVnd(parseFloat(e.target.value) || 0)}
                disabled={isRunning}
              />
              <span className="unit-label">VND</span>
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
              disabled={isRunning || sources.length < 2}
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

        {/* Results Area */}
        {result && (
          <>
            <DashboardKPIs
              summary={result.summary}
              activeFilter={statusFilter}
              onSelectFilter={(st) => setStatusFilter(st)}
            />

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

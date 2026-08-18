/**
 * Data Contract Interfaces for Multi-Source Accounting Reconciliation
 * Synchronized with Rust models in `crates/reconciliation-core/src/models/`
 */

export type DataSourceKind =
  | "e_invoice"
  | "ledger_511"
  | "ledger_3331"
  | "ledger_133"
  | "ledger_131"
  | "bank_statement"
  | "cash_book"
  | "branch_ledger"
  | "custom";

export interface ColumnMapping {
  dateColumn?: string;
  docNoColumn?: string;
  seriesColumn?: string;
  templateCodeColumn?: string;
  partnerTaxIdColumn?: string;
  partnerNameColumn?: string;
  pretaxAmountColumn?: string;
  vatAmountColumn?: string;
  totalAmountColumn?: string;
  debitAmountColumn?: string;
  creditAmountColumn?: string;
  vatRateColumn?: string;
  debitAccountColumn?: string;
  creditAccountColumn?: string;
  voucherNoColumn?: string;
  descriptionColumn?: string;
  bankAccountColumn?: string;
}

export interface DataSource {
  id: string;
  name: string;
  filePath: string;
  sheetName: string;
  kind: DataSourceKind;
  headerRow: number;
  dataStartRow: number;
  columnMapping: ColumnMapping;
}

export interface SheetMetadata {
  name: string;
  totalRows: number;
  totalCols: number;
  detectedHeaderRow: number;
  detectedDataStartRow: number;
  columns: string[];
  suggestedMapping: ColumnMapping;
  suggestedKind: DataSourceKind;
  confidenceScore: number;
  previewRows: string[][];
}

export interface ExcelFileMetadata {
  filePath: string;
  fileName: string;
  fileSizeBytes: number;
  sheets: SheetMetadata[];
}

export interface ReconciliationSession {
  sessionId: string;
  scenarioName: string;
  dataSources: DataSource[];
  matchingToleranceVnd: number;
  dateToleranceDays: number;
  enableAggregateMatch: boolean;
}

export interface CanonicalRecord {
  id: string;
  sourceId: string;
  sourceRow: number;
  date?: string;
  docNo?: string;
  series?: string;
  templateCode?: string;
  partnerTaxId?: string;
  partnerName?: string;
  pretaxAmount?: number;
  vatAmount?: number;
  totalAmount: number;
  debitAmount?: number;
  creditAmount?: number;
  vatRate?: string;
  debitAccount?: string;
  creditAccount?: string;
  voucherNo?: string;
  description?: string;
  bankAccount?: string;
  rawFields?: Record<string, string>;
}

export type MatchKeyType =
  | "doc_no"
  | "series_and_doc_no"
  | "tax_id_and_amount"
  | "tax_id_and_doc_no"
  | { custom_keys: string[] };

export interface MatchingRule {
  id: string;
  name: string;
  primaryKeys: MatchKeyType[];
  allowDateVarianceDays: number;
  allowAmountToleranceVnd: number;
  enableAggregateMatch: boolean;
  aggregateGroupingKeys?: string[];
}

export interface ReconciliationProfile {
  id: string;
  name: string;
  sourceIds: string[];
  rules: MatchingRule[];
}

export type MatchStatus =
  | "MATCHED_EXACT"
  | "MATCHED_WITH_TOLERANCE"
  | "MATCHED_AGGREGATE"
  | "MISMATCH_AMOUNT"
  | "MISMATCH_METADATA"
  | "UNMATCHED_MISSING_IN_TARGET"
  | "UNMATCHED_MISSING_IN_SOURCE"
  | "DUPLICATE_SUSPECT"
  | "AMBIGUOUS_MATCH";

export interface FieldDiscrepancy {
  fieldName: string;
  sourceValue?: string;
  targetValue?: string;
  amountDiff?: number;
  message: string;
}

export interface SourceMatchBreakdown {
  sourceId: string;
  sourceName: string;
  recordIds: string[];
  comparedAmount: number;
  status: MatchStatus;
  discrepancies?: FieldDiscrepancy[];
}

export interface MatchGroup {
  id: string;
  status: MatchStatus;
  primarySourceRecordIds: string[];
  targetSourceRecordIds: string[];
  sourceBreakdowns?: Record<string, SourceMatchBreakdown>;
  discrepancies?: FieldDiscrepancy[];
  totalSourceAmount: number;
  totalTargetAmount: number;
  amountVariance: number;
}

export interface ReconciliationSummary {
  totalSourceRecords: number;
  totalTargetRecords: number;
  exactMatchesCount: number;
  toleranceMatchesCount: number;
  aggregateMatchesCount: number;
  mismatchesCount: number;
  missingInTargetCount: number;
  missingInSourceCount: number;
  duplicatesCount: number;
  ambiguousCount: number;
  netFinancialVariance: number;
}

export interface ReconciliationResult {
  sessionId: string;
  executedAt: string;
  profileId: string;
  summary: ReconciliationSummary;
  groups: MatchGroup[];
}

export interface ExportSummary {
  outputPath: string;
  fileSizeBytes: number;
  totalGroupsExported: number;
  createdAt: string;
}

export interface PreconfiguredScenario {
  id: string;
  name: string;
  description: string;
  recommendedSources: {
    kind: DataSourceKind;
    title: string;
    description: string;
    required: boolean;
  }[];
  defaultToleranceVnd: number;
  defaultDateDays: number;
  enableAggregate: boolean;
}

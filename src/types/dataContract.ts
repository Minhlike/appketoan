/**
 * Data Contract Interfaces for Multi-Source Accounting Reconciliation
 * Synchronized with Rust models in `crates/reconciliation-core/src/models/`
 */

export type SourceRole = "PRIMARY" | "REQUIRED_SECONDARY" | "OPTIONAL_SECONDARY";

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

export type ComparisonSemantic =
  | "REVENUE"
  | "VAT"
  | "RECEIVABLE"
  | "BANK_PAYMENT"
  | "OTHER";

export type MoneyValue = string;

export interface ComparisonRule {
  id: string;
  name: string;
  semantic: ComparisonSemantic;
  primarySourceKind: DataSourceKind;
  primaryField: string;
  secondarySourceKind: DataSourceKind;
  secondaryField: string;
  isRequired: boolean;
  toleranceVnd: MoneyValue;
  dateToleranceDays: number;
}

export interface ColumnMapping {
  dateColumn?: string;
  docNoColumn?: string;
  docCodeColumn?: string;
  seriesColumn?: string;
  templateCodeColumn?: string;
  partnerTaxIdColumn?: string;
  buyerTaxIdColumn?: string;
  sellerTaxIdColumn?: string;
  partnerNameColumn?: string;
  pretaxAmountColumn?: string;
  vatAmountColumn?: string;
  discountAmountColumn?: string;
  feeAmountColumn?: string;
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
  role?: SourceRole;
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
  primarySourceId?: string;
  expectedPrimaryKind?: DataSourceKind;
  requiredSourceIds?: string[];
  optionalSourceIds?: string[];
  dataSources: DataSource[];
  comparisonRules?: ComparisonRule[];
  matchingToleranceVnd: MoneyValue;
  dateToleranceDays: number;
  enableAggregateMatch: boolean;
}

export type ValueOrigin = "SOURCE" | "DERIVED";

export interface CanonicalRecord {
  id: string;
  sourceId: string;
  sourceRow: number;
  date?: string;
  docNo?: string;
  docCode?: string;
  series?: string;
  templateCode?: string;
  partnerTaxId?: string;
  buyerTaxId?: string;
  sellerTaxId?: string;
  partnerName?: string;
  pretaxAmount?: MoneyValue;
  vatAmount?: MoneyValue;
  discountAmount?: MoneyValue;
  feeAmount?: MoneyValue;
  totalAmount: MoneyValue;
  totalAmountOrigin?: ValueOrigin;
  debitAmount?: MoneyValue;
  creditAmount?: MoneyValue;
  vatRate?: string;
  debitAccount?: string;
  creditAccount?: string;
  voucherNo?: string;
  description?: string;
  bankAccount?: string;
  rawFields?: Record<string, string>;
}

export type MatchStatus =
  | "MATCHED_EXACT"
  | "MATCHED_WITH_TOLERANCE"
  | "MATCHED_AGGREGATE"
  | "MATCHED_WITH_MISSING_SOURCE"
  | "MISMATCH_AMOUNT"
  | "MISMATCH_METADATA"
  | "UNMATCHED_MISSING_IN_TARGET"
  | "UNMATCHED_MISSING_IN_SOURCE"
  | "DUPLICATE_SUSPECT"
  | "AMBIGUOUS_MATCH"
  | "NEEDS_REVIEW"
  | "INSUFFICIENT_MATCHING_EVIDENCE"
  /** Semantic was NOT evaluated — secondary source absent from this session */
  | "NOT_CHECKED";

export interface FieldDiscrepancy {
  fieldName: string;
  sourceValue?: string;
  targetValue?: string;
  amountDiff?: MoneyValue;
  message: string;
}

export interface SourceMatchBreakdown {
  sourceId: string;
  sourceName: string;
  recordIds: string[];
  comparedAmount: MoneyValue;
  status: MatchStatus;
  discrepancies?: FieldDiscrepancy[];
}

export interface SemanticFieldComparison {
  semantic: ComparisonSemantic;
  semanticName: string;
  primarySourceId: string;
  primarySourceName: string;
  secondarySourceId: string;
  secondarySourceName: string;
  secondarySourceKind: DataSourceKind;
  semanticField: string;
  expectedAmount: MoneyValue;
  actualAmount: MoneyValue;
  variance: MoneyValue;
  status: MatchStatus;
  primaryRecordIds?: string[];
  secondaryRecordIds?: string[];
  discrepancies?: FieldDiscrepancy[];
}

export interface MatchGroup {
  id: string;
  status: MatchStatus;
  docNo?: string;
  series?: string;
  date?: string;
  partnerName?: string;
  primarySourceRecordIds: string[];
  targetSourceRecordIds: string[];
  sourceBreakdowns?: Record<string, SourceMatchBreakdown>;
  discrepancies?: FieldDiscrepancy[];
  semanticComparisons?: SemanticFieldComparison[];
  revenueVariance?: MoneyValue;
  vatVariance?: MoneyValue;
  receivableVariance?: MoneyValue;
  otherVariance?: MoneyValue;
  totalSourceAmount: MoneyValue;
  totalTargetAmount: MoneyValue;
  amountVariance: MoneyValue;
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
  needsReviewCount?: number;
  revenueVariance?: MoneyValue;
  vatVariance?: MoneyValue;
  receivableVariance?: MoneyValue;
  totalDiscrepantAmount?: MoneyValue;
  netFinancialVariance: MoneyValue;
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

export interface ScenarioRuleDefinition {
  semantic: ComparisonSemantic;
  title: string;
  description: string;
  primaryField: string;
  secondarySourceKind?: DataSourceKind;
  secondaryField: string;
  toleranceVnd?: number;
  dateToleranceDays?: number;
}

export interface PreconfiguredScenario {
  id: string;
  name: string;
  description: string;
  recommendedSources: {
    kind: DataSourceKind;
    role: SourceRole;
    title: string;
    description: string;
    required: boolean;
  }[];
  rules: ScenarioRuleDefinition[];
  defaultToleranceVnd: number;
  defaultDateDays: number;
  enableAggregate: boolean;
}

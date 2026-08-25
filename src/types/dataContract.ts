/**
 * Data Contract Interfaces for Multi-Source Accounting Reconciliation
 * Synchronized with Rust models in `crates/reconciliation-core/src/models/`
 */

export type SourceRole = "PRIMARY" | "REQUIRED_SECONDARY" | "OPTIONAL_SECONDARY" | "REFERENCE_MASTER";

export type DataSourceKind =
  | "e_invoice"
  | "ledger_511"
  | "ledger_3331"
  | "ledger_133"
  | "ledger_131"
  | "ledger_112"
  | "partner_master"
  | "sales_register"
  | "sales_analysis_report"
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
  partnerCodeColumn?: string;
  addressColumn?: string;
  isCustomerColumn?: string;
  isSupplierColumn?: string;
  statusColumn?: string;
  currencyColumn?: string;
  exchangeRateColumn?: string;
  invoiceStatusColumn?: string;
  invoiceCheckResultColumn?: string;
  transactionNumberColumn?: string;
  transactionDateColumn?: string;
  accountingDateColumn?: string;
  counterpartyAccountColumn?: string;
  counterpartyNameColumn?: string;
  balanceColumn?: string;
  productCodeColumn?: string;
  productNameColumn?: string;
  quantityColumn?: string;
  unitPriceColumn?: string;
  revenueColumn?: string;
  costColumn?: string;
  profitColumn?: string;
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
  rangeStartRow?: number;
  detectedHeaderRow: number;
  detectedDataStartRow: number;
  physicalHeaderRow?: number;
  physicalDataStartRow?: number;
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

export interface AccountingPeriod {
  startDate: string;
  endDate: string;
}

export type SourceCapability =
  | { kind: "LEDGER_ENTRY"; account: string }
  | { kind: "INVOICE" }
  | { kind: "BANK_TRANSACTION" }
  | { kind: "PARTNER_MASTER" }
  | { kind: "SALES_TRANSACTION" }
  | { kind: "ANALYTICAL_SALES_REPORT" };

export type ControlPlanStatus =
  | "READY"
  | "MISSING_SOURCE"
  | "NEEDS_MAPPING"
  | "NEEDS_REVIEW"
  | "NOT_APPLICABLE";

export interface PeriodEvidence {
  earliestDate?: string;
  latestDate?: string;
  recordsInPeriod: number;
  recordsOutsidePeriod: number;
  missingOrUnparseableDates: number;
}

export interface SourceCatalogEntry {
  sourceId: string;
  sourceName: string;
  sourceKind: DataSourceKind;
  capabilities: SourceCapability[];
  recordCount: number;
  mappingComplete: boolean;
  periodEvidence: PeriodEvidence;
  warnings: string[];
  provenanceSha256?: string;
}

export interface ControlPlan {
  controlId: string;
  title: string;
  status: ControlPlanStatus;
  sourceIds: string[];
  missingCapabilities: SourceCapability[];
  warnings: string[];
  effectivePeriod?: AccountingPeriod;
}

export type ControlExecutionStatus =
  | "PASS"
  | "NEEDS_REVIEW"
  | "NOT_RUN"
  | "FAILED"
  | "CANCELLED";

export type AuditRunStatus = "COMPLETED" | "PARTIAL" | "FAILED" | "CANCELLED";

export type AuditErrorCode =
  | "SOURCE_READ_ERROR"
  | "INVALID_WORKBOOK"
  | "UNSUPPORTED_FORMAT"
  | "PASSWORD_PROTECTED_WORKBOOK"
  | "NO_VISIBLE_SHEET"
  | "HEADER_NOT_DETECTED"
  | "MAPPING_INCOMPLETE"
  | "INVALID_DATE"
  | "ACCOUNTING_PERIOD_INVALID"
  | "CAPABILITY_AMBIGUOUS"
  | "CAPABILITY_MISSING"
  | "CONTROL_PRECONDITION_FAILED"
  | "CONTROL_EXECUTION_FAILED"
  | "COMPLEXITY_LIMIT"
  | "EXPORT_FAILED"
  | "OUT_OF_MEMORY_RISK"
  | "CANCELLED"
  | "INTERNAL_ERROR";

export interface AuditExecutionError {
  code: AuditErrorCode;
  scope: "SESSION" | "SOURCE" | "CONTROL" | "EXPORT";
  sourceId?: string;
  controlId?: string;
  safeUserMessage: string;
  technicalDetail?: string;
  recoverability: "RETRY" | "USER_ACTION_REQUIRED" | "CONTINUE_OTHER_CONTROLS" | "FATAL";
  recommendedAction?: string;
}

export interface ControlFinding {
  code: string;
  severity: string;
  message: string;
}

export interface ControlResult {
  controlId: string;
  status: ControlExecutionStatus;
  evidence: string[];
  findings: ControlFinding[];
  sourceIds: string[];
  missingCapabilities: SourceCapability[];
  effectivePeriod?: AccountingPeriod;
  elapsedMs: number;
  summaryMetrics: Record<string, number>;
  limitations: string[];
  error?: AuditExecutionError;
  reconciliationResult?: ReconciliationResult;
}

export interface SourceReuseEvidence {
  sourceId: string;
  readCount: number;
  normalizeCount: number;
  indexCount: number;
  normalizedRecordCount: number;
  cacheHit: boolean;
}

export interface AuditExecutionMetrics {
  stages: {
    fileReadMs: number;
    excelParseMs: number;
    normalizationMs: number;
    periodFilteringMs: number;
    capabilityDetectionMs: number;
    indexConstructionMs: number;
    controlPlanningMs: number;
    resultSerializationMs: number;
    totalBackendMs: number;
  };
  controlExecution: { controlId: string; elapsedMs: number }[];
  sourceCount: number;
  normalizedRecordCount: number;
  cacheHits: number;
  cacheMisses: number;
  estimatedCacheBytes: number;
  peakMemoryBytes?: number;
  ipcInclusiveMs?: number;
  ipcRoundTripOverheadMs?: number;
  frontendRenderMs?: number;
}

export interface AuditWorkspaceReport {
  sessionId: string;
  accountingPeriod: AccountingPeriod;
  sourceCatalog: { sources: SourceCatalogEntry[] };
  controlPlans: ControlPlan[];
  controlResults: ControlResult[];
  sourceReuse: SourceReuseEvidence[];
  runStatus: AuditRunStatus;
  errors: AuditExecutionError[];
  metrics: AuditExecutionMetrics;
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

export type DatasetRelation =
  | "UNIQUE"
  | { EXACT_DUPLICATE: { originalSourceId: string; rawSha256: string } }
  | { CONTENT_DUPLICATE: { originalSourceId: string; contentFingerprint: string } }
  | { SUBSET_DUPLICATE: { supersetSourceId: string; recordCount: number; totalInSuperset: number } }
  | { PARTIAL_OVERLAP: { overlappingSourceId: string; overlapCount: number; overlapRatio: string } };

export interface IntakeSourceAnalysis {
  sourceId: string;
  sourceName: string;
  filePath: string;
  rawSha256?: string;
  canonicalContentFingerprint: string;
  totalRecords: number;
  relation: DatasetRelation;
  isEligibleForReconciliation: boolean;
  diagnosticMessage: string;
}

export interface IntakeAnalysisResult {
  totalPhysicalSources: number;
  uniqueDatasetsCount: number;
  exactDuplicatesCount: number;
  contentDuplicatesCount: number;
  subsetDuplicatesCount: number;
  partialOverlapsCount: number;
  logicalSourcesCount: number;
  sourceAnalyses: IntakeSourceAnalysis[];
  requiresUserConfirmation: boolean;
}

export interface ReconciliationResult {
  sessionId: string;
  executedAt: string;
  profileId: string;
  summary: ReconciliationSummary;
  groups: MatchGroup[];
  referenceControls?: ReferenceControlResult[];
  intakeAnalysis?: IntakeAnalysisResult;
}

export interface ReferenceControlResult {
  sourceId: string;
  sourceKind: DataSourceKind;
  recordCount: number;
  status: MatchStatus;
  message: string;
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
  toleranceVnd?: MoneyValue;
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
  defaultToleranceVnd: MoneyValue;
  defaultDateDays: number;
  enableAggregate: boolean;
}

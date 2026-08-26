use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    evaluate_partner_identity_cross_source, evaluate_partner_master_control,
    evaluate_sales_analysis_control, execute_reconciliation, AnalyticalRowLevel,
    AuditCancellationToken, AuditExecutionError, AuditExecutionMetrics, AuditRunStatus,
    CanonicalRecord, ComparisonRule, ComparisonSemantic, ControlTimingMetric, DataSource,
    DataSourceKind, DocumentIntegrityResult, DocumentIntegritySource, MatchStatus, PartnerRecord,
    ReconciliationResult, ReconciliationSession, SalesAnalysisRecord, SourceRole,
};

pub const REVENUE_CONTROL_ID: &str = "REVENUE_INVOICE_REGISTER_LEDGER";
pub const BANK_CONTROL_ID: &str = "BANK_LEDGER_RECONCILIATION";
pub const PARTNER_CONTROL_ID: &str = "PARTNER_IDENTITY";
pub const SALES_ANALYSIS_CONTROL_ID: &str = "SALES_ANALYSIS_CONTROL";
pub const VAT_CONTROL_ID: &str = "VAT_ACCOUNTING_CONTROL";
pub const RECEIVABLE_CONTROL_ID: &str = "RECEIVABLE_CONTROL";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountingPeriod {
    pub start_date: String,
    pub end_date: String,
}

impl AccountingPeriod {
    pub fn validate(&self) -> Result<(NaiveDate, NaiveDate), String> {
        let start = NaiveDate::parse_from_str(&self.start_date, "%Y-%m-%d")
            .map_err(|_| "INVALID_ACCOUNTING_PERIOD_START: expected YYYY-MM-DD".to_string())?;
        let end = NaiveDate::parse_from_str(&self.end_date, "%Y-%m-%d")
            .map_err(|_| "INVALID_ACCOUNTING_PERIOD_END: expected YYYY-MM-DD".to_string())?;
        if start > end {
            return Err("INVALID_ACCOUNTING_PERIOD_RANGE: start must not exceed end".to_string());
        }
        Ok((start, end))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceCapability {
    LedgerEntry { account: String },
    Invoice,
    BankTransaction,
    PartnerMaster,
    SalesTransaction,
    AnalyticalSalesReport,
}

impl SourceCapability {
    pub fn display_name(&self) -> String {
        match self {
            Self::LedgerEntry { account } => format!("LedgerEntry(account={account})"),
            Self::Invoice => "Invoice".to_string(),
            Self::BankTransaction => "BankTransaction".to_string(),
            Self::PartnerMaster => "PartnerMaster".to_string(),
            Self::SalesTransaction => "SalesTransaction".to_string(),
            Self::AnalyticalSalesReport => "AnalyticalSalesReport".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PeriodPolicy {
    RequiredTransactionDate,
    NotRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlRuleDefinition {
    pub id: String,
    pub semantic: ComparisonSemantic,
    pub primary_capability: SourceCapability,
    pub primary_field: String,
    pub secondary_capability: SourceCapability,
    pub secondary_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlDefinition {
    pub id: String,
    pub title: String,
    pub required_source_capabilities: Vec<SourceCapability>,
    pub optional_source_capabilities: Vec<SourceCapability>,
    pub reference_capabilities: Vec<SourceCapability>,
    pub period_policy: PeriodPolicy,
    pub rules: Vec<ControlRuleDefinition>,
    pub implemented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlPlanStatus {
    Ready,
    MissingSource,
    NeedsMapping,
    NeedsReview,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PeriodEvidence {
    pub earliest_date: Option<String>,
    pub latest_date: Option<String>,
    pub records_in_period: usize,
    pub records_outside_period: usize,
    pub missing_or_unparseable_dates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCatalogEntry {
    pub source_id: String,
    pub source_name: String,
    pub source_kind: DataSourceKind,
    pub capabilities: Vec<SourceCapability>,
    pub record_count: usize,
    pub mapping_complete: bool,
    pub period_evidence: PeriodEvidence,
    pub warnings: Vec<String>,
    pub provenance_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceCatalog {
    pub sources: Vec<SourceCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlPlan {
    pub control_id: String,
    pub title: String,
    pub status: ControlPlanStatus,
    pub source_ids: Vec<String>,
    pub missing_capabilities: Vec<SourceCapability>,
    pub warnings: Vec<String>,
    pub effective_period: Option<AccountingPeriod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlExecutionStatus {
    Pass,
    NeedsReview,
    NotRun,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlFinding {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlResult {
    pub control_id: String,
    pub status: ControlExecutionStatus,
    pub evidence: Vec<String>,
    pub findings: Vec<ControlFinding>,
    pub source_ids: Vec<String>,
    pub missing_capabilities: Vec<SourceCapability>,
    pub effective_period: Option<AccountingPeriod>,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub summary_metrics: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AuditExecutionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_result: Option<ReconciliationResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_integrity_result: Option<DocumentIntegrityResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceReuseEvidence {
    pub source_id: String,
    pub read_count: usize,
    pub normalize_count: usize,
    pub index_count: usize,
    pub normalized_record_count: usize,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditWorkspaceReport {
    pub session_id: String,
    pub accounting_period: AccountingPeriod,
    pub source_catalog: SourceCatalog,
    pub control_plans: Vec<ControlPlan>,
    pub control_results: Vec<ControlResult>,
    pub source_reuse: Vec<SourceReuseEvidence>,
    pub run_status: AuditRunStatus,
    pub errors: Vec<AuditExecutionError>,
    pub metrics: AuditExecutionMetrics,
}

#[derive(Debug, Clone)]
pub struct PreparedSourceIndex {
    pub records: Vec<CanonicalRecord>,
    /// Records with missing/unparseable required dates are retained for typed
    /// validation and review, but are never part of an auto-match index.
    pub review_records: Vec<CanonicalRecord>,
    pub by_document_number: HashMap<String, Vec<usize>>,
}

#[derive(Debug, Clone, Default)]
pub struct NormalizedAuditDatasets {
    pub transactional: HashMap<String, PreparedSourceIndex>,
    pub partner_masters: HashMap<String, Vec<PartnerRecord>>,
    pub sales_analysis: HashMap<String, Vec<SalesAnalysisRecord>>,
}

#[derive(Debug, Clone)]
pub struct AuditSession {
    pub session_id: String,
    pub accounting_period: AccountingPeriod,
    pub sources: Vec<DataSource>,
    pub source_catalog: SourceCatalog,
    pub normalized_datasets: NormalizedAuditDatasets,
    pub control_plans: Vec<ControlPlan>,
    pub source_reuse: Vec<SourceReuseEvidence>,
    pub metrics: AuditExecutionMetrics,
}

fn ledger_capability(account: &str) -> SourceCapability {
    SourceCapability::LedgerEntry {
        account: account.to_string(),
    }
}

pub fn control_definitions() -> Vec<ControlDefinition> {
    let invoice = SourceCapability::Invoice;
    let register = SourceCapability::SalesTransaction;
    let ledger511 = ledger_capability("511");
    let ledger112 = ledger_capability("112");
    vec![
        ControlDefinition {
            id: REVENUE_CONTROL_ID.to_string(),
            title: "Hóa đơn ↔ Bảng kê bán hàng ↔ Sổ cái doanh thu".to_string(),
            required_source_capabilities: vec![
                invoice.clone(),
                register.clone(),
                ledger511.clone(),
            ],
            optional_source_capabilities: vec![],
            reference_capabilities: vec![],
            period_policy: PeriodPolicy::RequiredTransactionDate,
            rules: vec![
                ControlRuleDefinition {
                    id: "register-revenue".to_string(),
                    semantic: ComparisonSemantic::Revenue,
                    primary_capability: invoice.clone(),
                    primary_field: "pretaxAmount".to_string(),
                    secondary_capability: register.clone(),
                    secondary_field: "pretaxAmount".to_string(),
                },
                ControlRuleDefinition {
                    id: "register-vat".to_string(),
                    semantic: ComparisonSemantic::Vat,
                    primary_capability: invoice.clone(),
                    primary_field: "vatAmount".to_string(),
                    secondary_capability: register.clone(),
                    secondary_field: "vatAmount".to_string(),
                },
                ControlRuleDefinition {
                    id: "register-receivable".to_string(),
                    semantic: ComparisonSemantic::Receivable,
                    primary_capability: invoice.clone(),
                    primary_field: "totalAmount".to_string(),
                    secondary_capability: register,
                    secondary_field: "totalAmount".to_string(),
                },
                ControlRuleDefinition {
                    id: "ledger-revenue".to_string(),
                    semantic: ComparisonSemantic::Revenue,
                    primary_capability: invoice,
                    primary_field: "pretaxAmount".to_string(),
                    secondary_capability: ledger511,
                    secondary_field: "creditAmount".to_string(),
                },
            ],
            implemented: true,
        },
        ControlDefinition {
            id: BANK_CONTROL_ID.to_string(),
            title: "Sổ tiền gửi ngân hàng ↔ Sao kê ngân hàng".to_string(),
            required_source_capabilities: vec![
                ledger112.clone(),
                SourceCapability::BankTransaction,
            ],
            optional_source_capabilities: vec![],
            reference_capabilities: vec![],
            period_policy: PeriodPolicy::RequiredTransactionDate,
            rules: vec![ControlRuleDefinition {
                id: "bank-payment".to_string(),
                semantic: ComparisonSemantic::BankPayment,
                primary_capability: ledger112,
                primary_field: "directionalAmount".to_string(),
                secondary_capability: SourceCapability::BankTransaction,
                secondary_field: "directionalAmount".to_string(),
            }],
            implemented: true,
        },
        ControlDefinition {
            id: PARTNER_CONTROL_ID.to_string(),
            title: "Kiểm soát định danh đối tác".to_string(),
            required_source_capabilities: vec![SourceCapability::PartnerMaster],
            optional_source_capabilities: vec![
                SourceCapability::Invoice,
                SourceCapability::SalesTransaction,
                ledger_capability("511"),
                ledger_capability("112"),
                ledger_capability("131"),
                ledger_capability("3331"),
            ],
            reference_capabilities: vec![SourceCapability::PartnerMaster],
            period_policy: PeriodPolicy::NotRequired,
            rules: vec![],
            implemented: true,
        },
        ControlDefinition {
            id: SALES_ANALYSIS_CONTROL_ID.to_string(),
            title: "Kiểm soát báo cáo phân tích bán hàng".to_string(),
            required_source_capabilities: vec![SourceCapability::AnalyticalSalesReport],
            optional_source_capabilities: vec![],
            reference_capabilities: vec![],
            period_policy: PeriodPolicy::NotRequired,
            rules: vec![],
            implemented: true,
        },
        ControlDefinition {
            id: VAT_CONTROL_ID.to_string(),
            title: "Kiểm soát kế toán thuế GTGT".to_string(),
            required_source_capabilities: vec![ledger_capability("3331")],
            optional_source_capabilities: vec![],
            reference_capabilities: vec![],
            period_policy: PeriodPolicy::RequiredTransactionDate,
            rules: vec![],
            implemented: false,
        },
        ControlDefinition {
            id: RECEIVABLE_CONTROL_ID.to_string(),
            title: "Kiểm soát công nợ phải thu".to_string(),
            required_source_capabilities: vec![ledger_capability("131")],
            optional_source_capabilities: vec![],
            reference_capabilities: vec![],
            period_policy: PeriodPolicy::RequiredTransactionDate,
            rules: vec![],
            implemented: false,
        },
    ]
}

fn account_capabilities(records: &[CanonicalRecord]) -> Vec<SourceCapability> {
    const SUPPORTED: [&str; 4] = ["511", "112", "131", "3331"];
    let mut accounts = HashSet::new();
    for record in records {
        for account in [
            record.debit_account.as_deref(),
            record.credit_account.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            let normalized: String = account.chars().filter(char::is_ascii_digit).collect();
            for prefix in SUPPORTED {
                if normalized.starts_with(prefix) {
                    accounts.insert(prefix.to_string());
                }
            }
        }
    }
    let mut capabilities: Vec<_> = accounts
        .into_iter()
        .map(|account| SourceCapability::LedgerEntry { account })
        .collect();
    capabilities.sort_by_key(SourceCapability::display_name);
    capabilities
}

pub fn capabilities_for_source(
    source: &DataSource,
    records: &[CanonicalRecord],
) -> Vec<SourceCapability> {
    let mut capabilities = match source.kind {
        DataSourceKind::EInvoice => vec![SourceCapability::Invoice],
        DataSourceKind::SalesRegister => vec![SourceCapability::SalesTransaction],
        DataSourceKind::BankStatement => vec![SourceCapability::BankTransaction],
        DataSourceKind::PartnerMaster => vec![SourceCapability::PartnerMaster],
        DataSourceKind::SalesAnalysisReport => vec![SourceCapability::AnalyticalSalesReport],
        DataSourceKind::Ledger511 => vec![ledger_capability("511")],
        DataSourceKind::Ledger112 => vec![ledger_capability("112")],
        DataSourceKind::Ledger131 => vec![ledger_capability("131")],
        DataSourceKind::Ledger3331 => vec![ledger_capability("3331")],
        _ => vec![],
    };
    if matches!(
        source.kind,
        DataSourceKind::Custom | DataSourceKind::BranchLedger | DataSourceKind::CashBook
    ) {
        for capability in account_capabilities(records) {
            if !capabilities.contains(&capability) {
                capabilities.push(capability);
            }
        }
    }
    capabilities
}

fn effective_record_date(record: &CanonicalRecord) -> Option<&str> {
    record
        .date
        .as_deref()
        .or(record.transaction_date.as_deref())
        .or(record.accounting_date.as_deref())
}

fn filter_period(
    records: Vec<CanonicalRecord>,
    start: NaiveDate,
    end: NaiveDate,
) -> (Vec<CanonicalRecord>, Vec<CanonicalRecord>, PeriodEvidence) {
    let mut evidence = PeriodEvidence::default();
    let mut in_period = Vec::new();
    let mut review_records = Vec::new();
    for record in records {
        let Some(raw_date) = effective_record_date(&record) else {
            evidence.missing_or_unparseable_dates += 1;
            review_records.push(record);
            continue;
        };
        let Ok(date) = NaiveDate::parse_from_str(raw_date, "%Y-%m-%d") else {
            evidence.missing_or_unparseable_dates += 1;
            review_records.push(record);
            continue;
        };
        let iso = date.format("%Y-%m-%d").to_string();
        if date >= start && date <= end {
            if evidence
                .earliest_date
                .as_ref()
                .is_none_or(|value| iso < *value)
            {
                evidence.earliest_date = Some(iso.clone());
            }
            if evidence
                .latest_date
                .as_ref()
                .is_none_or(|value| iso > *value)
            {
                evidence.latest_date = Some(iso);
            }
            evidence.records_in_period += 1;
            in_period.push(record);
        } else {
            evidence.records_outside_period += 1;
        }
    }
    (in_period, review_records, evidence)
}

fn build_index(
    records: Vec<CanonicalRecord>,
    review_records: Vec<CanonicalRecord>,
) -> PreparedSourceIndex {
    let mut by_document_number: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        if let Some(document) = record.doc_no.as_deref().filter(|value| !value.is_empty()) {
            by_document_number
                .entry(document.to_string())
                .or_default()
                .push(index);
        }
    }
    PreparedSourceIndex {
        records,
        review_records,
        by_document_number,
    }
}

fn source_for_capability<'a>(
    catalog: &'a SourceCatalog,
    capability: &SourceCapability,
) -> Option<&'a SourceCatalogEntry> {
    catalog
        .sources
        .iter()
        .find(|source| source.capabilities.contains(capability))
}

fn sources_for_capability<'a>(
    catalog: &'a SourceCatalog,
    capability: &SourceCapability,
) -> Vec<&'a SourceCatalogEntry> {
    catalog
        .sources
        .iter()
        .filter(|source| source.capabilities.contains(capability))
        .collect()
}

pub fn plan_controls(catalog: &SourceCatalog) -> Vec<ControlPlan> {
    control_definitions()
        .into_iter()
        .map(|definition| {
            let matched: Vec<_> = definition
                .required_source_capabilities
                .iter()
                .filter_map(|capability| source_for_capability(catalog, capability))
                .collect();
            let missing: Vec<_> = definition
                .required_source_capabilities
                .iter()
                .filter(|capability| source_for_capability(catalog, capability).is_none())
                .cloned()
                .collect();
            let mut planned_source_ids: Vec<String> = matched
                .iter()
                .map(|source| source.source_id.clone())
                .collect();
            for capability in definition
                .optional_source_capabilities
                .iter()
                .chain(definition.reference_capabilities.iter())
            {
                for source in sources_for_capability(catalog, capability) {
                    if !planned_source_ids.contains(&source.source_id) {
                        planned_source_ids.push(source.source_id.clone());
                    }
                }
            }
            let has_unmapped = catalog.sources.iter().any(|source| !source.mapping_complete);
            let duplicate_capabilities: Vec<_> = definition
                .required_source_capabilities
                .iter()
                .filter(|capability| sources_for_capability(catalog, capability).len() > 1)
                .cloned()
                .collect();
            let mut warnings = Vec::new();
            let mut effective_period = None;
            let status = if !missing.is_empty() {
                if has_unmapped {
                    warnings.push(
                        "Có nguồn chưa nhận dạng đủ capability; cần mapping trước khi kết luận thiếu."
                            .to_string(),
                    );
                    ControlPlanStatus::NeedsMapping
                } else {
                    ControlPlanStatus::MissingSource
                }
            } else if !definition.implemented {
                warnings.push("Control được khai báo cho tương lai, chưa triển khai trong V16.".to_string());
                ControlPlanStatus::NotApplicable
            } else if !duplicate_capabilities.is_empty() {
                warnings.push(format!(
                    "MULTIPLE_SOURCES_NEED_REVIEW: nhiều nguồn cùng capability {}.",
                    duplicate_capabilities
                        .iter()
                        .map(SourceCapability::display_name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                ControlPlanStatus::NeedsReview
            } else if definition.period_policy == PeriodPolicy::RequiredTransactionDate {
                let missing_dates: usize = matched
                    .iter()
                    .map(|source| source.period_evidence.missing_or_unparseable_dates)
                    .sum();
                let in_period_sources = matched
                    .iter()
                    .filter(|source| source.period_evidence.records_in_period > 0)
                    .count();
                if missing_dates > 0 {
                    let effective_start = matched
                        .iter()
                        .filter_map(|source| source.period_evidence.earliest_date.as_ref())
                        .max()
                        .cloned();
                    let effective_end = matched
                        .iter()
                        .filter_map(|source| source.period_evidence.latest_date.as_ref())
                        .min()
                        .cloned();
                    if let (Some(start), Some(end)) = (effective_start, effective_end) {
                        if start <= end {
                            effective_period = Some(AccountingPeriod {
                                start_date: start,
                                end_date: end,
                            });
                        }
                    }
                    warnings.push(format!(
                        "PERIOD_DATE_NEEDS_REVIEW: {missing_dates} record thiếu hoặc không đọc được ngày."
                    ));
                    ControlPlanStatus::NeedsReview
                } else if in_period_sources == 0 {
                    warnings.push("Không có record thuộc kỳ đã chọn.".to_string());
                    ControlPlanStatus::NotApplicable
                } else if in_period_sources != matched.len() {
                    warnings.push("Một nguồn bắt buộc không có record trong kỳ đã chọn.".to_string());
                    ControlPlanStatus::NeedsReview
                } else {
                    let effective_start = matched
                        .iter()
                        .filter_map(|source| source.period_evidence.earliest_date.as_ref())
                        .max()
                        .cloned();
                    let effective_end = matched
                        .iter()
                        .filter_map(|source| source.period_evidence.latest_date.as_ref())
                        .min()
                        .cloned();
                    match (effective_start, effective_end) {
                        (Some(start), Some(end)) if start <= end => {
                            effective_period = Some(AccountingPeriod {
                                start_date: start,
                                end_date: end,
                            });
                            ControlPlanStatus::Ready
                        }
                        _ => {
                            warnings.push(
                                "PERIOD_INTERSECTION_EMPTY: các nguồn không cùng kỳ; không auto-match."
                                    .to_string(),
                            );
                            ControlPlanStatus::NeedsReview
                        }
                    }
                }
            } else {
                ControlPlanStatus::Ready
            };
            ControlPlan {
                control_id: definition.id,
                title: definition.title,
                status,
                source_ids: planned_source_ids,
                missing_capabilities: missing,
                warnings,
                effective_period,
            }
        })
        .collect()
}

fn apply_authoritative_revenue_period(
    catalog: &SourceCatalog,
    accounting_period: &AccountingPeriod,
    plans: &mut [ControlPlan],
) {
    let required = [
        SourceCapability::Invoice,
        SourceCapability::SalesTransaction,
        ledger_capability("511"),
    ];
    if required
        .iter()
        .any(|capability| sources_for_capability(catalog, capability).len() != 1)
    {
        return;
    }
    let Some(plan) = plans
        .iter_mut()
        .find(|plan| plan.control_id == REVENUE_CONTROL_ID)
    else {
        return;
    };
    if !plan.missing_capabilities.is_empty() {
        return;
    }

    let matched = required
        .iter()
        .filter_map(|capability| source_for_capability(catalog, capability))
        .collect::<Vec<_>>();
    let missing_dates = matched
        .iter()
        .map(|source| source.period_evidence.missing_or_unparseable_dates)
        .sum::<usize>();
    let records_in_period = matched
        .iter()
        .map(|source| source.period_evidence.records_in_period)
        .sum::<usize>();

    // Source earliest/latest values are evidence only. They must never shrink
    // the user-selected accounting period for document completeness controls.
    plan.effective_period = Some(accounting_period.clone());
    plan.warnings.clear();
    for source in &matched {
        match (
            source.period_evidence.earliest_date.as_deref(),
            source.period_evidence.latest_date.as_deref(),
        ) {
            (Some(earliest), Some(latest)) => plan.warnings.push(format!(
                "SOURCE_PERIOD_EVIDENCE: {} có dữ liệu từ {} đến {}; boundary kiểm soát vẫn là kỳ người dùng chọn {} đến {}.",
                source.source_name,
                earliest,
                latest,
                accounting_period.start_date,
                accounting_period.end_date
            )),
            _ => plan.warnings.push(format!(
                "SOURCE_PERIOD_EVIDENCE: {} không có đủ date evidence trong kỳ; boundary kiểm soát vẫn là kỳ người dùng chọn.",
                source.source_name
            )),
        }
    }
    if missing_dates > 0 {
        plan.warnings.push(format!(
            "PERIOD_DATE_NEEDS_REVIEW: {missing_dates} record thiếu hoặc không đọc được ngày."
        ));
        plan.status = ControlPlanStatus::NeedsReview;
    } else if records_in_period == 0 {
        plan.status = ControlPlanStatus::NotApplicable;
    } else {
        // A required source with no rows in the selected period is executable
        // evidence of missing documents, not a reason to suppress the control.
        plan.status = ControlPlanStatus::Ready;
    }
}

pub fn prepare_audit_session(
    session_id: String,
    accounting_period: AccountingPeriod,
    sources: Vec<DataSource>,
    mut transactional_records: HashMap<String, Vec<CanonicalRecord>>,
    partner_masters: HashMap<String, Vec<PartnerRecord>>,
    sales_analysis: HashMap<String, Vec<SalesAnalysisRecord>>,
    provenance_sha256: HashMap<String, String>,
) -> Result<AuditSession, String> {
    let prepare_started = Instant::now();
    let (start, end) = accounting_period.validate()?;
    let mut metrics = AuditExecutionMetrics {
        source_count: sources.len(),
        ..Default::default()
    };
    let mut datasets = NormalizedAuditDatasets {
        partner_masters,
        sales_analysis,
        ..Default::default()
    };
    let mut catalog = SourceCatalog::default();
    let mut source_reuse = Vec::new();

    for source in &sources {
        let original_records = transactional_records.remove(&source.id).unwrap_or_default();
        let capability_started = Instant::now();
        let capabilities = capabilities_for_source(source, &original_records);
        metrics.stages.capability_detection_ms = metrics
            .stages
            .capability_detection_ms
            .saturating_add(capability_started.elapsed().as_millis() as u64);
        let record_count = if source.kind == DataSourceKind::PartnerMaster {
            datasets.partner_masters.get(&source.id).map_or(0, Vec::len)
        } else if source.kind == DataSourceKind::SalesAnalysisReport {
            datasets.sales_analysis.get(&source.id).map_or(0, Vec::len)
        } else {
            original_records.len()
        };
        let is_transactional = !original_records.is_empty()
            || !matches!(
                source.kind,
                DataSourceKind::PartnerMaster | DataSourceKind::SalesAnalysisReport
            );
        let (prepared, period_evidence) = if is_transactional {
            let filtering_started = Instant::now();
            let (filtered, review_records, evidence) = filter_period(original_records, start, end);
            metrics.stages.period_filtering_ms = metrics
                .stages
                .period_filtering_ms
                .saturating_add(filtering_started.elapsed().as_millis() as u64);
            let indexing_started = Instant::now();
            let prepared = build_index(filtered, review_records);
            metrics.stages.index_construction_ms = metrics
                .stages
                .index_construction_ms
                .saturating_add(indexing_started.elapsed().as_millis() as u64);
            (prepared, evidence)
        } else {
            (
                PreparedSourceIndex {
                    records: vec![],
                    review_records: vec![],
                    by_document_number: HashMap::new(),
                },
                PeriodEvidence::default(),
            )
        };
        let indexed_count = prepared.records.len();
        if is_transactional {
            datasets.transactional.insert(source.id.clone(), prepared);
        }
        let mapping_complete = !capabilities.is_empty();
        let mut warnings = Vec::new();
        if !mapping_complete {
            warnings.push("NEEDS_MAPPING: chưa xác định được capability nghiệp vụ.".to_string());
        }
        if period_evidence.records_outside_period > 0 {
            warnings.push(format!(
                "Đã loại {} record ngoài kỳ trước khi matching.",
                period_evidence.records_outside_period
            ));
        }
        if period_evidence.missing_or_unparseable_dates > 0 {
            warnings.push(format!(
                "{} record thiếu/không đọc được ngày; không được auto-match.",
                period_evidence.missing_or_unparseable_dates
            ));
        }
        catalog.sources.push(SourceCatalogEntry {
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            source_kind: source.kind.clone(),
            capabilities,
            record_count,
            mapping_complete,
            period_evidence,
            warnings,
            provenance_sha256: provenance_sha256.get(&source.id).cloned(),
        });
        source_reuse.push(SourceReuseEvidence {
            source_id: source.id.clone(),
            read_count: 1,
            normalize_count: 1,
            index_count: 1,
            normalized_record_count: if is_transactional {
                indexed_count
            } else {
                record_count
            },
            cache_hit: false,
        });
    }
    metrics.normalized_record_count = source_reuse
        .iter()
        .map(|source| source.normalized_record_count)
        .sum();
    let planning_started = Instant::now();
    let mut control_plans = plan_controls(&catalog);
    apply_authoritative_revenue_period(&catalog, &accounting_period, &mut control_plans);
    metrics.stages.control_planning_ms = planning_started.elapsed().as_millis() as u64;
    metrics.stages.total_backend_ms = prepare_started.elapsed().as_millis() as u64;
    Ok(AuditSession {
        session_id,
        accounting_period,
        sources,
        source_catalog: catalog,
        normalized_datasets: datasets,
        control_plans,
        source_reuse,
        metrics,
    })
}

fn source_by_id<'a>(session: &'a AuditSession, id: &str) -> Result<&'a DataSource, String> {
    session
        .sources
        .iter()
        .find(|source| source.id == id)
        .ok_or_else(|| format!("SOURCE_NOT_FOUND: {id}"))
}

fn execution_kind_for_capability(
    capability: &SourceCapability,
    fallback: &DataSourceKind,
) -> DataSourceKind {
    match capability {
        SourceCapability::Invoice => DataSourceKind::EInvoice,
        SourceCapability::SalesTransaction => DataSourceKind::SalesRegister,
        SourceCapability::BankTransaction => DataSourceKind::BankStatement,
        SourceCapability::PartnerMaster => DataSourceKind::PartnerMaster,
        SourceCapability::AnalyticalSalesReport => DataSourceKind::SalesAnalysisReport,
        SourceCapability::LedgerEntry { account } if account == "511" => DataSourceKind::Ledger511,
        SourceCapability::LedgerEntry { account } if account == "112" => DataSourceKind::Ledger112,
        SourceCapability::LedgerEntry { account } if account == "131" => DataSourceKind::Ledger131,
        SourceCapability::LedgerEntry { account } if account == "3331" => {
            DataSourceKind::Ledger3331
        }
        SourceCapability::LedgerEntry { .. } => fallback.clone(),
    }
}

fn record_belongs_to_capability(
    record: &CanonicalRecord,
    capability: &SourceCapability,
    source_kind: &DataSourceKind,
) -> bool {
    let SourceCapability::LedgerEntry { account } = capability else {
        return true;
    };
    let legacy_account = match source_kind {
        DataSourceKind::Ledger511 => Some("511"),
        DataSourceKind::Ledger112 => Some("112"),
        DataSourceKind::Ledger131 => Some("131"),
        DataSourceKind::Ledger3331 => Some("3331"),
        _ => None,
    };
    if legacy_account == Some(account.as_str()) {
        return true;
    }
    [
        record.debit_account.as_deref(),
        record.credit_account.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(|value| {
        value
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
    })
    .any(|value| value.starts_with(account))
}

fn compile_transaction_control(
    audit: &AuditSession,
    definition: &ControlDefinition,
    plan: &ControlPlan,
    tolerance_vnd: Decimal,
    date_tolerance_days: u32,
) -> Result<ReconciliationResult, String> {
    let primary_capability = definition
        .rules
        .first()
        .map(|rule| &rule.primary_capability)
        .ok_or_else(|| format!("CONTROL_RULES_MISSING: {}", definition.id))?;
    let primary_entry = source_for_capability(&audit.source_catalog, primary_capability)
        .ok_or_else(|| format!("PRIMARY_CAPABILITY_MISSING: {}", definition.id))?;
    let primary = source_by_id(audit, &primary_entry.source_id)?;
    let execution_kinds: HashMap<String, DataSourceKind> = definition
        .required_source_capabilities
        .iter()
        .filter_map(|capability| {
            source_for_capability(&audit.source_catalog, capability).map(|entry| {
                (
                    entry.source_id.clone(),
                    execution_kind_for_capability(capability, &entry.source_kind),
                )
            })
        })
        .collect();
    let execution_capabilities: HashMap<String, SourceCapability> = definition
        .required_source_capabilities
        .iter()
        .filter_map(|capability| {
            source_for_capability(&audit.source_catalog, capability)
                .map(|entry| (entry.source_id.clone(), capability.clone()))
        })
        .collect();
    let sources: Vec<DataSource> = plan
        .source_ids
        .iter()
        .map(|id| {
            source_by_id(audit, id).cloned().map(|mut source| {
                if let Some(kind) = execution_kinds.get(id) {
                    source.kind = kind.clone();
                }
                source
            })
        })
        .collect::<Result<_, _>>()?;
    let rules = definition
        .rules
        .iter()
        .map(|rule| {
            let primary_source =
                source_for_capability(&audit.source_catalog, &rule.primary_capability)
                    .ok_or_else(|| format!("PRIMARY_CAPABILITY_MISSING: {}", rule.id))?;
            let secondary_source =
                source_for_capability(&audit.source_catalog, &rule.secondary_capability)
                    .ok_or_else(|| format!("SECONDARY_CAPABILITY_MISSING: {}", rule.id))?;
            Ok(ComparisonRule {
                id: rule.id.clone(),
                name: rule.id.clone(),
                semantic: rule.semantic,
                primary_source_kind: execution_kind_for_capability(
                    &rule.primary_capability,
                    &primary_source.source_kind,
                ),
                primary_field: rule.primary_field.clone(),
                secondary_source_kind: execution_kind_for_capability(
                    &rule.secondary_capability,
                    &secondary_source.source_kind,
                ),
                secondary_field: rule.secondary_field.clone(),
                is_required: true,
                tolerance_vnd,
                date_tolerance_days,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let required_source_ids = sources.iter().map(|source| source.id.clone()).collect();
    let reconciliation_session = ReconciliationSession {
        session_id: format!("{}:{}", audit.session_id, definition.id),
        scenario_name: definition.id.clone(),
        primary_source_id: Some(primary.id.clone()),
        expected_primary_kind: Some(execution_kind_for_capability(
            primary_capability,
            &primary.kind,
        )),
        required_source_ids: Some(required_source_ids),
        optional_source_ids: None,
        data_sources: sources
            .into_iter()
            .map(|mut source| {
                source.role = if source.id == primary.id {
                    SourceRole::Primary
                } else {
                    SourceRole::RequiredSecondary
                };
                source
            })
            .collect(),
        comparison_rules: rules,
        matching_tolerance_vnd: tolerance_vnd,
        date_tolerance_days,
        enable_aggregate_match: false,
    };
    let effective_period = plan
        .effective_period
        .as_ref()
        .ok_or_else(|| format!("EFFECTIVE_PERIOD_MISSING: {}", definition.id))?;
    let (effective_start, effective_end) = effective_period.validate()?;
    let records: HashMap<String, Vec<CanonicalRecord>> = audit
        .normalized_datasets
        .transactional
        .iter()
        .filter(|(id, _)| plan.source_ids.contains(id))
        .map(|(id, prepared)| {
            let capability = execution_capabilities.get(id);
            let source_kind = audit
                .source_catalog
                .sources
                .iter()
                .find(|source| source.source_id == *id)
                .map(|source| &source.source_kind);
            let records = prepared
                .records
                .iter()
                .filter(|record| {
                    let period_matches = effective_record_date(record)
                        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                        .is_some_and(|date| date >= effective_start && date <= effective_end);
                    let capability_matches = match (capability, source_kind) {
                        (Some(capability), Some(kind)) => {
                            record_belongs_to_capability(record, capability, kind)
                        }
                        _ => true,
                    };
                    period_matches && capability_matches
                })
                .cloned()
                .collect();
            (id.clone(), records)
        })
        .collect();
    execute_reconciliation(&reconciliation_session, &records)
}

fn reconciliation_control_result(
    definition: &ControlDefinition,
    plan: &ControlPlan,
    result: ReconciliationResult,
) -> ControlResult {
    let summary_metrics = reconciliation_summary_metrics(definition, &result);
    let limitations = if definition.id == BANK_CONTROL_ID {
        vec!["TK112_RUNNING_BALANCE_NOT_VERIFIED".to_string()]
    } else {
        vec![]
    };
    let review_groups: Vec<_> = result
        .groups
        .iter()
        .filter(|group| {
            !matches!(
                group.status,
                MatchStatus::MatchedExact
                    | MatchStatus::MatchedWithTolerance
                    | MatchStatus::MatchedAggregate
            )
        })
        .collect();
    let findings = review_groups
        .iter()
        .map(|group| {
            let high = group
                .discrepancies
                .iter()
                .any(|item| item.message.contains("HIGH"));
            ControlFinding {
                code: format!("{:?}", group.status).to_ascii_uppercase(),
                severity: if high { "HIGH" } else { "MEDIUM" }.to_string(),
                message: group
                    .discrepancies
                    .first()
                    .map(|item| item.message.clone())
                    .unwrap_or_else(|| "Bản ghi cần kiểm tra thủ công.".to_string()),
            }
        })
        .collect();
    ControlResult {
        control_id: definition.id.clone(),
        status: if review_groups.is_empty() {
            ControlExecutionStatus::Pass
        } else {
            ControlExecutionStatus::NeedsReview
        },
        evidence: vec![
            format!("records_primary={}", result.summary.total_source_records),
            format!("records_secondary={}", result.summary.total_target_records),
            format!("exact_matches={}", result.summary.exact_matches_count),
        ],
        findings,
        source_ids: plan.source_ids.clone(),
        missing_capabilities: vec![],
        effective_period: plan.effective_period.clone(),
        elapsed_ms: 0,
        summary_metrics,
        limitations,
        error: None,
        reconciliation_result: Some(result),
        document_integrity_result: None,
    }
}

fn document_records_for_capability<'a>(
    audit: &'a AuditSession,
    plan: &ControlPlan,
    capability: &SourceCapability,
) -> Result<(&'a DataSource, Cow<'a, [CanonicalRecord]>), String> {
    let catalog_entry = source_for_capability(&audit.source_catalog, capability)
        .ok_or_else(|| format!("CAPABILITY_MISSING: {}", capability.display_name()))?;
    if !plan.source_ids.contains(&catalog_entry.source_id) {
        return Err(format!(
            "CONTROL_SOURCE_NOT_PLANNED: {}",
            catalog_entry.source_id
        ));
    }
    let source = source_by_id(audit, &catalog_entry.source_id)?;
    let prepared = audit
        .normalized_datasets
        .transactional
        .get(&catalog_entry.source_id)
        .ok_or_else(|| format!("TRANSACTION_DATASET_MISSING: {}", catalog_entry.source_id))?;
    // `PreparedSourceIndex.records` was already filtered once by the
    // authoritative session period. Borrow the full prepared slice whenever
    // the source capability owns every record, avoiding a second 100k clone.
    let capability_owns_all = prepared
        .records
        .iter()
        .all(|record| record_belongs_to_capability(record, capability, &source.kind));
    if capability_owns_all && prepared.review_records.is_empty() {
        return Ok((source, Cow::Borrowed(&prepared.records)));
    }
    let mut records: Vec<CanonicalRecord> = prepared
        .records
        .iter()
        .filter(|record| record_belongs_to_capability(record, capability, &source.kind))
        .cloned()
        .collect();
    // These records failed the period-date boundary and therefore can only be
    // validated/reviewed. They are deliberately absent from every matcher
    // index, but V18 still reports their field-level provenance.
    records.extend(
        prepared
            .review_records
            .iter()
            .filter(|record| record_belongs_to_capability(record, capability, &source.kind))
            .cloned(),
    );
    Ok((source, Cow::Owned(records)))
}

fn document_integrity_control_result(
    definition: &ControlDefinition,
    plan: &ControlPlan,
    audit: &AuditSession,
) -> Result<ControlResult, String> {
    let (invoice_source, invoices) =
        document_records_for_capability(audit, plan, &SourceCapability::Invoice)?;
    let (register_source, registers) =
        document_records_for_capability(audit, plan, &SourceCapability::SalesTransaction)?;
    let ledger_capability = ledger_capability("511");
    let (ledger_source, ledger) = document_records_for_capability(audit, plan, &ledger_capability)?;
    let document_result = crate::evaluate_document_integrity(
        DocumentIntegritySource {
            source: invoice_source,
            records: &invoices,
        },
        DocumentIntegritySource {
            source: register_source,
            records: &registers,
        },
        DocumentIntegritySource {
            source: ledger_source,
            records: &ledger,
        },
    );
    let findings = document_result
        .documents
        .iter()
        .flat_map(|document| {
            document.errors.iter().map(move |item| ControlFinding {
                code: item.code.as_str().to_string(),
                severity: item.severity.clone(),
                message: format!("{} [{}]", item.message, document.id),
            })
        })
        .collect();
    let summary = &document_result.summary;
    let summary_metrics = BTreeMap::from([
        ("invoiceRecords".to_string(), summary.invoice_records as u64),
        (
            "salesRegisterRecords".to_string(),
            summary.sales_register_records as u64,
        ),
        (
            "ledger511Records".to_string(),
            summary.ledger_511_records as u64,
        ),
        ("fullyMatched".to_string(), summary.fully_matched as u64),
        ("dateMismatch".to_string(), summary.date_mismatch as u64),
        (
            "invoiceNumberMismatch".to_string(),
            summary.invoice_number_mismatch as u64,
        ),
        ("pretaxMismatch".to_string(), summary.pretax_mismatch as u64),
        ("vatMismatch".to_string(), summary.vat_mismatch as u64),
        ("totalMismatch".to_string(), summary.total_mismatch as u64),
        ("missingInBk".to_string(), summary.missing_in_bk as u64),
        ("extraInBk".to_string(), summary.extra_in_bk as u64),
        (
            "duplicateInvoiceNumber".to_string(),
            summary.duplicate_invoice_number as u64,
        ),
        ("ambiguousMatch".to_string(), summary.ambiguous_match as u64),
        ("invalidDate".to_string(), summary.invalid_date as u64),
        ("invalidAmount".to_string(), summary.invalid_amount as u64),
        (
            "missingInvoiceNumber".to_string(),
            summary.missing_invoice_number as u64,
        ),
        (
            "missingInTk511".to_string(),
            summary.missing_in_tk511 as u64,
        ),
        (
            "invoiceLifecycleNeedsReview".to_string(),
            summary.invoice_lifecycle_needs_review as u64,
        ),
    ]);
    Ok(ControlResult {
        control_id: definition.id.clone(),
        status: if document_result.documents_pass {
            ControlExecutionStatus::Pass
        } else {
            ControlExecutionStatus::NeedsReview
        },
        evidence: vec![
            format!("documents_pass={}", document_result.documents_pass),
            format!("totals_equal={}", document_result.totals_equal),
            format!("fully_matched={}", summary.fully_matched),
            "legacy_reconciliation_materialized=false".to_string(),
        ],
        findings,
        source_ids: plan.source_ids.clone(),
        missing_capabilities: vec![],
        effective_period: plan.effective_period.clone(),
        elapsed_ms: 0,
        summary_metrics,
        limitations: vec![
            "TK511_VAT_NOT_CHECKED".to_string(),
            "TK511_RECEIVABLE_NOT_CHECKED".to_string(),
        ],
        error: None,
        reconciliation_result: None,
        document_integrity_result: Some(document_result),
    })
}

fn reconciliation_summary_metrics(
    definition: &ControlDefinition,
    result: &ReconciliationResult,
) -> BTreeMap<String, u64> {
    let mut metrics = BTreeMap::from([
        (
            "primaryRecords".to_string(),
            result.summary.total_source_records as u64,
        ),
        (
            "secondaryRecords".to_string(),
            result.summary.total_target_records as u64,
        ),
        (
            "exactMatches".to_string(),
            result.summary.exact_matches_count as u64,
        ),
        (
            "toleranceMatches".to_string(),
            result.summary.tolerance_matches_count as u64,
        ),
        (
            "aggregateMatches".to_string(),
            result.summary.aggregate_matches_count as u64,
        ),
        (
            "needsReview".to_string(),
            (result.summary.mismatches_count
                + result.summary.duplicates_count
                + result.summary.ambiguous_count
                + result.summary.needs_review_count) as u64,
        ),
        (
            "missingInTarget".to_string(),
            result.summary.missing_in_target_count as u64,
        ),
        (
            "missingInSource".to_string(),
            result.summary.missing_in_source_count as u64,
        ),
    ]);

    if definition.id == BANK_CONTROL_ID {
        let mut accepted_secondary_ids = HashSet::new();
        let mut suggested_secondary_ids = HashSet::new();
        let mut ambiguous_secondary_ids = HashSet::new();
        let mut true_bank_only_ids = HashSet::new();
        let mut true_ledger_only_ids = HashSet::new();
        let mut direction_conflict = 0_u64;
        let mut insufficient_evidence = 0_u64;

        for group in &result.groups {
            let messages = group
                .discrepancies
                .iter()
                .map(|item| item.message.as_str())
                .collect::<Vec<_>>();
            if matches!(
                group.status,
                MatchStatus::MatchedExact
                    | MatchStatus::MatchedWithTolerance
                    | MatchStatus::MatchedAggregate
            ) {
                accepted_secondary_ids.extend(group.target_source_record_ids.iter().cloned());
            }
            if messages
                .iter()
                .any(|message| message.contains("SUGGESTED_DIRECTION_AMOUNT_DATE"))
            {
                suggested_secondary_ids.extend(group.target_source_record_ids.iter().cloned());
            }
            if messages
                .iter()
                .any(|message| message.contains("AMBIGUOUS_BANK_CANDIDATES"))
            {
                ambiguous_secondary_ids.extend(group.target_source_record_ids.iter().cloned());
            }
            if group.status == MatchStatus::UnmatchedMissingInSource {
                true_bank_only_ids.extend(group.target_source_record_ids.iter().cloned());
            }
            if group.status == MatchStatus::UnmatchedMissingInTarget {
                true_ledger_only_ids.extend(group.primary_source_record_ids.iter().cloned());
            }
            direction_conflict += messages
                .iter()
                .filter(|message| message.contains("DIRECTION_CONFLICT"))
                .count() as u64;
            insufficient_evidence += messages
                .iter()
                .filter(|message| message.contains("INSUFFICIENT_BANK_EVIDENCE"))
                .count() as u64;
        }

        suggested_secondary_ids.retain(|id| {
            !accepted_secondary_ids.contains(id) && !ambiguous_secondary_ids.contains(id)
        });
        ambiguous_secondary_ids.retain(|id| !accepted_secondary_ids.contains(id));
        true_bank_only_ids.retain(|id| {
            !accepted_secondary_ids.contains(id)
                && !suggested_secondary_ids.contains(id)
                && !ambiguous_secondary_ids.contains(id)
        });

        metrics.extend([
            (
                "strongAccepted".to_string(),
                accepted_secondary_ids.len() as u64,
            ),
            (
                "suggestedReviewLinked".to_string(),
                suggested_secondary_ids.len() as u64,
            ),
            (
                "ambiguousReviewLinked".to_string(),
                ambiguous_secondary_ids.len() as u64,
            ),
            ("trueBankOnly".to_string(), true_bank_only_ids.len() as u64),
            (
                "trueLedgerOnly".to_string(),
                true_ledger_only_ids.len() as u64,
            ),
            ("directionConflict".to_string(), direction_conflict),
            ("insufficientEvidence".to_string(), insufficient_evidence),
        ]);
    }
    metrics
}

fn not_run_result(plan: &ControlPlan) -> ControlResult {
    ControlResult {
        control_id: plan.control_id.clone(),
        status: ControlExecutionStatus::NotRun,
        evidence: plan.warnings.clone(),
        findings: vec![],
        source_ids: plan.source_ids.clone(),
        missing_capabilities: plan.missing_capabilities.clone(),
        effective_period: plan.effective_period.clone(),
        elapsed_ms: 0,
        summary_metrics: BTreeMap::new(),
        limitations: vec![],
        error: None,
        reconciliation_result: None,
        document_integrity_result: None,
    }
}

fn failed_result(plan: &ControlPlan, error: AuditExecutionError, elapsed_ms: u64) -> ControlResult {
    ControlResult {
        control_id: plan.control_id.clone(),
        status: ControlExecutionStatus::Failed,
        evidence: vec![],
        findings: vec![ControlFinding {
            code: format!("{:?}", error.code).to_ascii_uppercase(),
            severity: "HIGH".to_string(),
            message: error.safe_user_message.to_string(),
        }],
        source_ids: plan.source_ids.clone(),
        missing_capabilities: plan.missing_capabilities.clone(),
        effective_period: plan.effective_period.clone(),
        elapsed_ms,
        summary_metrics: BTreeMap::new(),
        limitations: vec![],
        error: Some(error),
        reconciliation_result: None,
        document_integrity_result: None,
    }
}

fn cancelled_result(plan: &ControlPlan) -> ControlResult {
    let error = AuditExecutionError::cancelled();
    ControlResult {
        control_id: plan.control_id.clone(),
        status: ControlExecutionStatus::Cancelled,
        evidence: vec![],
        findings: vec![],
        source_ids: plan.source_ids.clone(),
        missing_capabilities: plan.missing_capabilities.clone(),
        effective_period: plan.effective_period.clone(),
        elapsed_ms: 0,
        summary_metrics: BTreeMap::new(),
        limitations: vec![],
        error: Some(error),
        reconciliation_result: None,
        document_integrity_result: None,
    }
}

pub fn execute_audit_session(
    audit: AuditSession,
    tolerance_vnd: Decimal,
    date_tolerance_days: u32,
) -> Result<AuditWorkspaceReport, String> {
    execute_audit_session_with_cancellation(
        audit,
        tolerance_vnd,
        date_tolerance_days,
        &AuditCancellationToken::default(),
    )
}

pub fn execute_audit_session_with_cancellation(
    audit: AuditSession,
    tolerance_vnd: Decimal,
    date_tolerance_days: u32,
    cancellation: &AuditCancellationToken,
) -> Result<AuditWorkspaceReport, String> {
    let execution_started = Instant::now();
    let definitions: HashMap<_, _> = control_definitions()
        .into_iter()
        .map(|definition| (definition.id.clone(), definition))
        .collect();
    let mut control_results = Vec::new();
    let mut errors = Vec::new();
    let mut metrics = audit.metrics.clone();
    let mut was_cancelled = false;
    for plan in &audit.control_plans {
        if cancellation.is_cancelled() {
            was_cancelled = true;
            if plan.status == ControlPlanStatus::Ready {
                control_results.push(cancelled_result(plan));
            } else {
                control_results.push(not_run_result(plan));
            }
            continue;
        }
        let revenue_date_validation_run = plan.control_id == REVENUE_CONTROL_ID
            && plan.status == ControlPlanStatus::NeedsReview
            && plan.missing_capabilities.is_empty()
            && plan.effective_period.is_some()
            && plan
                .warnings
                .iter()
                .any(|warning| warning.contains("PERIOD_DATE_NEEDS_REVIEW"));
        if plan.status != ControlPlanStatus::Ready && !revenue_date_validation_run {
            control_results.push(not_run_result(plan));
            continue;
        }
        let mut executable_plan = plan.clone();
        if revenue_date_validation_run && executable_plan.effective_period.is_none() {
            executable_plan.effective_period = Some(audit.accounting_period.clone());
        }
        let executable_plan = &executable_plan;
        let control_started = Instant::now();
        let result: Result<ControlResult, String> = (|| {
            let definition = definitions
                .get(&plan.control_id)
                .ok_or_else(|| format!("CONTROL_DEFINITION_NOT_FOUND: {}", plan.control_id))?;
            match executable_plan.control_id.as_str() {
                REVENUE_CONTROL_ID => {
                    document_integrity_control_result(definition, executable_plan, &audit)
                }
                BANK_CONTROL_ID => compile_transaction_control(
                    &audit,
                    definition,
                    executable_plan,
                    tolerance_vnd,
                    date_tolerance_days,
                )
                .map(|result| reconciliation_control_result(definition, plan, result)),
                PARTNER_CONTROL_ID => {
                    let master_id = plan
                        .source_ids
                        .first()
                        .ok_or_else(|| "PARTNER_MASTER_MISSING".to_string())?;
                    let masters = audit
                        .normalized_datasets
                        .partner_masters
                        .get(master_id)
                        .ok_or_else(|| "PARTNER_MASTER_DATASET_MISSING".to_string())?;
                    let compatible_records = audit
                        .source_catalog
                        .sources
                        .iter()
                        .filter(|source| {
                            source.capabilities.contains(&SourceCapability::Invoice)
                                || source
                                    .capabilities
                                    .contains(&SourceCapability::SalesTransaction)
                                || source.capabilities.iter().any(|capability| {
                                    matches!(capability, SourceCapability::LedgerEntry { .. })
                                })
                        })
                        .filter_map(|source| {
                            audit
                                .normalized_datasets
                                .transactional
                                .get(&source.source_id)
                        })
                        .flat_map(|prepared| prepared.records.iter());
                    let control = if audit
                        .normalized_datasets
                        .transactional
                        .values()
                        .all(|prepared| prepared.records.is_empty())
                    {
                        evaluate_partner_master_control(master_id.clone(), masters)
                    } else {
                        evaluate_partner_identity_cross_source(
                            master_id.clone(),
                            masters,
                            compatible_records,
                        )
                    };
                    Ok(ControlResult {
                        control_id: definition.id.clone(),
                        status: if control.status == MatchStatus::MatchedExact {
                            ControlExecutionStatus::Pass
                        } else {
                            ControlExecutionStatus::NeedsReview
                        },
                        evidence: vec![format!("records_checked={}", control.record_count)],
                        findings: if control.status == MatchStatus::MatchedExact {
                            vec![]
                        } else {
                            vec![ControlFinding {
                                code: "PARTNER_IDENTITY_NEEDS_REVIEW".to_string(),
                                severity: "MEDIUM".to_string(),
                                message: control.message,
                            }]
                        },
                        source_ids: plan.source_ids.clone(),
                        missing_capabilities: vec![],
                        effective_period: plan.effective_period.clone(),
                        elapsed_ms: 0,
                        summary_metrics: BTreeMap::from([(
                            "recordsChecked".to_string(),
                            control.record_count as u64,
                        )]),
                        limitations: vec!["NO_FUZZY_AUTO_MERGE".to_string()],
                        error: None,
                        reconciliation_result: None,
                        document_integrity_result: None,
                    })
                }
                SALES_ANALYSIS_CONTROL_ID => {
                    let source_id = plan
                        .source_ids
                        .first()
                        .ok_or_else(|| "SALES_ANALYSIS_SOURCE_MISSING".to_string())?;
                    let records = audit
                        .normalized_datasets
                        .sales_analysis
                        .get(source_id)
                        .ok_or_else(|| "SALES_ANALYSIS_DATASET_MISSING".to_string())?;
                    let control = evaluate_sales_analysis_control(source_id.clone(), records);
                    let group_count = records
                        .iter()
                        .filter(|row| row.row_level == AnalyticalRowLevel::Group)
                        .count();
                    let detail_count = records
                        .iter()
                        .filter(|row| row.row_level == AnalyticalRowLevel::Detail)
                        .count();
                    Ok(ControlResult {
                        control_id: definition.id.clone(),
                        status: if control.status == MatchStatus::MatchedExact {
                            ControlExecutionStatus::Pass
                        } else {
                            ControlExecutionStatus::NeedsReview
                        },
                        evidence: vec![
                            format!("group_records={group_count}"),
                            format!("detail_records={detail_count}"),
                        ],
                        findings: if control.status == MatchStatus::MatchedExact {
                            vec![]
                        } else {
                            vec![ControlFinding {
                                code: "SALES_ANALYSIS_NEEDS_REVIEW".to_string(),
                                severity: "MEDIUM".to_string(),
                                message: control.message,
                            }]
                        },
                        source_ids: plan.source_ids.clone(),
                        missing_capabilities: vec![],
                        effective_period: plan.effective_period.clone(),
                        elapsed_ms: 0,
                        summary_metrics: BTreeMap::from([
                            ("groupRecords".to_string(), group_count as u64),
                            ("detailRecords".to_string(), detail_count as u64),
                        ]),
                        limitations: vec!["GROUP_ROWS_EXCLUDED_FROM_DETAIL_TOTALS".to_string()],
                        error: None,
                        reconciliation_result: None,
                        document_integrity_result: None,
                    })
                }
                _ => Ok(not_run_result(plan)),
            }
        })();
        let elapsed_ms = control_started.elapsed().as_millis() as u64;
        metrics.control_execution.push(ControlTimingMetric {
            control_id: plan.control_id.clone(),
            elapsed_ms,
        });
        match result {
            Ok(mut result) => {
                result.elapsed_ms = elapsed_ms;
                control_results.push(result);
            }
            Err(detail) => {
                let error = AuditExecutionError::control(plan.control_id.clone(), detail);
                control_results.push(failed_result(plan, error.clone(), elapsed_ms));
                errors.push(error);
            }
        };
    }
    metrics.stages.total_backend_ms = metrics
        .stages
        .total_backend_ms
        .saturating_add(execution_started.elapsed().as_millis() as u64);
    let run_status = if was_cancelled {
        AuditRunStatus::Cancelled
    } else if errors.is_empty() {
        AuditRunStatus::Completed
    } else if control_results.iter().any(|result| {
        matches!(
            result.status,
            ControlExecutionStatus::Pass | ControlExecutionStatus::NeedsReview
        )
    }) {
        AuditRunStatus::Partial
    } else {
        AuditRunStatus::Failed
    };
    Ok(AuditWorkspaceReport {
        session_id: audit.session_id,
        accounting_period: audit.accounting_period,
        source_catalog: audit.source_catalog,
        control_plans: audit.control_plans,
        control_results,
        source_reuse: audit.source_reuse,
        run_status,
        errors,
        metrics,
    })
}

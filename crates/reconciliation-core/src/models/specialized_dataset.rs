//! Typed normalized records for non-transactional sources.
//!
//! These records deliberately do not enter the transaction matcher. They retain
//! provenance and source semantics without forcing master data or analytical
//! report rows into `CanonicalRecord`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{CanonicalRecord, ColumnMapping, DataSourceKind, MatchStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceLifecycle {
    Standard,
    Adjusted,
    Replaced,
    Cancelled,
    Unknown,
}

impl InvoiceLifecycle {
    pub fn from_status(status: Option<&str>) -> Self {
        let Some(status) = status else {
            return Self::Unknown;
        };
        let normalized =
            crate::reader::header_detector::remove_diacritics(status).to_ascii_lowercase();
        if normalized.contains("dieu chinh") {
            Self::Adjusted
        } else if normalized.contains("thay the") {
            Self::Replaced
        } else if normalized.contains("huy") {
            Self::Cancelled
        } else if normalized.contains("moi")
            || normalized.contains("goc")
            || normalized.contains("hop le")
            || normalized.contains("da cap ma")
            || normalized.contains("con hieu luc")
        {
            Self::Standard
        } else {
            Self::Unknown
        }
    }

    pub fn is_regular(self) -> bool {
        self == Self::Standard
    }
}

/// Converts the mapped raw lifecycle field into a typed business decision.
/// Sources without a lifecycle mapping preserve legacy compatibility; a mapped
/// but absent or unfamiliar value is always `UNKNOWN` and therefore fail-closed.
pub fn invoice_lifecycle_for_record(
    record: &CanonicalRecord,
    mapping: &ColumnMapping,
) -> Option<InvoiceLifecycle> {
    mapping.invoice_status_column.as_ref().map(|column| {
        InvoiceLifecycle::from_status(record.raw_fields.get(column).map(String::as_str))
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnalyticalRowLevel {
    Group,
    Detail,
    NeedsReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartnerRecord {
    pub id: String,
    pub source_id: String,
    pub source_row: u32,
    pub partner_code: Option<String>,
    pub partner_name: Option<String>,
    pub partner_tax_id: Option<String>,
    pub address: Option<String>,
    pub is_customer: Option<bool>,
    pub is_supplier: Option<bool>,
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub raw_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesAnalysisRecord {
    pub id: String,
    pub source_id: String,
    pub source_row: u32,
    pub row_level: AnalyticalRowLevel,
    pub group_key: Option<String>,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub revenue: Option<Decimal>,
    pub vat: Option<Decimal>,
    pub discount: Option<Decimal>,
    pub receivable: Option<Decimal>,
    pub unit_cost: Option<Decimal>,
    pub cost: Option<Decimal>,
    pub profit: Option<Decimal>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub raw_fields: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormalizedDataset {
    Transactional,
    PartnerMaster(Vec<PartnerRecord>),
    SalesAnalysis(Vec<SalesAnalysisRecord>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceControlResult {
    pub source_id: String,
    pub source_kind: DataSourceKind,
    pub record_count: usize,
    pub status: MatchStatus,
    pub message: String,
}

pub fn evaluate_partner_master_control(
    source_id: String,
    records: &[PartnerRecord],
) -> ReferenceControlResult {
    let mut tax_id_counts: HashMap<&str, usize> = HashMap::new();
    for record in records {
        if let Some(tax_id) = record
            .partner_tax_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            *tax_id_counts.entry(tax_id).or_default() += 1;
        }
    }
    let ambiguous = tax_id_counts.values().filter(|&&count| count > 1).count();
    let (status, message) = if ambiguous > 0 {
        (
            MatchStatus::NeedsReview,
            format!(
                "AMBIGUOUS_MASTER_IDENTITY: {} mã số thuế xuất hiện trên nhiều đối tác.",
                ambiguous
            ),
        )
    } else {
        (
            MatchStatus::MatchedExact,
            "Danh mục đối tác hợp lệ; không phát hiện định danh MST trùng.".to_string(),
        )
    };
    ReferenceControlResult {
        source_id,
        source_kind: DataSourceKind::PartnerMaster,
        record_count: records.len(),
        status,
        message,
    }
}

pub fn evaluate_sales_analysis_control(
    source_id: String,
    records: &[SalesAnalysisRecord],
) -> ReferenceControlResult {
    let (status, message) = match select_sales_analysis_control_layer(records) {
        Ok(totals) => (
            MatchStatus::MatchedExact,
            format!(
                "Đã xác minh một tầng dữ liệu {:?}; không cộng đồng thời nhóm và chi tiết.",
                totals.row_level
            ),
        ),
        Err(code) => (MatchStatus::NeedsReview, code.to_string()),
    };
    ReferenceControlResult {
        source_id,
        source_kind: DataSourceKind::SalesAnalysisReport,
        record_count: records.len(),
        status,
        message,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartnerIdentityResolution {
    Unique(String),
    AmbiguousMasterIdentity(Vec<String>),
    NotFound,
}

pub fn resolve_partner_by_tax_id(
    records: &[PartnerRecord],
    tax_id: &str,
) -> PartnerIdentityResolution {
    let normalized = crate::models::CanonicalRecord::normalize_tax_id(tax_id);
    let matches: Vec<String> = records
        .iter()
        .filter(|record| record.partner_tax_id.as_deref() == Some(normalized.as_str()))
        .map(|record| record.id.clone())
        .collect();
    match matches.len() {
        0 => PartnerIdentityResolution::NotFound,
        1 => PartnerIdentityResolution::Unique(matches[0].clone()),
        _ => PartnerIdentityResolution::AmbiguousMasterIdentity(matches),
    }
}

/// Cross-source identity control. It is intentionally read-only: no canonical
/// record is mutated or silently merged. MST is authoritative, partner code is
/// the deterministic fallback, and names are never used for auto-resolution.
pub fn evaluate_partner_identity_cross_source<'a>(
    master_source_id: String,
    masters: &[PartnerRecord],
    transactional_records: impl Iterator<Item = &'a CanonicalRecord>,
) -> ReferenceControlResult {
    let mut unresolved_or_ambiguous = 0usize;
    let mut checked = 0usize;
    for record in transactional_records {
        let resolution = if let Some(tax_id) = record
            .partner_tax_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            resolve_partner_by_tax_id(masters, tax_id)
        } else if let Some(code) = record
            .partner_code
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            let matches: Vec<String> = masters
                .iter()
                .filter(|master| master.partner_code.as_deref() == Some(code))
                .map(|master| master.id.clone())
                .collect();
            match matches.len() {
                0 => PartnerIdentityResolution::NotFound,
                1 => PartnerIdentityResolution::Unique(matches[0].clone()),
                _ => PartnerIdentityResolution::AmbiguousMasterIdentity(matches),
            }
        } else {
            // Name is supporting evidence only and must never auto-merge.
            PartnerIdentityResolution::NotFound
        };
        checked += 1;
        if !matches!(resolution, PartnerIdentityResolution::Unique(_)) {
            unresolved_or_ambiguous += 1;
        }
    }
    let status = if unresolved_or_ambiguous == 0 {
        MatchStatus::MatchedExact
    } else {
        MatchStatus::NeedsReview
    };
    ReferenceControlResult {
        source_id: master_source_id,
        source_kind: DataSourceKind::PartnerMaster,
        record_count: checked,
        status,
        message: if unresolved_or_ambiguous == 0 {
            "PARTNER_IDENTITY: MST hoặc mã đối tác định danh duy nhất; không thay đổi bản ghi nguồn.".to_string()
        } else {
            format!("PARTNER_IDENTITY_NEEDS_REVIEW: {} bản ghi không định danh duy nhất theo MST/mã đối tác.", unresolved_or_ambiguous)
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalesAnalysisControlTotals {
    pub row_level: AnalyticalRowLevel,
    pub revenue: Decimal,
    pub vat: Decimal,
    pub receivable: Decimal,
    pub cost: Decimal,
    pub profit: Decimal,
}

fn totals_for_level(
    records: &[SalesAnalysisRecord],
    level: AnalyticalRowLevel,
) -> SalesAnalysisControlTotals {
    let mut totals = SalesAnalysisControlTotals {
        row_level: level.clone(),
        revenue: Decimal::ZERO,
        vat: Decimal::ZERO,
        receivable: Decimal::ZERO,
        cost: Decimal::ZERO,
        profit: Decimal::ZERO,
    };
    for record in records.iter().filter(|record| record.row_level == level) {
        totals.revenue += record.revenue.unwrap_or(Decimal::ZERO);
        totals.vat += record.vat.unwrap_or(Decimal::ZERO);
        totals.receivable += record.receivable.unwrap_or(Decimal::ZERO);
        totals.cost += record.cost.unwrap_or(Decimal::ZERO);
        totals.profit += record.profit.unwrap_or(Decimal::ZERO);
    }
    totals
}

/// Returns one authoritative layer only. Any hierarchy that cannot prove
/// group/detail equality is fail-closed and requires user review.
pub fn select_sales_analysis_control_layer(
    records: &[SalesAnalysisRecord],
) -> Result<SalesAnalysisControlTotals, &'static str> {
    if records
        .iter()
        .any(|record| record.row_level == AnalyticalRowLevel::NeedsReview)
    {
        return Err("SALES_ANALYSIS_HIERARCHY_NEEDS_REVIEW");
    }
    let groups = totals_for_level(records, AnalyticalRowLevel::Group);
    let details = totals_for_level(records, AnalyticalRowLevel::Detail);
    if groups.revenue.is_zero() || details.revenue.is_zero() {
        return Err("SALES_ANALYSIS_LAYER_MISSING");
    }
    if groups.revenue != details.revenue
        || groups.vat != details.vat
        || groups.receivable != details.receivable
        || groups.cost != details.cost
        || groups.profit != details.profit
    {
        return Err("SALES_ANALYSIS_GROUP_DETAIL_MISMATCH");
    }
    Ok(details)
}

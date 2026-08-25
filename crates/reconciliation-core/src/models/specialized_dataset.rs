//! Typed normalized records for non-transactional sources.
//!
//! These records deliberately do not enter the transaction matcher. They retain
//! provenance and source semantics without forcing master data or analytical
//! report rows into `CanonicalRecord`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

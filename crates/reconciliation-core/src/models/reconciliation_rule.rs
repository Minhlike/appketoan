use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::{ComparisonSemantic, DataSourceKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKeyType {
    DocNo,
    SeriesAndDocNo,
    TaxIdAndAmount,
    TaxIdAndDocNo,
    CustomKeys(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonRule {
    pub id: String,
    pub name: String,
    pub semantic: ComparisonSemantic,
    pub primary_source_kind: DataSourceKind,
    pub primary_field: String,
    pub secondary_source_kind: DataSourceKind,
    pub secondary_field: String,
    #[serde(default = "default_true")]
    pub is_required: bool,
    #[serde(default)]
    pub tolerance_vnd: Decimal,
    #[serde(default)]
    pub date_tolerance_days: u32,
}

fn default_true() -> bool {
    true
}

impl Default for ComparisonRule {
    fn default() -> Self {
        Self {
            id: "rule_revenue_pretax".to_string(),
            name: "Doanh thu bán hàng (Pretax ↔ TK511 Có)".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchingRule {
    pub id: String,
    pub name: String,
    pub primary_keys: Vec<MatchKeyType>,
    #[serde(default)]
    pub allow_date_variance_days: u32,
    #[serde(default)]
    pub allow_amount_tolerance_vnd: Decimal,
    #[serde(default)]
    pub enable_aggregate_match: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aggregate_grouping_keys: Vec<String>,
}

impl Default for MatchingRule {
    fn default() -> Self {
        Self {
            id: "rule_standard_invoice".to_string(),
            name: "Đối chiếu Hóa đơn chuẩn (Số HĐ + Ký hiệu + Tiền)".to_string(),
            primary_keys: vec![MatchKeyType::SeriesAndDocNo],
            allow_date_variance_days: 3,
            allow_amount_tolerance_vnd: Decimal::ONE,
            enable_aggregate_match: false,
            aggregate_grouping_keys: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationProfile {
    pub id: String,
    pub name: String,
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub comparison_rules: Vec<ComparisonRule>,
    pub rules: Vec<MatchingRule>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching_rule_serialization() {
        let rule = MatchingRule::default();
        let json = serde_json::to_string(&rule).expect("Serialization failed");
        assert!(json.contains("series_and_doc_no"));
        let deserialized: MatchingRule =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(rule, deserialized);
    }

    #[test]
    fn test_comparison_rule_serialization() {
        let comp = ComparisonRule::default();
        let json = serde_json::to_string(&comp).expect("Serialization failed");
        assert!(json.contains("REVENUE"));
        let deserialized: ComparisonRule =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(comp, deserialized);
    }
}

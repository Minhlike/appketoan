use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
        let deserialized: MatchingRule = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(rule, deserialized);
    }
}

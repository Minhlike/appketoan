use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{DataSourceKind, ReferenceControlResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchStatus {
    MatchedExact,
    MatchedWithTolerance,
    MatchedAggregate,
    MatchedWithMissingSource,
    MismatchAmount,
    MismatchMetadata,
    UnmatchedMissingInTarget,
    UnmatchedMissingInSource,
    DuplicateSuspect,
    AmbiguousMatch,
    #[serde(
        alias = "INSUFFICIENT_MATCHING_EVIDENCE",
        alias = "INSUFFICIENT_EVIDENCE"
    )]
    NeedsReview,
    /// Semantic was NOT evaluated because the source was absent from this session.
    /// Never means "matched". Must be shown as "CHƯA ĐỐI CHIẾU" in UI.
    NotChecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComparisonSemantic {
    Revenue,
    Vat,
    Receivable,
    BankPayment,
    Other,
}

impl ComparisonSemantic {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Revenue => "DOANH THU",
            Self::Vat => "THUẾ GTGT",
            Self::Receivable => "CÔNG NỢ (PHẢI THU)",
            Self::BankPayment => "DÒNG TIỀN / SAO KÊ",
            Self::Other => "ĐỐI CHIẾU KHÁC",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDiscrepancy {
    pub field_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_diff: Option<Decimal>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMatchBreakdown {
    pub source_id: String,
    pub source_name: String,
    pub record_ids: Vec<String>,
    pub compared_amount: Decimal,
    pub status: MatchStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discrepancies: Vec<FieldDiscrepancy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticFieldComparison {
    pub semantic: ComparisonSemantic,
    pub semantic_name: String,
    pub primary_source_id: String,
    pub primary_source_name: String,
    pub secondary_source_id: String,
    pub secondary_source_name: String,
    pub secondary_source_kind: DataSourceKind,
    pub semantic_field: String,
    pub expected_amount: Decimal,
    pub actual_amount: Decimal,
    pub variance: Decimal,
    pub status: MatchStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primary_record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discrepancies: Vec<FieldDiscrepancy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchGroup {
    pub id: String,
    pub status: MatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_name: Option<String>,
    pub primary_source_record_ids: Vec<String>,
    pub target_source_record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub source_breakdowns: HashMap<String, SourceMatchBreakdown>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discrepancies: Vec<FieldDiscrepancy>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_comparisons: Vec<SemanticFieldComparison>,
    #[serde(default)]
    pub revenue_variance: Decimal,
    #[serde(default)]
    pub vat_variance: Decimal,
    #[serde(default)]
    pub receivable_variance: Decimal,
    #[serde(default)]
    pub other_variance: Decimal,
    pub total_source_amount: Decimal,
    pub total_target_amount: Decimal,
    pub amount_variance: Decimal,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationSummary {
    pub total_source_records: usize,
    pub total_target_records: usize,
    pub exact_matches_count: usize,
    pub tolerance_matches_count: usize,
    pub aggregate_matches_count: usize,
    pub mismatches_count: usize,
    pub missing_in_target_count: usize,
    pub missing_in_source_count: usize,
    pub duplicates_count: usize,
    pub ambiguous_count: usize,
    #[serde(default)]
    pub needs_review_count: usize,
    #[serde(default)]
    pub revenue_variance: Decimal,
    #[serde(default)]
    pub vat_variance: Decimal,
    #[serde(default)]
    pub receivable_variance: Decimal,
    #[serde(default)]
    pub total_discrepant_amount: Decimal,
    pub net_financial_variance: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationResult {
    pub session_id: String,
    pub executed_at: String,
    pub profile_id: String,
    pub summary: ReconciliationSummary,
    pub groups: Vec<MatchGroup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reference_controls: Vec<ReferenceControlResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake_analysis: Option<crate::intake::IntakeAnalysisResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_reconciliation_result_summary() {
        let result = ReconciliationResult {
            session_id: "sess_01".to_string(),
            executed_at: "2026-08-18T22:30:00Z".to_string(),
            profile_id: "prof_standard".to_string(),
            summary: ReconciliationSummary {
                total_source_records: 10,
                total_target_records: 10,
                exact_matches_count: 9,
                mismatches_count: 1,
                revenue_variance: dec!(0),
                vat_variance: dec!(0),
                receivable_variance: dec!(0),
                total_discrepant_amount: dec!(50000),
                net_financial_variance: dec!(50000),
                ..Default::default()
            },
            groups: vec![
                MatchGroup {
                    id: "grp_1".to_string(),
                    status: MatchStatus::MatchedExact,
                    doc_no: Some("001".to_string()),
                    series: Some("1C26TAA".to_string()),
                    date: Some("2026-01-05".to_string()),
                    partner_name: Some("Công ty A".to_string()),
                    primary_source_record_ids: vec!["rec_1".to_string()],
                    target_source_record_ids: vec!["rec_2".to_string()],
                    source_breakdowns: HashMap::new(),
                    discrepancies: vec![],
                    semantic_comparisons: vec![],
                    revenue_variance: Decimal::ZERO,
                    vat_variance: Decimal::ZERO,
                    receivable_variance: Decimal::ZERO,
                    other_variance: Decimal::ZERO,
                    total_source_amount: dec!(1000000),
                    total_target_amount: dec!(1000000),
                    amount_variance: Decimal::ZERO,
                },
                MatchGroup {
                    id: "grp_2".to_string(),
                    status: MatchStatus::MismatchAmount,
                    doc_no: Some("002".to_string()),
                    series: Some("1C26TAA".to_string()),
                    date: Some("2026-01-06".to_string()),
                    partner_name: Some("Công ty B".to_string()),
                    primary_source_record_ids: vec!["rec_3".to_string()],
                    target_source_record_ids: vec!["rec_4".to_string()],
                    source_breakdowns: HashMap::new(),
                    discrepancies: vec![FieldDiscrepancy {
                        field_name: "totalAmount".to_string(),
                        source_value: Some("1,050,000".to_string()),
                        target_value: Some("1,000,000".to_string()),
                        amount_diff: Some(dec!(50000)),
                        message: "Lệch tiền thanh toán 50,000 VND".to_string(),
                    }],
                    semantic_comparisons: vec![],
                    revenue_variance: dec!(50000),
                    vat_variance: Decimal::ZERO,
                    receivable_variance: Decimal::ZERO,
                    other_variance: Decimal::ZERO,
                    total_source_amount: dec!(1050000),
                    total_target_amount: dec!(1000000),
                    amount_variance: dec!(50000),
                },
            ],
            reference_controls: vec![],
            intake_analysis: None,
        };

        let json = serde_json::to_string(&result).expect("Serialization failed");
        assert!(json.contains("MATCHED_EXACT"));
        assert!(json.contains("MISMATCH_AMOUNT"));
        assert!(json.contains("50000"));

        let deserialized: ReconciliationResult =
            serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(result, deserialized);
    }
}

use std::collections::HashMap;
use std::path::Path;

use reconciliation_core::{
    detect_header_and_mapping_with_context, execute_reconciliation, inspect_excel_file,
    normalize_data_source_rows, read_sheet_rows, CanonicalRecord, ColumnMapping, ComparisonRule,
    ComparisonSemantic, DataSource, DataSourceKind, MatchStatus, ReconciliationSession, SourceRole,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn create_source(
    id: &str,
    name: &str,
    kind: DataSourceKind,
    role: SourceRole,
    doc_col: Option<&str>,
    amount_col: Option<&str>,
) -> DataSource {
    DataSource {
        id: id.to_string(),
        name: name.to_string(),
        file_path: format!("C:/mock/{}.xlsx", id),
        sheet_name: "Sheet1".to_string(),
        kind,
        role,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: doc_col.map(|s| s.to_string()),
            credit_amount_column: amount_col.map(|s| s.to_string()),
            total_amount_column: amount_col.map(|s| s.to_string()),
            pretax_amount_column: amount_col.map(|s| s.to_string()),
            ..Default::default()
        },
    }
}

// -------------------------------------------------------------------------------------------------
// 1. ENFORCE REQUIRED / OPTIONAL SOURCE IN RUST CORE
// -------------------------------------------------------------------------------------------------
#[test]
fn test_01_required_source_missing_before_run() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_req_01".to_string(),
        scenario_name: "Test Required".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec![
            "src_inv".to_string(),
            "src_511".to_string(),
            "src_3331".to_string(),
        ]),
        optional_source_ids: None,
        data_sources: vec![src1, src2], // src_3331 is missing!
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_inv".to_string(), vec![]);
    records_map.insert("src_511".to_string(), vec![]);

    let res = execute_reconciliation(&session, &records_map);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Thiếu nguồn dữ liệu bắt buộc"));
}

#[test]
fn test_02_required_source_uploaded_but_record_missing() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );
    let src3 = create_source(
        "src_3331",
        "Sổ 3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_req_02".to_string(),
        scenario_name: "Test Required 3-Way".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string(), "src_3331".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src1, src2, src3],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            pretax_amount: Some(dec!(10000000)),
            vat_amount: Some(dec!(1000000)),
            total_amount: dec!(11000000),
            ..Default::default()
        }],
    );
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk511_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            credit_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );
    // src_3331 has NO record for #100
    records_map.insert("src_3331".to_string(), vec![]);

    let res =
        execute_reconciliation(&session, &records_map).expect("Reconciliation should execute");
    assert_eq!(res.groups.len(), 1);
    let grp = &res.groups[0];

    assert_eq!(grp.status, MatchStatus::UnmatchedMissingInTarget);
    assert_ne!(grp.status, MatchStatus::MatchedExact);
    assert_eq!(grp.revenue_variance, dec!(0));
    assert_eq!(grp.vat_variance, dec!(1000000));
}

#[test]
fn test_03_optional_source_absent() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_opt_03".to_string(),
        scenario_name: "Test Optional".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string()]),
        optional_source_ids: Some(vec!["src_bank".to_string()]), // Not uploaded
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            pretax_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk511_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            credit_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    assert_eq!(res.summary.exact_matches_count, 1);
    assert_eq!(res.groups[0].status, MatchStatus::MatchedExact);
}

// -------------------------------------------------------------------------------------------------
// 2. PRIMARY VALIDATION
// -------------------------------------------------------------------------------------------------
#[test]
fn test_04_primary_kind_validation_rejects_wrong_kind() {
    let src1 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::Primary, // User set 511 as primary
        Some("Số CT"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_3331",
        "Sổ 3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_kind_04".to_string(),
        scenario_name: "Scenario EInvoice Required".to_string(),
        primary_source_id: Some("src_511".to_string()),
        expected_primary_kind: Some(DataSourceKind::EInvoice), // Scenario strictly requires EInvoice
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_511".to_string(), vec![]);
    records_map.insert("src_3331".to_string(), vec![]);

    let res = execute_reconciliation(&session, &records_map);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .contains("không đúng loại dữ liệu kịch bản yêu cầu"));
}

// -------------------------------------------------------------------------------------------------
// 3. DYNAMIC SCENARIO COMPARISON RULES
// -------------------------------------------------------------------------------------------------
#[test]
fn test_05_scenario_dynamic_comparison_rules_execution() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let custom_rule = ComparisonRule {
        id: "rule_custom_pretax".to_string(),
        name: "Quy tắc kiểm tra Pretax".to_string(),
        semantic: ComparisonSemantic::Revenue,
        primary_source_kind: DataSourceKind::EInvoice,
        primary_field: "pretaxAmount".to_string(),
        secondary_source_kind: DataSourceKind::Ledger511,
        secondary_field: "creditAmount".to_string(),
        is_required: true,
        tolerance_vnd: dec!(100),
        date_tolerance_days: 3,
    };

    let session = ReconciliationSession {
        session_id: "sess_rules_05".to_string(),
        scenario_name: "Dynamic Scenario".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![custom_rule],
        matching_tolerance_vnd: dec!(100),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000200".to_string()),
            pretax_amount: Some(dec!(5000000)),
            total_amount: dec!(5500000),
            ..Default::default()
        }],
    );
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk511_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000200".to_string()),
            credit_amount: Some(dec!(5000050)), // within 100 tolerance
            total_amount: dec!(5000050),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    assert_eq!(res.groups[0].status, MatchStatus::MatchedWithTolerance);
}

// -------------------------------------------------------------------------------------------------
// 4. COMPOSITE KEY (SERIES-AWARE)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_06_same_doc_no_different_series_collision_prevention() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_series_06".to_string(),
        scenario_name: "Series Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "inv_a".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                series: Some("C26AAA".to_string()),
                doc_no: Some("00000123".to_string()),
                pretax_amount: Some(dec!(10000000)),
                total_amount: dec!(10000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "inv_b".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 3,
                series: Some("C26BBB".to_string()),
                doc_no: Some("00000123".to_string()),
                pretax_amount: Some(dec!(20000000)),
                total_amount: dec!(20000000),
                ..Default::default()
            },
        ],
    );
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk511_a".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            series: Some("C26AAA".to_string()),
            doc_no: Some("00000123".to_string()),
            credit_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    assert_eq!(res.summary.exact_matches_count, 1);
    assert_eq!(res.summary.missing_in_target_count, 1);

    let exact_grp = res
        .groups
        .iter()
        .find(|g| g.status == MatchStatus::MatchedExact)
        .unwrap();
    assert_eq!(exact_grp.series.as_deref(), Some("C26AAA"));

    let missing_grp = res
        .groups
        .iter()
        .find(|g| g.status == MatchStatus::UnmatchedMissingInTarget)
        .unwrap();
    assert_eq!(missing_grp.series.as_deref(), Some("C26BBB"));
}

// -------------------------------------------------------------------------------------------------
// 5. AGGREGATE BOUNDARY & AMBIGUITY
// -------------------------------------------------------------------------------------------------
#[test]
fn test_07_aggregate_boundary_date_filtering() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_agg_date_07".to_string(),
        scenario_name: "Aggregate Date".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 5,
        enable_aggregate_match: true,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000555".to_string()),
            date: Some("2026-01-01".to_string()),
            pretax_amount: Some(dec!(100000000)),
            total_amount: dec!(100000000),
            ..Default::default()
        }],
    );

    // One candidate within date window (01/01/2026), one candidate 6 months later (15/06/2026)
    records_map.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "tk_a".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("00000555".to_string()),
                date: Some("2026-01-02".to_string()),
                credit_amount: Some(dec!(40000000)),
                total_amount: dec!(40000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "tk_b".to_string(),
                source_id: "src_511".to_string(),
                source_row: 3,
                doc_no: Some("00000555".to_string()),
                date: Some("2026-06-15".to_string()), // Far date outside tolerance!
                credit_amount: Some(dec!(60000000)),
                total_amount: dec!(60000000),
                ..Default::default()
            },
        ],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    // MUST NOT aggregate tk_a and tk_b because tk_b date is outside tolerance!
    assert_ne!(res.groups[0].status, MatchStatus::MatchedAggregate);
    assert_eq!(res.groups[0].status, MatchStatus::MismatchAmount);
}

#[test]
fn test_08_unsafe_aggregate_ambiguity_no_arbitrary_guess() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_ambig_08".to_string(),
        scenario_name: "Ambiguity Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 5,
        enable_aggregate_match: true,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000999".to_string()),
            pretax_amount: Some(dec!(100000000)),
            total_amount: dec!(100000000),
            ..Default::default()
        }],
    );

    // Candidate A = 100M, Candidate B = 40M, Candidate C = 60M (Two subsets equal 100M!)
    records_map.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "cand_a".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("00000999".to_string()),
                credit_amount: Some(dec!(100000000)),
                total_amount: dec!(100000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "cand_b".to_string(),
                source_id: "src_511".to_string(),
                source_row: 3,
                doc_no: Some("00000999".to_string()),
                credit_amount: Some(dec!(40000000)),
                total_amount: dec!(40000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "cand_c".to_string(),
                source_id: "src_511".to_string(),
                source_row: 4,
                doc_no: Some("00000999".to_string()),
                credit_amount: Some(dec!(60000000)),
                total_amount: dec!(60000000),
                ..Default::default()
            },
        ],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    assert_eq!(res.groups[0].status, MatchStatus::AmbiguousMatch);
    assert_eq!(res.summary.ambiguous_count, 1);
}

// -------------------------------------------------------------------------------------------------
// 6. SINGLE CANDIDATE METADATA VALIDATION
// -------------------------------------------------------------------------------------------------
#[test]
fn test_09_single_candidate_validates_metadata() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_meta_09".to_string(),
        scenario_name: "Meta Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 2,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000777".to_string()),
            date: Some("2026-01-01".to_string()),
            partner_tax_id: Some("0100000001".to_string()),
            pretax_amount: Some(dec!(50000000)),
            total_amount: dec!(50000000),
            ..Default::default()
        }],
    );

    // Same doc & amount, but date differs by 10 days
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000777".to_string()),
            date: Some("2026-01-11".to_string()),
            partner_tax_id: Some("0100000001".to_string()),
            credit_amount: Some(dec!(50000000)),
            total_amount: dec!(50000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    assert_eq!(res.groups[0].status, MatchStatus::MismatchMetadata);
    assert_ne!(res.groups[0].status, MatchStatus::MatchedExact);
}

// -------------------------------------------------------------------------------------------------
// 7. NO-DOC FALLBACK MULTI-SOURCE EVALUATION
// -------------------------------------------------------------------------------------------------
#[test]
fn test_10_no_doc_fallback_requires_multi_evidence_and_evaluates_all_secondaries() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        None,
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        None,
        Some("Tiền"),
    );
    let src3 = create_source(
        "src_3331",
        "Sổ 3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        None,
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_nodoc_10".to_string(),
        scenario_name: "No Doc Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string(), "src_3331".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src1, src2, src3],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_nodoc".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: None,
            date: Some("2026-01-05".to_string()),
            partner_tax_id: Some("0109998888".to_string()),
            pretax_amount: Some(dec!(30000000)),
            vat_amount: Some(dec!(3000000)),
            total_amount: dec!(33000000),
            ..Default::default()
        }],
    );

    // Matches TK511, but TK3331 is absent
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk_nodoc".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: None,
            date: Some("2026-01-05".to_string()),
            partner_tax_id: Some("0109998888".to_string()),
            credit_amount: Some(dec!(30000000)),
            total_amount: dec!(30000000),
            ..Default::default()
        }],
    );
    records_map.insert("src_3331".to_string(), vec![]);

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    assert_eq!(res.groups[0].status, MatchStatus::UnmatchedMissingInTarget);
}

// -------------------------------------------------------------------------------------------------
// 8. ALL-SECONDARY-MISSING PRESERVES FULL SEMANTICS
// -------------------------------------------------------------------------------------------------
#[test]
fn test_11_all_secondary_missing_preserves_semantic_comparisons() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );
    let src3 = create_source(
        "src_3331",
        "Sổ 3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_missing_all_11".to_string(),
        scenario_name: "Missing All Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2, src3],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_missing".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000999".to_string()),
            pretax_amount: Some(dec!(100000000)),
            vat_amount: Some(dec!(10000000)),
            total_amount: dec!(110000000),
            ..Default::default()
        }],
    );
    records_map.insert("src_511".to_string(), vec![]);
    records_map.insert("src_3331".to_string(), vec![]);

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    let grp = &res.groups[0];

    assert_eq!(grp.status, MatchStatus::UnmatchedMissingInTarget);
    assert_eq!(grp.revenue_variance, dec!(100000000));
    assert_eq!(grp.vat_variance, dec!(10000000));
    assert_eq!(grp.semantic_comparisons.len(), 2);
    assert_eq!(
        grp.semantic_comparisons[0].semantic,
        ComparisonSemantic::Revenue
    );
    assert_eq!(
        grp.semantic_comparisons[1].semantic,
        ComparisonSemantic::Vat
    );
}

// -------------------------------------------------------------------------------------------------
// 9. RESIDUAL SECONDARY SEMANTICS
// -------------------------------------------------------------------------------------------------
#[test]
fn test_12_residual_secondary_semantics_by_kind() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_3331",
        "Sổ 3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_res_12".to_string(),
        scenario_name: "Residual 3331".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_inv".to_string(), vec![]);
    records_map.insert(
        "src_3331".to_string(),
        vec![CanonicalRecord {
            id: "tk3331_extra".to_string(),
            source_id: "src_3331".to_string(),
            source_row: 2,
            doc_no: Some("00000888".to_string()),
            credit_amount: Some(dec!(5000000)),
            total_amount: dec!(5000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    let grp = &res.groups[0];

    assert_eq!(grp.status, MatchStatus::UnmatchedMissingInSource);
    assert_eq!(grp.vat_variance, dec!(-5000000));
    assert_eq!(grp.revenue_variance, dec!(0));
}

// -------------------------------------------------------------------------------------------------
// 10. VARIANCE ISOLATION (NO CANCELING)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_13_variance_isolation_no_canceling_rev_vat() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );
    let src3 = create_source(
        "src_3331",
        "Sổ 3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_cancel_13".to_string(),
        scenario_name: "No Canceling".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2, src3],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            pretax_amount: Some(dec!(100000000)),
            vat_amount: Some(dec!(10000000)),
            total_amount: dec!(110000000),
            ..Default::default()
        }],
    );

    // 511 is 90M (+10M rev var), 3331 is 20M (-10M vat var)
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk_511".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            credit_amount: Some(dec!(90000000)),
            total_amount: dec!(90000000),
            ..Default::default()
        }],
    );
    records_map.insert(
        "src_3331".to_string(),
        vec![CanonicalRecord {
            id: "tk_3331".to_string(),
            source_id: "src_3331".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            credit_amount: Some(dec!(20000000)),
            total_amount: dec!(20000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    let grp = &res.groups[0];

    assert_eq!(grp.status, MatchStatus::MismatchAmount);
    assert_eq!(grp.revenue_variance, dec!(10000000));
    assert_eq!(grp.vat_variance, dec!(-10000000));
    assert_eq!(res.summary.total_discrepant_amount, dec!(20000000));
}

// -------------------------------------------------------------------------------------------------
// 14. DECIMAL RUST SERIALIZATION & ARTIFACT GENERATION
// -------------------------------------------------------------------------------------------------
#[test]
fn test_14_decimal_rust_serialization_and_artifact_generation() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_ipc_contract_v5".to_string(),
        scenario_name: "IPC Contract Verification".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "rec_zero".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("00000001".to_string()),
                pretax_amount: Some(dec!(0)),
                total_amount: dec!(0),
                ..Default::default()
            },
            CanonicalRecord {
                id: "rec_large_inv".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 3,
                doc_no: Some("00000002".to_string()),
                pretax_amount: Some(dec!(7328121057)),
                total_amount: dec!(7328121057),
                ..Default::default()
            },
            CanonicalRecord {
                id: "rec_missing_233".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 4,
                doc_no: Some("00000233".to_string()),
                pretax_amount: Some(dec!(105000000)),
                total_amount: dec!(105000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "rec_fractional".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 5,
                doc_no: Some("00000003".to_string()),
                pretax_amount: Some(dec!(1250000.50)),
                total_amount: dec!(1250000.50),
                ..Default::default()
            },
            CanonicalRecord {
                id: "rec_negative".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 6,
                doc_no: Some("00000004".to_string()),
                pretax_amount: Some(dec!(-500000)),
                total_amount: dec!(-500000),
                ..Default::default()
            },
        ],
    );
    records_map.insert("src_511".to_string(), vec![]);

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    let json_str = serde_json::to_string_pretty(&res).expect("Serialization failed");

    // Verify preservation
    assert!(json_str.contains("7328121057"));
    assert!(json_str.contains("105000000"));
    assert!(json_str.contains("1250000.50"));
    assert!(json_str.contains("-500000"));

    // Write to fixture artifact directory for TS verification
    let artifact_path = Path::new("fixtures/artifacts/ipc_contract_output.json");
    if let Some(parent) = artifact_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(artifact_path, &json_str);

    // Also write to audit/generated-ipc-contract.json at root and current dir
    for audit_rel in &[
        "audit/generated-ipc-contract.json",
        "../../audit/generated-ipc-contract.json",
    ] {
        let p = Path::new(audit_rel);
        if let Some(parent) = p.parent() {
            if parent.exists() || audit_rel.starts_with("audit") {
                let _ = std::fs::create_dir_all(parent);
                let _ = std::fs::write(p, &json_str);
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// 15. SOURCE DETECTION FAIL-CLOSED
// -------------------------------------------------------------------------------------------------
#[test]
fn test_15_source_detection_fail_closed() {
    let rows_unrecognized = vec![
        vec![
            "Cột 1".to_string(),
            "Cột 2".to_string(),
            "Cột 3".to_string(),
        ],
        vec![
            "Val A".to_string(),
            "Val B".to_string(),
            "Val C".to_string(),
        ],
    ];
    let (_, _, _, _, kind, conf) =
        detect_header_and_mapping_with_context("Data", &rows_unrecognized);
    assert_eq!(kind, DataSourceKind::Custom);
    assert!(conf < 0.5);
}

// -------------------------------------------------------------------------------------------------
// 16. DIRTY EXCEL METAMORPHIC ROBUSTNESS
// -------------------------------------------------------------------------------------------------
#[test]
fn test_16_dirty_excel_metamorphic_robustness() {
    // Header at row 4 with leading titles and extra spaces
    let dirty_rows = vec![
        vec!["CÔNG TY TNHH KẾ TOÁN MẪU".to_string()],
        vec!["BẢNG KÊ DOANH THU THÁNG 7".to_string()],
        vec!["".to_string()],
        vec![
            "  KÝ HIỆU HÓA ĐƠN  ".to_string(),
            "SỐ HÓA ĐƠN".to_string(),
            "NGÀY LẬP".to_string(),
            "TỔNG TIỀN CHƯA THUẾ".to_string(),
            "TỔNG TIỀN THUẾ".to_string(),
            "TỔNG TIỀN THANH TOÁN".to_string(),
        ],
        vec![
            "1C26TAA".to_string(),
            "00000101".to_string(),
            "05/01/2026".to_string(),
            "10.000.000".to_string(),
            "1.000.000".to_string(),
            "11.000.000".to_string(),
        ],
    ];

    let (hdr_idx, data_idx, _, mapping, kind, _) =
        detect_header_and_mapping_with_context("HĐ", &dirty_rows);
    assert_eq!(hdr_idx, 4);
    assert_eq!(data_idx, 5);
    assert_eq!(kind, DataSourceKind::EInvoice);
    assert!(mapping.doc_no_column.is_some());
    assert!(mapping.pretax_amount_column.is_some());
}

// -------------------------------------------------------------------------------------------------
// 17. DETERMINISM TEST (20 PERMUTATIONS)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_17_determinism_20_permutations() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_det_17".to_string(),
        scenario_name: "Determinism".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let base_records = vec![
        CanonicalRecord {
            id: "inv_3".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 4,
            doc_no: Some("00000300".to_string()),
            date: Some("2026-01-03".to_string()),
            pretax_amount: Some(dec!(30000000)),
            total_amount: dec!(30000000),
            ..Default::default()
        },
        CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            date: Some("2026-01-01".to_string()),
            pretax_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        },
        CanonicalRecord {
            id: "inv_2".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 3,
            doc_no: Some("00000200".to_string()),
            date: Some("2026-01-02".to_string()),
            pretax_amount: Some(dec!(20000000)),
            total_amount: dec!(20000000),
            ..Default::default()
        },
    ];

    let mut first_json = String::new();

    for i in 0..20 {
        let mut permuted = base_records.clone();
        if i % 2 == 0 {
            permuted.reverse();
        } else if i % 3 == 0 {
            permuted.swap(0, 1);
        }

        let mut records_map = HashMap::new();
        records_map.insert("src_inv".to_string(), permuted);
        records_map.insert("src_511".to_string(), vec![]);

        let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
        let json = serde_json::to_string(&res.groups).expect("JSON failed");

        if i == 0 {
            first_json = json;
        } else {
            assert_eq!(first_json, json, "Run {} failed determinism check!", i);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// 18. FALSE-POSITIVE ADVERSARIAL SUITE
// -------------------------------------------------------------------------------------------------
#[test]
fn test_18_false_positive_adversarial_suite() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_adv_18".to_string(),
        scenario_name: "Adversarial".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    // Case: Same amount, completely different doc numbers
    let mut records_map = HashMap::new();
    records_map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000111".to_string()),
            pretax_amount: Some(dec!(99999999)),
            total_amount: dec!(99999999),
            ..Default::default()
        }],
    );
    records_map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000222".to_string()), // Different doc!
            credit_amount: Some(dec!(99999999)),
            total_amount: dec!(99999999),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map).expect("Should succeed");
    // FALSE MATCH MUST BE 0!
    assert_eq!(res.summary.exact_matches_count, 0);
    assert_eq!(res.summary.missing_in_target_count, 1);
    assert_eq!(res.summary.missing_in_source_count, 1);
}

// -------------------------------------------------------------------------------------------------
// 19. REAL 2-FILE BASELINE REGRESSION (46 / 45 / 45 / #233)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_19_real_2_file_baseline_46_45_missing_233() {
    let inv_path = "D:\\appketoan\\T7.2026 Thuế.xlsx";
    let tk_path = "D:\\appketoan\\T7.2026.xlsx";

    if !Path::new(inv_path).exists() || !Path::new(tk_path).exists() {
        println!("Skipping real 2-file test as files are not present in current env");
        return;
    }

    let meta_inv = inspect_excel_file(inv_path).expect("Failed to inspect invoice file");
    let sheet_inv = &meta_inv.sheets[0];
    let raw_inv = read_sheet_rows(inv_path, &sheet_inv.name).expect("Failed to read invoices");
    let ds_inv = DataSource {
        id: "src_real_invoice".to_string(),
        name: "Hóa đơn Thuế T7.2026".to_string(),
        file_path: inv_path.to_string(),
        sheet_name: sheet_inv.name.clone(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: sheet_inv.detected_header_row,
        data_start_row: sheet_inv.detected_data_start_row,
        column_mapping: sheet_inv.suggested_mapping.clone(),
    };
    let records_inv = normalize_data_source_rows(&ds_inv, &sheet_inv.columns, &raw_inv);

    let meta_tk = inspect_excel_file(tk_path).expect("Failed to inspect TK511 file");
    let sheet_tk = &meta_tk.sheets[0];
    let raw_tk = read_sheet_rows(tk_path, &sheet_tk.name).expect("Failed to read TK511");
    let ds_tk = DataSource {
        id: "src_real_tk511".to_string(),
        name: "Sổ cái TK 511 T7.2026".to_string(),
        file_path: tk_path.to_string(),
        sheet_name: sheet_tk.name.clone(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: sheet_tk.detected_header_row,
        data_start_row: sheet_tk.detected_data_start_row,
        column_mapping: sheet_tk.suggested_mapping.clone(),
    };
    let records_tk = normalize_data_source_rows(&ds_tk, &sheet_tk.columns, &raw_tk);

    assert_eq!(records_inv.len(), 46, "Expected exactly 46 invoice records");
    assert_eq!(records_tk.len(), 45, "Expected exactly 45 TK511 records");

    let inv_pretax_sum: Decimal = records_inv
        .iter()
        .map(|r| r.pretax_amount.unwrap_or(r.total_amount))
        .sum();
    let tk_credit_sum: Decimal = records_tk
        .iter()
        .map(|r| r.credit_amount.unwrap_or(r.total_amount))
        .sum();

    assert_eq!(inv_pretax_sum, dec!(7328121057));
    assert_eq!(tk_credit_sum, dec!(7223121057));

    let session = ReconciliationSession {
        session_id: "sess_real_baseline_v5".to_string(),
        scenario_name: "Đối chiếu Doanh thu Hóa đơn ↔ Sổ cái TK511".to_string(),
        primary_source_id: Some("src_real_invoice".to_string()),
        expected_primary_kind: Some(DataSourceKind::EInvoice),
        required_source_ids: Some(vec!["src_real_tk511".to_string()]),
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_tk],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_real_invoice".to_string(), records_inv);
    map.insert("src_real_tk511".to_string(), records_tk);

    let res = execute_reconciliation(&session, &map).expect("Reconciliation should succeed");

    assert_eq!(res.summary.exact_matches_count, 45);
    assert_eq!(res.summary.mismatches_count, 0);
    assert_eq!(res.summary.missing_in_target_count, 1);
    assert_eq!(res.summary.missing_in_source_count, 0);
    assert_eq!(res.summary.revenue_variance, dec!(105000000));
    assert_eq!(res.summary.net_financial_variance, dec!(105000000));

    let missing_grp = res
        .groups
        .iter()
        .find(|g| g.status == MatchStatus::UnmatchedMissingInTarget)
        .unwrap();
    assert_eq!(missing_grp.doc_no.as_deref(), Some("233"));
    assert_eq!(missing_grp.date.as_deref(), Some("2026-07-06"));
    assert_eq!(missing_grp.total_source_amount, dec!(105000000));
}

// -------------------------------------------------------------------------------------------------
// 20. INDEPENDENT ORACLE TEST
// -------------------------------------------------------------------------------------------------
#[test]
fn test_20_independent_oracle_verification() {
    let inv_path = "D:\\appketoan\\T7.2026 Thuế.xlsx";
    let tk_path = "D:\\appketoan\\T7.2026.xlsx";

    if !Path::new(inv_path).exists() || !Path::new(tk_path).exists() {
        return;
    }

    let meta_inv = inspect_excel_file(inv_path).unwrap();
    let raw_inv = read_sheet_rows(inv_path, &meta_inv.sheets[0].name).unwrap();
    let ds_inv = DataSource {
        id: "src_inv".to_string(),
        name: "HĐ".to_string(),
        file_path: inv_path.to_string(),
        sheet_name: meta_inv.sheets[0].name.clone(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: meta_inv.sheets[0].detected_header_row,
        data_start_row: meta_inv.sheets[0].detected_data_start_row,
        column_mapping: meta_inv.sheets[0].suggested_mapping.clone(),
    };
    let records_inv = normalize_data_source_rows(&ds_inv, &meta_inv.sheets[0].columns, &raw_inv);

    let meta_tk = inspect_excel_file(tk_path).unwrap();
    let raw_tk = read_sheet_rows(tk_path, &meta_tk.sheets[0].name).unwrap();
    let ds_tk = DataSource {
        id: "src_tk".to_string(),
        name: "TK511".to_string(),
        file_path: tk_path.to_string(),
        sheet_name: meta_tk.sheets[0].name.clone(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: meta_tk.sheets[0].detected_header_row,
        data_start_row: meta_tk.sheets[0].detected_data_start_row,
        column_mapping: meta_tk.sheets[0].suggested_mapping.clone(),
    };
    let records_tk = normalize_data_source_rows(&ds_tk, &meta_tk.sheets[0].columns, &raw_tk);

    // Minimal Independent Oracle Lookup
    let mut oracle_tk_map: HashMap<String, Decimal> = HashMap::new();
    for rec in &records_tk {
        if let Some(doc) = &rec.doc_no {
            let clean = CanonicalRecord::normalize_doc_no(doc);
            oracle_tk_map.insert(clean, rec.credit_amount.unwrap_or(rec.total_amount));
        }
    }

    let mut oracle_exact = 0;
    let mut oracle_missing = 0;
    let mut oracle_missing_doc = String::new();

    for inv in &records_inv {
        let clean = CanonicalRecord::normalize_doc_no(inv.doc_no.as_deref().unwrap_or(""));
        let inv_amt = inv.pretax_amount.unwrap_or(inv.total_amount);
        if let Some(tk_amt) = oracle_tk_map.get(&clean) {
            if *tk_amt == inv_amt {
                oracle_exact += 1;
            }
        } else {
            oracle_missing += 1;
            oracle_missing_doc = clean;
        }
    }

    assert_eq!(oracle_exact, 45);
    assert_eq!(oracle_missing, 1);
    assert_eq!(oracle_missing_doc, "233");
}

// -------------------------------------------------------------------------------------------------
// 21. PROPERTY INVARIANTS TEST
// -------------------------------------------------------------------------------------------------
#[test]
fn test_21_property_invariants() {
    let src1 = create_source(
        "src_inv",
        "HĐ",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "TK511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_prop_21".to_string(),
        scenario_name: "Property Invariants".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut map = HashMap::new();
    map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_1".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            pretax_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );
    map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            credit_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &map).unwrap();

    // Property Invariant 1: MATCHED_EXACT => all semantic comparisons exact
    for g in &res.groups {
        if g.status == MatchStatus::MatchedExact {
            for comp in &g.semantic_comparisons {
                assert_eq!(comp.status, MatchStatus::MatchedExact);
                assert_eq!(comp.variance, dec!(0));
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// 22. PRIMARY RECORD CONSERVATION (NO RECORD DISAPPEARS EVEN WITH NO DOC & NO MST)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_22_primary_record_never_disappears_even_without_doc_and_tax_id() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_prim_conserv_22".to_string(),
        scenario_name: "Primary Conservation".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };

    let mut map = HashMap::new();
    map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_no_doc_no_mst".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: None,
            partner_tax_id: None,
            date: Some("2026-01-15".to_string()),
            pretax_amount: Some(dec!(50000000)),
            total_amount: dec!(55000000),
            ..Default::default()
        }],
    );
    map.insert("src_511".to_string(), vec![]);

    let res = execute_reconciliation(&session, &map).expect("Should succeed");
    assert_eq!(res.groups.len(), 1);
    let grp = &res.groups[0];
    assert_eq!(grp.status, MatchStatus::NeedsReview);
    assert_ne!(grp.status, MatchStatus::MatchedExact);
    assert_eq!(
        grp.primary_source_record_ids,
        vec!["inv_no_doc_no_mst".to_string()]
    );
    assert_eq!(res.summary.needs_review_count, 1);
    assert_eq!(res.summary.total_source_records, 1);
}

// -------------------------------------------------------------------------------------------------
// 23. CANDIDATE CONSUMPTION SAFETY (AMBIGUOUS DOES NOT THEFT CANDIDATES FROM EXACT)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_23_candidate_not_consumed_by_ambiguous_or_mismatch() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_consumption_safety_23".to_string(),
        scenario_name: "Candidate Consumption Safety".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    // Primary A: doc 100, series None, amount 50M -> In ledger there are [30M, 20M, 50M] => multiple combos -> Ambiguous
    // Primary B: doc 200, series "1C26TAA", amount 30M -> Matches exactly TK_200
    map.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "inv_ambiguous".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("00000100".to_string()),
                series: None,
                date: Some("2026-01-10".to_string()),
                pretax_amount: Some(dec!(50000000)),
                total_amount: dec!(50000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "inv_exact".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 3,
                doc_no: Some("00000200".to_string()),
                series: Some("1C26TAA".to_string()),
                date: Some("2026-01-10".to_string()),
                pretax_amount: Some(dec!(30000000)),
                total_amount: dec!(30000000),
                ..Default::default()
            },
        ],
    );

    map.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "tk_100_a".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("00000100".to_string()),
                date: Some("2026-01-10".to_string()),
                credit_amount: Some(dec!(50000000)),
                total_amount: dec!(50000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "tk_100_b".to_string(),
                source_id: "src_511".to_string(),
                source_row: 3,
                doc_no: Some("00000100".to_string()),
                date: Some("2026-01-10".to_string()),
                credit_amount: Some(dec!(50000000)),
                total_amount: dec!(50000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "tk_200".to_string(),
                source_id: "src_511".to_string(),
                source_row: 4,
                doc_no: Some("00000200".to_string()),
                series: Some("1C26TAA".to_string()),
                date: Some("2026-01-10".to_string()),
                credit_amount: Some(dec!(30000000)),
                total_amount: dec!(30000000),
                ..Default::default()
            },
        ],
    );

    let res = execute_reconciliation(&session, &map).expect("Should succeed");
    let grp_amb = res
        .groups
        .iter()
        .find(|g| g.id.contains("inv_ambiguous"))
        .unwrap();
    assert_eq!(grp_amb.status, MatchStatus::AmbiguousMatch);

    let grp_exact = res
        .groups
        .iter()
        .find(|g| g.id.contains("inv_exact"))
        .unwrap();
    assert_eq!(grp_exact.status, MatchStatus::MatchedExact);
    assert_eq!(
        grp_exact.target_source_record_ids,
        vec!["tk_200".to_string()]
    );
}

// -------------------------------------------------------------------------------------------------
// 24. AGGREGATE CROSS-COUNTERPARTY TAX ID REJECTED
// -------------------------------------------------------------------------------------------------
#[test]
fn test_24_aggregate_cross_counterparty_tax_id_rejected() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_cross_mst_24".to_string(),
        scenario_name: "Aggregate Cross MST".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    // Primary: Invoice #500, MST A = 100M
    map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_500".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000500".to_string()),
            partner_tax_id: Some("0101112222".to_string()),
            date: Some("2026-01-10".to_string()),
            pretax_amount: Some(dec!(100000000)),
            total_amount: dec!(100000000),
            ..Default::default()
        }],
    );

    // Ledger has: #500 MST A = 40M, and #500 MST B = 60M
    // 40M + 60M = 100M, but MST B is different counterparty! Must NOT aggregate!
    map.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "tk_500_mst_a".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("00000500".to_string()),
                partner_tax_id: Some("0101112222".to_string()),
                date: Some("2026-01-10".to_string()),
                credit_amount: Some(dec!(40000000)),
                total_amount: dec!(40000000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "tk_500_mst_b".to_string(),
                source_id: "src_511".to_string(),
                source_row: 3,
                doc_no: Some("00000500".to_string()),
                partner_tax_id: Some("0309998888".to_string()),
                date: Some("2026-01-10".to_string()),
                credit_amount: Some(dec!(60000000)),
                total_amount: dec!(60000000),
                ..Default::default()
            },
        ],
    );

    let res = execute_reconciliation(&session, &map).expect("Should succeed");
    let grp = &res.groups[0];
    assert_eq!(grp.status, MatchStatus::MismatchAmount);
    assert_ne!(grp.status, MatchStatus::MatchedAggregate);
}

// -------------------------------------------------------------------------------------------------
// 25. RULE-SPECIFIC TOLERANCE PRECEDENCE
// -------------------------------------------------------------------------------------------------
#[test]
fn test_25_rule_specific_tolerance_precedence() {
    let src1 = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "Sổ 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let custom_rule = ComparisonRule {
        id: "rule_high_tol".to_string(),
        name: "Doanh thu dung sai 50k".to_string(),
        semantic: ComparisonSemantic::Revenue,
        primary_source_kind: DataSourceKind::EInvoice,
        primary_field: "pretaxAmount".to_string(),
        secondary_source_kind: DataSourceKind::Ledger511,
        secondary_field: "creditAmount".to_string(),
        is_required: true,
        tolerance_vnd: dec!(50000), // Rule-specific tolerance
        date_tolerance_days: 5,
    };

    let session = ReconciliationSession {
        session_id: "sess_rule_tol_25".to_string(),
        scenario_name: "Rule Tolerance Precedence".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![custom_rule],
        matching_tolerance_vnd: dec!(0), // Session tolerance is 0!
        date_tolerance_days: 1,
        enable_aggregate_match: false,
    };

    let mut map = HashMap::new();
    map.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "inv_tol".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            date: Some("2026-01-10".to_string()),
            pretax_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        }],
    );
    map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "tk_tol".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            date: Some("2026-01-14".to_string()), // 4 days diff (<= 5 from rule)
            credit_amount: Some(dec!(10040000)),  // 40k diff (<= 50k from rule)
            total_amount: dec!(10040000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &map).expect("Should succeed");
    let grp = &res.groups[0];
    assert_eq!(grp.status, MatchStatus::MatchedWithTolerance);
}

// -------------------------------------------------------------------------------------------------
// 26. TABLE-DRIVEN ALL BUILTIN SCENARIOS
// -------------------------------------------------------------------------------------------------
#[test]
fn test_26_table_driven_all_builtin_scenarios() {
    let test_cases = vec![
        (
            DataSourceKind::EInvoice,
            DataSourceKind::Ledger511,
            ComparisonSemantic::Revenue,
        ),
        (
            DataSourceKind::EInvoice,
            DataSourceKind::Ledger3331,
            ComparisonSemantic::Vat,
        ),
        (
            DataSourceKind::EInvoice,
            DataSourceKind::Ledger131,
            ComparisonSemantic::Receivable,
        ),
        (
            DataSourceKind::EInvoice,
            DataSourceKind::Ledger133,
            ComparisonSemantic::Vat,
        ),
        (
            DataSourceKind::EInvoice,
            DataSourceKind::BankStatement,
            ComparisonSemantic::BankPayment,
        ),
        (
            DataSourceKind::Ledger131,
            DataSourceKind::BankStatement,
            ComparisonSemantic::BankPayment,
        ),
    ];

    for (pri_k, sec_k, expected_sem) in test_cases {
        let src1 = create_source(
            "src_pri",
            "Pri",
            pri_k.clone(),
            SourceRole::Primary,
            Some("Số"),
            Some("Tiền"),
        );
        let src2 = create_source(
            "src_sec",
            "Sec",
            sec_k.clone(),
            SourceRole::RequiredSecondary,
            Some("Số"),
            Some("Tiền"),
        );

        let session = ReconciliationSession {
            session_id: format!("sess_tbl_{:?}_{:?}", pri_k, sec_k),
            scenario_name: "Table Scenario".to_string(),
            primary_source_id: Some("src_pri".to_string()),
            expected_primary_kind: None,
            required_source_ids: None,
            optional_source_ids: None,
            data_sources: vec![src1, src2],
            comparison_rules: vec![],
            matching_tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
            enable_aggregate_match: false,
        };

        let mut map = HashMap::new();
        map.insert(
            "src_pri".to_string(),
            vec![CanonicalRecord {
                id: "p1".to_string(),
                source_id: "src_pri".to_string(),
                source_row: 2,
                doc_no: Some("00000001".to_string()),
                date: Some("2026-01-01".to_string()),
                pretax_amount: Some(dec!(10000000)),
                vat_amount: Some(dec!(1000000)),
                total_amount: dec!(11000000),
                credit_amount: Some(dec!(11000000)),
                ..Default::default()
            }],
        );
        map.insert(
            "src_sec".to_string(),
            vec![CanonicalRecord {
                id: "s1".to_string(),
                source_id: "src_sec".to_string(),
                source_row: 2,
                doc_no: Some("00000001".to_string()),
                date: Some("2026-01-01".to_string()),
                pretax_amount: Some(dec!(10000000)),
                credit_amount: match expected_sem {
                    ComparisonSemantic::Revenue => Some(dec!(10000000)),
                    ComparisonSemantic::Vat => Some(dec!(1000000)),
                    _ => Some(dec!(11000000)),
                },
                debit_amount: match expected_sem {
                    ComparisonSemantic::Vat => Some(dec!(1000000)),
                    _ => Some(dec!(11000000)),
                },
                total_amount: dec!(11000000),
                ..Default::default()
            }],
        );

        let res = execute_reconciliation(&session, &map).expect("Should succeed");
        assert_eq!(res.groups.len(), 1);
        assert_eq!(res.groups[0].status, MatchStatus::MatchedExact);
        assert_eq!(res.groups[0].semantic_comparisons[0].semantic, expected_sem);
    }
}

// -------------------------------------------------------------------------------------------------
// 27. RECORD CONSERVATION PROPERTY INVARIANT ACROSS ARBITRARY DATASETS
// -------------------------------------------------------------------------------------------------
#[test]
fn test_27_record_conservation_property_invariant() {
    let src1 = create_source(
        "src_inv",
        "HĐ",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("Số HĐ"),
        Some("Tiền"),
    );
    let src2 = create_source(
        "src_511",
        "TK511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );
    let src3 = create_source(
        "src_3331",
        "TK3331",
        DataSourceKind::Ledger3331,
        SourceRole::RequiredSecondary,
        Some("Số CT"),
        Some("Tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_conserv_27".to_string(),
        scenario_name: "Record Conservation Property".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2, src3],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut primary_records = Vec::new();
    for i in 1..=20 {
        primary_records.push(CanonicalRecord {
            id: format!("inv_{}", i),
            source_id: "src_inv".to_string(),
            source_row: i + 1,
            doc_no: if i == 19 || i == 20 {
                None // Records without doc_no
            } else if i == 18 {
                Some("00000018".to_string()) // Duplicate doc
            } else {
                Some(format!("{:08}", i))
            },
            series: Some("1C26TAA".to_string()),
            partner_tax_id: if i == 20 {
                None // Record with neither doc_no nor tax_id
            } else {
                Some("0109998888".to_string())
            },
            date: Some("2026-01-10".to_string()),
            pretax_amount: Some(Decimal::from(i * 1_000_000)),
            vat_amount: Some(Decimal::from(i * 100_000)),
            total_amount: Decimal::from(i * 1_100_000),
            ..Default::default()
        });
    }

    let mut tk511_records = Vec::new();
    for i in 1..=15 {
        tk511_records.push(CanonicalRecord {
            id: format!("tk511_{}", i),
            source_id: "src_511".to_string(),
            source_row: i + 1,
            doc_no: Some(format!("{:08}", i)),
            series: Some("1C26TAA".to_string()),
            date: Some("2026-01-10".to_string()),
            partner_tax_id: Some("0109998888".to_string()),
            credit_amount: Some(Decimal::from(i * 1_000_000)),
            total_amount: Decimal::from(i * 1_000_000),
            ..Default::default()
        });
    }
    // Extra secondary record
    tk511_records.push(CanonicalRecord {
        id: "tk511_extra".to_string(),
        source_id: "src_511".to_string(),
        source_row: 17,
        doc_no: Some("00000999".to_string()),
        credit_amount: Some(dec!(99000000)),
        total_amount: dec!(99000000),
        ..Default::default()
    });

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), primary_records.clone());
    map.insert("src_511".to_string(), tk511_records.clone());
    map.insert("src_3331".to_string(), vec![]);

    let res = execute_reconciliation(&session, &map).expect("Should succeed");

    // INVARIANT 1: Every single primary record ID must appear in groups EXACTLY ONCE
    let mut represented_primary_ids = Vec::new();
    for g in &res.groups {
        for pid in &g.primary_source_record_ids {
            represented_primary_ids.push(pid.clone());
        }
    }
    assert_eq!(represented_primary_ids.len(), primary_records.len());
    for p in &primary_records {
        assert!(
            represented_primary_ids.contains(&p.id),
            "Primary record {} is missing from result groups!",
            p.id
        );
    }

    // INVARIANT 2: Every unconsumed secondary record must appear in residual groups
    let mut represented_secondary_ids = Vec::new();
    for g in &res.groups {
        for sid in &g.target_source_record_ids {
            represented_secondary_ids.push(sid.clone());
        }
    }
    for s in &tk511_records {
        assert!(
            represented_secondary_ids.contains(&s.id),
            "Secondary record {} missing from groups!",
            s.id
        );
    }
}

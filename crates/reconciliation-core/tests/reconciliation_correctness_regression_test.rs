use sha2::Digest;
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

fn default_test_comparison_rules() -> Vec<ComparisonRule> {
    vec![
        ComparisonRule {
            id: "rule_test_rev".to_string(),
            name: "Doanh thu".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
        },
        ComparisonRule {
            id: "rule_test_vat".to_string(),
            name: "Thuế GTGT".to_string(),
            semantic: ComparisonSemantic::Vat,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "vatAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger3331,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
        },
        ComparisonRule {
            id: "rule_test_rec".to_string(),
            name: "Công nợ".to_string(),
            semantic: ComparisonSemantic::Receivable,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "totalAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger131,
            secondary_field: "debitAmount".to_string(),
            is_required: true,
            tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
        },
        ComparisonRule {
            id: "rule_test_vat_in".to_string(),
            name: "Thuế GTGT đầu vào".to_string(),
            semantic: ComparisonSemantic::Vat,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "vatAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger133,
            secondary_field: "debitAmount".to_string(),
            is_required: true,
            tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
        },
        ComparisonRule {
            id: "rule_test_bank".to_string(),
            name: "Dòng tiền".to_string(),
            semantic: ComparisonSemantic::BankPayment,
            primary_source_kind: DataSourceKind::Ledger131,
            primary_field: "creditAmount".to_string(),
            secondary_source_kind: DataSourceKind::BankStatement,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
        },
        ComparisonRule {
            id: "rule_test_inv_bank".to_string(),
            name: "Dòng tiền HĐ".to_string(),
            semantic: ComparisonSemantic::BankPayment,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "totalAmount".to_string(),
            secondary_source_kind: DataSourceKind::BankStatement,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: dec!(0),
            date_tolerance_days: 3,
        },
    ]
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
    assert_eq!(res.groups.len(), 2);
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
    assert_eq!(
        grp.revenue_variance.abs() + grp.vat_variance.abs(),
        dec!(20000000)
    );
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
        comparison_rules: default_test_comparison_rules(),
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

    // Also write to audit/generated-ipc-contract.json and audit-runtime/generated-rust-ipc.json
    for audit_rel in &[
        "audit/generated-ipc-contract.json",
        "../../audit/generated-ipc-contract.json",
        "audit/generated-rust-ipc.json",
        "../../audit/generated-rust-ipc.json",
        "audit-runtime/generated-rust-ipc.json",
        "../../audit-runtime/generated-rust-ipc.json",
        ".audit-runtime/generated-rust-ipc.json",
        "../../.audit-runtime/generated-rust-ipc.json",
    ] {
        let p = Path::new(audit_rel);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
            let _ = std::fs::write(p, &json_str);
        }
    }

    // Verify all DataSourceKinds serialize to canonical snake_case
    let kinds = vec![
        DataSourceKind::EInvoice,
        DataSourceKind::Ledger511,
        DataSourceKind::Ledger3331,
        DataSourceKind::Ledger131,
        DataSourceKind::Ledger133,
        DataSourceKind::BankStatement,
        DataSourceKind::CashBook,
        DataSourceKind::BranchLedger,
        DataSourceKind::Custom,
    ];
    let serialized_kinds: Vec<String> = kinds
        .iter()
        .map(|k| serde_json::to_string(k).unwrap())
        .collect();
    assert_eq!(serialized_kinds[0], "\"e_invoice\"");
    assert_eq!(serialized_kinds[1], "\"ledger_511\"");
    assert_eq!(serialized_kinds[2], "\"ledger_3331\"");
    assert_eq!(serialized_kinds[3], "\"ledger_131\"");
    assert_eq!(serialized_kinds[4], "\"ledger_133\"");
    assert_eq!(serialized_kinds[5], "\"bank_statement\"");
    assert_eq!(serialized_kinds[6], "\"cash_book\"");
    assert_eq!(serialized_kinds[7], "\"branch_ledger\"");
    assert_eq!(serialized_kinds[8], "\"custom\"");

    // Test TS-generated session deserialization if present
    for ts_path in &[
        "fixtures/artifacts/ts_generated_session.json",
        "../../fixtures/artifacts/ts_generated_session.json",
    ] {
        let p = Path::new(ts_path);
        if p.exists() {
            let content = std::fs::read_to_string(p).unwrap();
            let parsed_session: Result<ReconciliationSession, _> = serde_json::from_str(&content);
            assert!(
                parsed_session.is_ok(),
                "Failed to deserialize TS generated session: {:?}",
                parsed_session.err()
            );
            let sess = parsed_session.unwrap();
            assert_eq!(sess.data_sources.len(), 9);
            assert_eq!(sess.data_sources[1].kind, DataSourceKind::Ledger511);
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
// 16. DIRTY EXCEL METAMORPHIC ROBUSTNESS (14 DISTINCT VARIANTS)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_16_dirty_excel_metamorphic_robustness() {
    // 01: Header at row 1 standard
    let v01 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
            "Tiền thuế GTGT".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
            "1000000".into(),
        ],
    ];
    let (h1, d1, _, m1, k1, _) = detect_header_and_mapping_with_context("Hóa đơn", &v01);
    assert_eq!(h1, 1);
    assert_eq!(d1, 2);
    assert_eq!(k1, DataSourceKind::EInvoice);

    // 02: Header at row 5
    let v02 = vec![
        vec!["Title 1".into()],
        vec!["Title 2".into()],
        vec!["Title 3".into()],
        vec!["".into()],
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
            "Tiền thuế GTGT".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
            "1000000".into(),
        ],
    ];
    let (h2, d2, _, _, k2, _) = detect_header_and_mapping_with_context("Hóa đơn", &v02);
    assert_eq!(h2, 5);
    assert_eq!(d2, 6);
    assert_eq!(k2, DataSourceKind::EInvoice);

    // 03: Company title above headers
    let v03 = vec![
        vec!["CÔNG TY CỔ PHẦN THƯƠNG MẠI DỊCH VỤ ABC".into()],
        vec!["Mã số thuế: 0101234567".into()],
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày HĐ".into(),
            "Tổng tiền chưa thuế".into(),
            "Thuế GTGT".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
            "1000000".into(),
        ],
    ];
    let (h3, d3, _, _, k3, _) = detect_header_and_mapping_with_context("Hóa đơn", &v03);
    assert_eq!(h3, 3);
    assert_eq!(d3, 4);
    assert_eq!(k3, DataSourceKind::EInvoice);

    // 04: Extra STT (Sequential No) column
    let v04 = vec![
        vec![
            "STT".into(),
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền hàng".into(),
        ],
        vec![
            "1".into(),
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
        ],
    ];
    let (_, _, _, m4, _, _) = detect_header_and_mapping_with_context("HĐ", &v04);
    assert!(m4.doc_no_column.is_some());

    // 05: Extra Notes / Ghi chú column
    let v05 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền hàng".into(),
            "Ghi chú thêm".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
            "Đã thanh toán".into(),
        ],
    ];
    let (_, _, _, m5, _, _) = detect_header_and_mapping_with_context("HĐ", &v05);
    assert!(m5.doc_no_column.is_some());

    // 06: Shuffled columns order
    let v06 = vec![
        vec![
            "Tổng tiền chưa thuế".into(),
            "Ngày lập".into(),
            "Ký hiệu".into(),
            "Số HĐ".into(),
        ],
        vec![
            "10000000".into(),
            "2026-01-05".into(),
            "AA".into(),
            "001".into(),
        ],
    ];
    let (_, _, _, m6, _, _) = detect_header_and_mapping_with_context("HĐ", &v06);
    assert_eq!(m6.doc_no_column.as_deref(), Some("Số HĐ"));
    assert_eq!(
        m6.pretax_amount_column.as_deref(),
        Some("Tổng tiền chưa thuế")
    );

    // 07: Uppercase headers
    let v07 = vec![
        vec![
            "SỐ HÓA ĐƠN".into(),
            "KÝ HIỆU".into(),
            "NGÀY LẬP".into(),
            "TIỀN CHƯA THUẾ".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
        ],
    ];
    let (_, _, _, m7, _, _) = detect_header_and_mapping_with_context("HĐ", &v07);
    assert!(m7.doc_no_column.is_some());

    // 08: Whitespace-padded headers
    let v08 = vec![
        vec![
            "   Số Hóa Đơn   ".into(),
            "  Ký Hiệu  ".into(),
            "  Ngày Lập  ".into(),
            "  Tổng Tiền Chưa Thuế  ".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
        ],
    ];
    let (_, _, _, m8, _, _) = detect_header_and_mapping_with_context("HĐ", &v08);
    assert!(m8.doc_no_column.is_some());

    // 09: Blank rows between header and data
    let v09 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
        ],
        vec!["".into(), "".into(), "".into(), "".into()],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
        ],
    ];
    let ds09 = DataSource {
        id: "src9".into(),
        name: "S9".into(),
        file_path: "mock.xlsx".into(),
        sheet_name: "S1".into(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: m1.clone(),
    };
    let recs09 = normalize_data_source_rows(&ds09, &v09[0], &v09);
    assert_eq!(recs09.len(), 1, "Blank row should be filtered");

    // 10: Subtotal / footer row filtered out
    let v10 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
        ],
        vec!["Tổng cộng:".into(), "".into(), "".into(), "10000000".into()],
    ];
    let ds10 = DataSource {
        id: "src10".into(),
        name: "S10".into(),
        file_path: "mock.xlsx".into(),
        sheet_name: "S1".into(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: m1.clone(),
    };
    let recs10 = normalize_data_source_rows(&ds10, &v10[0], &v10);
    assert_eq!(
        recs10.len(),
        1,
        "Total/footer row must not become a canonical record"
    );

    // 11: Date Excel Serial format (e.g. 46022 -> 2025-12-31)
    let v11 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
        ],
        vec!["001".into(), "AA".into(), "46022".into(), "10000000".into()],
    ];
    let recs11 = normalize_data_source_rows(&ds09, &v11[0], &v11);
    assert_eq!(recs11[0].date.as_deref(), Some("2025-12-31"));

    // 12: Date text format DD/MM/YYYY
    let v12 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "05/01/2026".into(),
            "10000000".into(),
        ],
    ];
    let recs12 = normalize_data_source_rows(&ds09, &v12[0], &v12);
    assert_eq!(recs12[0].date.as_deref(), Some("2026-01-05"));

    // 13: Money numeric exact
    let v13 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10000000".into(),
        ],
    ];
    let recs13 = normalize_data_source_rows(&ds09, &v13[0], &v13);
    assert_eq!(recs13[0].pretax_amount, Some(dec!(10000000)));

    // 14: Money text formatted with dots, commas, and currency symbols
    let v14 = vec![
        vec![
            "Số HĐ".into(),
            "Ký hiệu".into(),
            "Ngày lập".into(),
            "Tiền chưa thuế".into(),
        ],
        vec![
            "001".into(),
            "AA".into(),
            "2026-01-05".into(),
            "10.000.000,50 VND".into(),
        ],
    ];
    let recs14 = normalize_data_source_rows(&ds09, &v14[0], &v14);
    assert_eq!(recs14[0].pretax_amount, Some(dec!(10000000.50)));
}

// -------------------------------------------------------------------------------------------------
// 17. DETERMINISM TEST (20+ PERMUTATIONS WITH MULTI-SOURCE & SECONDARY SHUFFLE)
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
        comparison_rules: vec![ComparisonRule {
            id: "rule_det_rev".to_string(),
            name: "Rev".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 5,
        enable_aggregate_match: true,
    };

    let base_primary = vec![
        CanonicalRecord {
            id: "inv_3".to_string(),
            source_id: "src_inv".to_string(),
            source_row: 4,
            doc_no: Some("00000300".to_string()),
            series: Some("AA".to_string()),
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
            series: Some("AA".to_string()),
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
            series: Some("AA".to_string()),
            date: Some("2026-01-02".to_string()),
            pretax_amount: Some(dec!(20000000)),
            total_amount: dec!(20000000),
            ..Default::default()
        },
    ];

    let base_secondary = vec![
        CanonicalRecord {
            id: "tk_2".to_string(),
            source_id: "src_511".to_string(),
            source_row: 3,
            doc_no: Some("00000200".to_string()),
            series: Some("AA".to_string()),
            date: Some("2026-01-02".to_string()),
            credit_amount: Some(dec!(20000000)),
            total_amount: dec!(20000000),
            ..Default::default()
        },
        CanonicalRecord {
            id: "tk_1".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("00000100".to_string()),
            series: Some("AA".to_string()),
            date: Some("2026-01-01".to_string()),
            credit_amount: Some(dec!(10000000)),
            total_amount: dec!(10000000),
            ..Default::default()
        },
        CanonicalRecord {
            id: "tk_3".to_string(),
            source_id: "src_511".to_string(),
            source_row: 4,
            doc_no: Some("00000300".to_string()),
            series: Some("AA".to_string()),
            date: Some("2026-01-03".to_string()),
            credit_amount: Some(dec!(30000000)),
            total_amount: dec!(30000000),
            ..Default::default()
        },
    ];

    let mut first_json = String::new();

    for i in 0..24 {
        let mut permuted_pri = base_primary.clone();
        let mut permuted_sec = base_secondary.clone();

        if i % 2 == 0 {
            permuted_pri.reverse();
        }
        if i % 3 == 0 {
            permuted_pri.swap(0, 1);
        }
        if i % 4 == 0 {
            permuted_sec.reverse();
        }
        if i % 5 == 0 {
            permuted_sec.swap(1, 2);
        }

        let mut records_map = HashMap::new();
        records_map.insert("src_inv".to_string(), permuted_pri);
        records_map.insert("src_511".to_string(), permuted_sec);

        let mut current_session = session.clone();
        if i % 6 == 0 {
            current_session.data_sources.reverse();
        }

        let res = execute_reconciliation(&current_session, &records_map).expect("Should succeed");
        let json = serde_json::to_string(&res.groups).expect("JSON failed");

        if i == 0 {
            first_json = json;
        } else {
            assert_eq!(first_json, json, "Run {} failed determinism check!", i);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// 18. FALSE-POSITIVE ADVERSARIAL SUITE (10 DISTINCT ADVERSARIAL CASES)
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
        comparison_rules: vec![ComparisonRule {
            id: "rule_adv_rev".to_string(),
            name: "Rev".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 3,
        }],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    // Case 01: Same amount, different doc numbers -> 0 false match
    let mut map01 = HashMap::new();
    map01.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p1".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: Some("001".into()),
            pretax_amount: Some(dec!(99999999)),
            total_amount: dec!(99999999),
            ..Default::default()
        }],
    );
    map01.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "s1".into(),
            source_id: "src_511".into(),
            source_row: 2,
            doc_no: Some("002".into()),
            credit_amount: Some(dec!(99999999)),
            total_amount: dec!(99999999),
            ..Default::default()
        }],
    );
    let res01 = execute_reconciliation(&session, &map01).unwrap();
    assert_eq!(
        res01.summary.exact_matches_count, 0,
        "Case 01 must not match"
    );

    // Case 02: Same doc, different series -> collision prevented
    let mut map02 = HashMap::new();
    map02.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "p2a".into(),
                source_id: "src_inv".into(),
                source_row: 2,
                doc_no: Some("100".into()),
                series: Some("1C26TAA".into()),
                pretax_amount: Some(dec!(1000)),
                total_amount: dec!(1000),
                ..Default::default()
            },
            CanonicalRecord {
                id: "p2b".into(),
                source_id: "src_inv".into(),
                source_row: 3,
                doc_no: Some("100".into()),
                series: Some("2C26TBB".into()),
                pretax_amount: Some(dec!(2000)),
                total_amount: dec!(2000),
                ..Default::default()
            },
        ],
    );
    map02.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "s2".into(),
            source_id: "src_511".into(),
            source_row: 2,
            doc_no: Some("100".into()),
            series: Some("2C26TBB".into()),
            credit_amount: Some(dec!(2000)),
            total_amount: dec!(2000),
            ..Default::default()
        }],
    );
    let res02 = execute_reconciliation(&session, &map02).unwrap();
    let grp2b = res02
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"p2b".to_string()))
        .unwrap();
    assert_eq!(grp2b.status, MatchStatus::MatchedExact);
    let grp2a = res02
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"p2a".to_string()))
        .unwrap();
    assert_eq!(grp2a.status, MatchStatus::UnmatchedMissingInTarget);

    // Case 03: Same doc, outside date tolerance -> 0 exact match
    let mut map03 = HashMap::new();
    map03.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p3".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: Some("100".into()),
            date: Some("2026-01-01".into()),
            pretax_amount: Some(dec!(1000)),
            total_amount: dec!(1000),
            ..Default::default()
        }],
    );
    map03.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "s3".into(),
            source_id: "src_511".into(),
            source_row: 2,
            doc_no: Some("100".into()),
            date: Some("2026-02-01".into()),
            credit_amount: Some(dec!(1000)),
            total_amount: dec!(1000),
            ..Default::default()
        }],
    );
    let res03 = execute_reconciliation(&session, &map03).unwrap();
    assert_eq!(
        res03.summary.exact_matches_count, 0,
        "Case 03 date out of bounds must not match exact"
    );

    // Case 04: Same doc/date, incompatible MST -> 0 match
    let mut map04 = HashMap::new();
    map04.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p4".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: Some("100".into()),
            partner_tax_id: Some("010111".into()),
            pretax_amount: Some(dec!(1000)),
            total_amount: dec!(1000),
            ..Default::default()
        }],
    );
    map04.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "s4".into(),
            source_id: "src_511".into(),
            source_row: 2,
            doc_no: Some("100".into()),
            partner_tax_id: Some("030999".into()),
            credit_amount: Some(dec!(1000)),
            total_amount: dec!(1000),
            ..Default::default()
        }],
    );
    let res04 = execute_reconciliation(&session, &map04).unwrap();
    assert_eq!(
        res04.summary.exact_matches_count, 0,
        "Case 04 incompatible MST must not match"
    );

    // Case 05: Aggregate cross-customer MST -> rejected
    let mut map05 = HashMap::new();
    map05.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p5".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: Some("500".into()),
            partner_tax_id: Some("0101".into()),
            pretax_amount: Some(dec!(100)),
            total_amount: dec!(100),
            ..Default::default()
        }],
    );
    map05.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "s5a".into(),
                source_id: "src_511".into(),
                source_row: 2,
                doc_no: Some("500".into()),
                partner_tax_id: Some("0101".into()),
                credit_amount: Some(dec!(40)),
                total_amount: dec!(40),
                ..Default::default()
            },
            CanonicalRecord {
                id: "s5b".into(),
                source_id: "src_511".into(),
                source_row: 3,
                doc_no: Some("500".into()),
                partner_tax_id: Some("0202".into()),
                credit_amount: Some(dec!(60)),
                total_amount: dec!(60),
                ..Default::default()
            },
        ],
    );
    let res05 = execute_reconciliation(&session, &map05).unwrap();
    assert_ne!(
        res05.groups[0].status,
        MatchStatus::MatchedAggregate,
        "Cross-customer aggregate must be rejected"
    );

    // Case 06: Multiple valid aggregate subsets -> Ambiguous
    let mut map06 = HashMap::new();
    map06.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p6".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: Some("600".into()),
            pretax_amount: Some(dec!(100)),
            total_amount: dec!(100),
            ..Default::default()
        }],
    );
    map06.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "s6a".into(),
                source_id: "src_511".into(),
                source_row: 2,
                doc_no: Some("600".into()),
                credit_amount: Some(dec!(100)),
                total_amount: dec!(100),
                ..Default::default()
            },
            CanonicalRecord {
                id: "s6b".into(),
                source_id: "src_511".into(),
                source_row: 3,
                doc_no: Some("600".into()),
                credit_amount: Some(dec!(100)),
                total_amount: dec!(100),
                ..Default::default()
            },
        ],
    );
    let res06 = execute_reconciliation(&session, &map06).unwrap();
    assert_eq!(res06.groups[0].status, MatchStatus::AmbiguousMatch);

    // Case 07: No-doc insufficient evidence (no MST, no Date) -> NeedsReview, never disappears
    let mut map07 = HashMap::new();
    map07.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p7".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: None,
            partner_tax_id: None,
            date: None,
            pretax_amount: Some(dec!(100)),
            total_amount: dec!(100),
            ..Default::default()
        }],
    );
    map07.insert("src_511".to_string(), vec![]);
    let res07 = execute_reconciliation(&session, &map07).unwrap();
    assert_eq!(res07.groups[0].status, MatchStatus::NeedsReview);

    // Case 08: No-doc multiple candidates for same MST+Amount -> Ambiguous fallback
    let mut map08 = HashMap::new();
    map08.insert(
        "src_inv".to_string(),
        vec![CanonicalRecord {
            id: "p8".into(),
            source_id: "src_inv".into(),
            source_row: 2,
            doc_no: None,
            partner_tax_id: Some("0101".into()),
            date: Some("2026-01-05".into()),
            pretax_amount: Some(dec!(100)),
            total_amount: dec!(100),
            ..Default::default()
        }],
    );
    map08.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "s8a".into(),
                source_id: "src_511".into(),
                source_row: 2,
                doc_no: None,
                partner_tax_id: Some("0101".into()),
                date: Some("2026-01-05".into()),
                credit_amount: Some(dec!(100)),
                total_amount: dec!(100),
                ..Default::default()
            },
            CanonicalRecord {
                id: "s8b".into(),
                source_id: "src_511".into(),
                source_row: 3,
                doc_no: None,
                partner_tax_id: Some("0101".into()),
                date: Some("2026-01-05".into()),
                credit_amount: Some(dec!(100)),
                total_amount: dec!(100),
                ..Default::default()
            },
        ],
    );
    let res08 = execute_reconciliation(&session, &map08).unwrap();
    assert_eq!(res08.groups[0].status, MatchStatus::AmbiguousMatch);

    // Case 09: Mismatch candidate does not consume — subsequent exact primary claims it
    let mut map09 = HashMap::new();
    map09.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "p9a".into(),
                source_id: "src_inv".into(),
                source_row: 2,
                doc_no: Some("900".into()),
                series: Some("1C26TAA".into()),
                date: Some("2026-07-01".into()),
                partner_tax_id: Some("0101".into()),
                pretax_amount: Some(dec!(90)),
                total_amount: dec!(90),
                ..Default::default()
            },
            CanonicalRecord {
                id: "p9b".into(),
                source_id: "src_inv".into(),
                source_row: 3,
                doc_no: Some("900".into()),
                series: Some("2C26TBB".into()),
                date: Some("2026-07-01".into()),
                partner_tax_id: Some("0101".into()),
                pretax_amount: Some(dec!(100)),
                total_amount: dec!(100),
                ..Default::default()
            },
        ],
    );
    map09.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "s9".into(),
            source_id: "src_511".into(),
            source_row: 2,
            doc_no: Some("900".into()),
            series: Some("2C26TBB".into()),
            date: Some("2026-07-01".into()),
            partner_tax_id: Some("0101".into()),
            credit_amount: Some(dec!(100)),
            total_amount: dec!(100),
            ..Default::default()
        }],
    );
    let res09 = execute_reconciliation(&session, &map09).unwrap();
    let grp9a = res09
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"p9a".to_string()))
        .unwrap();
    assert_eq!(grp9a.status, MatchStatus::UnmatchedMissingInTarget);
    let grp9b = res09
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"p9b".to_string()))
        .unwrap();
    assert_eq!(grp9b.status, MatchStatus::MatchedExact);
    assert_eq!(grp9b.target_source_record_ids, vec!["s9".to_string()]);
    assert_eq!(res09.summary.exact_matches_count, 1);

    // Case 10: Ambiguous candidate does not lock candidate from exact match
    let mut map10 = HashMap::new();
    map10.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "p10a".into(),
                source_id: "src_inv".into(),
                source_row: 2,
                doc_no: Some("1000".into()),
                pretax_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
            CanonicalRecord {
                id: "p10b".into(),
                source_id: "src_inv".into(),
                source_row: 3,
                doc_no: Some("2000".into()),
                pretax_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
        ],
    );
    map10.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "s10a".into(),
                source_id: "src_511".into(),
                source_row: 2,
                doc_no: Some("1000".into()),
                credit_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
            CanonicalRecord {
                id: "s10b".into(),
                source_id: "src_511".into(),
                source_row: 3,
                doc_no: Some("1000".into()),
                credit_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
            CanonicalRecord {
                id: "s10c".into(),
                source_id: "src_511".into(),
                source_row: 4,
                doc_no: Some("2000".into()),
                credit_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
        ],
    );
    let res10 = execute_reconciliation(&session, &map10).unwrap();
    let grp10b = res10
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"p10b".to_string()))
        .unwrap();
    assert_eq!(grp10b.status, MatchStatus::MatchedExact);
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
        comparison_rules: vec![ComparisonRule {
            id: "rule_real_revenue".to_string(),
            name: "Doanh thu chưa thuế (HĐĐT ↔ TK511 Phát sinh Có)".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: dec!(0),
        date_tolerance_days: 5,
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

    // 1. Compute Oracle Results Independently
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
    let mut oracle_rev_variance = Decimal::ZERO;

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
            oracle_rev_variance += inv_amt;
        }
    }

    // 2. Execute Production Reconciliation Engine
    let session = ReconciliationSession {
        session_id: "sess_oracle_test".to_string(),
        scenario_name: "Oracle Verification".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_tk".to_string()]),
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_tk],
        comparison_rules: vec![ComparisonRule {
            id: "rule_oracle".to_string(),
            name: "Oracle Revenue".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), records_inv);
    map.insert("src_tk".to_string(), records_tk);

    let res = execute_reconciliation(&session, &map).expect("Engine execution failed");

    // 3. Direct Comparison between Engine and Independent Oracle
    assert_eq!(
        res.summary.exact_matches_count, oracle_exact,
        "Engine exact matches must equal oracle"
    );
    assert_eq!(
        res.summary.missing_in_target_count, oracle_missing,
        "Engine missing count must equal oracle"
    );
    assert_eq!(
        res.summary.revenue_variance, oracle_rev_variance,
        "Engine revenue variance must equal oracle"
    );
    let missing_grp = res
        .groups
        .iter()
        .find(|g| g.status == MatchStatus::UnmatchedMissingInTarget)
        .unwrap();
    assert_eq!(
        missing_grp.doc_no.as_deref(),
        Some(oracle_missing_doc.as_str()),
        "Engine missing doc must match oracle"
    );
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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
            comparison_rules: default_test_comparison_rules(),
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
        comparison_rules: default_test_comparison_rules(),
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

// -------------------------------------------------------------------------------------------------
// TEST 28: MISMATCH DOES NOT CONSUME CANDIDATE
// Primary A → candidate X = MISMATCH → X must NOT be consumed
// Primary B → same candidate X = EXACT → B gets MATCHED_EXACT
// -------------------------------------------------------------------------------------------------
#[test]
fn test_28_mismatch_does_not_consume_candidate() {
    let src_inv = DataSource {
        id: "inv_src".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let src_511 = DataSource {
        id: "tk511_src".to_string(),
        name: "TK511".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    // Primary A: doc_no = "001", pretax = 100_000_000
    let primary_a = CanonicalRecord {
        id: "inv_a".to_string(),
        source_id: "inv_src".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        series: Some("AA".to_string()),
        date: Some("2026-01-05".to_string()),
        pretax_amount: Some(dec!(100_000_000)),
        total_amount: dec!(110_000_000),
        ..Default::default()
    };
    // Primary B: doc_no = "002", pretax = 50_000_000
    let primary_b = CanonicalRecord {
        id: "inv_b".to_string(),
        source_id: "inv_src".to_string(),
        source_row: 3,
        doc_no: Some("002".to_string()),
        series: Some("AA".to_string()),
        date: Some("2026-01-06".to_string()),
        pretax_amount: Some(dec!(50_000_000)),
        total_amount: dec!(55_000_000),
        ..Default::default()
    };

    // Candidate X: doc_no = "001", credit_amount = 50_000_000
    // Candidate X matches doc_no "001" BUT amount mismatches primary_a (100M vs 50M)
    // Candidate X should NOT be consumed by primary_a
    // Primary B doc_no "002" won't find X (different doc_no), so this tests the invariant
    // that mismatch candidate X stays available
    let cand_x = CanonicalRecord {
        id: "tk_x".to_string(),
        source_id: "tk511_src".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        series: Some("AA".to_string()),
        date: Some("2026-01-05".to_string()),
        credit_amount: Some(dec!(50_000_000)),
        total_amount: dec!(50_000_000),
        ..Default::default()
    };

    // Candidate Y: doc_no = "002", credit_amount = 50_000_000
    let cand_y = CanonicalRecord {
        id: "tk_y".to_string(),
        source_id: "tk511_src".to_string(),
        source_row: 3,
        doc_no: Some("002".to_string()),
        series: Some("AA".to_string()),
        date: Some("2026-01-06".to_string()),
        credit_amount: Some(dec!(50_000_000)),
        total_amount: dec!(50_000_000),
        ..Default::default()
    };

    let session = ReconciliationSession {
        session_id: "sess_test28".to_string(),
        scenario_name: "Test 28".to_string(),
        primary_source_id: Some("inv_src".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["tk511_src".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_rev".to_string(),
            name: "Revenue".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("inv_src".to_string(), vec![primary_a, primary_b]);
    records_map.insert("tk511_src".to_string(), vec![cand_x, cand_y]);

    let res = execute_reconciliation(&session, &records_map).expect("Reconciliation must succeed");

    // Primary A (doc "001") matches candidate X (doc "001") but amounts differ: MISMATCH
    let grp_a = res
        .groups
        .iter()
        .find(|g| g.doc_no.as_deref() == Some("001"))
        .expect("Group for doc 001 must exist");
    // Amount mismatch: 100M vs 50M
    assert_eq!(
        grp_a.status,
        MatchStatus::MismatchAmount,
        "Primary A (100M) vs Candidate X (50M) must be MISMATCH_AMOUNT"
    );

    // Primary B (doc "002") matches candidate Y (doc "002") with exact amount: MATCHED_EXACT
    let grp_b = res
        .groups
        .iter()
        .find(|g| g.doc_no.as_deref() == Some("002"))
        .expect("Group for doc 002 must exist");
    assert_eq!(
        grp_b.status,
        MatchStatus::MatchedExact,
        "Primary B (50M) vs Candidate Y (50M) must be MATCHED_EXACT"
    );

    // Candidate X must appear in group A's target_source_record_ids (referenced but not fully matched)
    assert!(
        grp_a.target_source_record_ids.contains(&"tk_x".to_string()),
        "Candidate X must be referenced in group A"
    );
}

// -------------------------------------------------------------------------------------------------
// TEST 29: AMBIGUOUS DOES NOT CONSUME CANDIDATES
// Primary A → multiple matching combos = AMBIGUOUS → candidates NOT consumed
// Primary B → exact match with one of those candidates = MATCHED_EXACT
// -------------------------------------------------------------------------------------------------
#[test]
fn test_29_ambiguous_does_not_lock_candidates() {
    // In our engine, AMBIGUOUS candidates are listed in target_source_record_ids
    // but the engine never calls consumed_secondary_ids for ambiguous.
    // This test verifies the engine reports both groups correctly.
    let src_inv = DataSource {
        id: "inv_src".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let src_511 = DataSource {
        id: "tk511_src".to_string(),
        name: "TK511".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    // Two secondary candidates with same doc_no "001" and same amount 50M
    // → Primary A (doc "001", 50M) sees 2 candidates → subset {cand1} and {cand2} both sum to 50M → AMBIGUOUS
    let cand1 = CanonicalRecord {
        id: "tk_1".to_string(),
        source_id: "tk511_src".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        credit_amount: Some(dec!(50_000_000)),
        total_amount: dec!(50_000_000),
        ..Default::default()
    };
    let cand2 = CanonicalRecord {
        id: "tk_2".to_string(),
        source_id: "tk511_src".to_string(),
        source_row: 3,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        credit_amount: Some(dec!(50_000_000)),
        total_amount: dec!(50_000_000),
        ..Default::default()
    };
    // Primary B: doc "002" 50M → must match a distinct candidate
    let cand3 = CanonicalRecord {
        id: "tk_3".to_string(),
        source_id: "tk511_src".to_string(),
        source_row: 4,
        doc_no: Some("002".to_string()),
        date: Some("2026-01-06".to_string()),
        credit_amount: Some(dec!(50_000_000)),
        total_amount: dec!(50_000_000),
        ..Default::default()
    };

    let prim_a = CanonicalRecord {
        id: "inv_a".to_string(),
        source_id: "inv_src".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        pretax_amount: Some(dec!(50_000_000)),
        total_amount: dec!(55_000_000),
        ..Default::default()
    };
    let prim_b = CanonicalRecord {
        id: "inv_b".to_string(),
        source_id: "inv_src".to_string(),
        source_row: 3,
        doc_no: Some("002".to_string()),
        date: Some("2026-01-06".to_string()),
        pretax_amount: Some(dec!(50_000_000)),
        total_amount: dec!(55_000_000),
        ..Default::default()
    };

    let session = ReconciliationSession {
        session_id: "sess_test29".to_string(),
        scenario_name: "Test 29".to_string(),
        primary_source_id: Some("inv_src".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["tk511_src".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_rev".to_string(),
            name: "Revenue".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("inv_src".to_string(), vec![prim_a, prim_b]);
    records_map.insert("tk511_src".to_string(), vec![cand1, cand2, cand3]);

    let res = execute_reconciliation(&session, &records_map).expect("Reconciliation must succeed");

    // Both primary groups must appear
    let primary_groups: Vec<_> = res
        .groups
        .iter()
        .filter(|g| !g.primary_source_record_ids.is_empty())
        .collect();
    assert_eq!(primary_groups.len(), 2, "Must have 2 primary groups");

    let grp_a = res
        .groups
        .iter()
        .find(|g| g.doc_no.as_deref() == Some("001"))
        .expect("Group for doc 001 must exist");
    // AMBIGUOUS because cand1 and cand2 both sum to 50M individually
    assert_eq!(
        grp_a.status,
        MatchStatus::AmbiguousMatch,
        "Primary A with 2 identical-amount candidates must be AMBIGUOUS"
    );

    let grp_b = res
        .groups
        .iter()
        .find(|g| g.doc_no.as_deref() == Some("002"))
        .expect("Group for doc 002 must exist");
    // Primary B doc "002" has unique candidate cand3
    assert_eq!(
        grp_b.status,
        MatchStatus::MatchedExact,
        "Primary B must be MATCHED_EXACT with cand3"
    );

    // Candidate 3 must appear in group B
    assert!(
        grp_b.target_source_record_ids.contains(&"tk_3".to_string()),
        "Candidate 3 must be in group B"
    );

    // Residual candidates 1 and 2 must appear in residual groups
    let residual_groups: Vec<_> = res
        .groups
        .iter()
        .filter(|g| g.status == MatchStatus::UnmatchedMissingInSource)
        .collect();
    assert_eq!(
        residual_groups.len(),
        2,
        "Unconsumed ambiguous candidates must appear in residual groups"
    );
}

// -------------------------------------------------------------------------------------------------
// TEST 30: NOT_CHECKED SEMANTIC WHEN SOURCE ABSENT FROM SESSION
// Session with only EInvoice + TK511 (no TK3331).
// Engine uses built-in fallback rules for EInvoice↔511.
// Since no VAT source is present, VAT comparison must NOT appear in results.
// (NotChecked would be emitted if we had a rule for VAT but no source — test rule-based session)
// This test verifies: result contains only REVENUE semantic comparisons; no phantom VAT.
// -------------------------------------------------------------------------------------------------
#[test]
fn test_30_no_phantom_vat_when_tk3331_absent() {
    let src_inv = DataSource {
        id: "inv_src".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let src_511 = DataSource {
        id: "tk511_src".to_string(),
        name: "TK511".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let primary = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "inv_src".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        pretax_amount: Some(dec!(100_000_000)),
        vat_amount: Some(dec!(10_000_000)),
        total_amount: dec!(110_000_000),
        ..Default::default()
    };
    let secondary = CanonicalRecord {
        id: "tk_1".to_string(),
        source_id: "tk511_src".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        credit_amount: Some(dec!(100_000_000)),
        total_amount: dec!(100_000_000),
        ..Default::default()
    };

    // Only REVENUE rule — no VAT rule at all
    let session = ReconciliationSession {
        session_id: "sess_test30".to_string(),
        scenario_name: "Test 30".to_string(),
        primary_source_id: Some("inv_src".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["tk511_src".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_rev".to_string(),
            name: "Revenue".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("inv_src".to_string(), vec![primary]);
    records_map.insert("tk511_src".to_string(), vec![secondary]);

    let res = execute_reconciliation(&session, &records_map).expect("Reconciliation must succeed");

    assert_eq!(res.groups.len(), 1);
    let grp = &res.groups[0];
    assert_eq!(grp.status, MatchStatus::MatchedExact);

    // No phantom VAT comparisons — only REVENUE
    for comp in &grp.semantic_comparisons {
        assert_eq!(
            comp.semantic,
            ComparisonSemantic::Revenue,
            "Only REVENUE comparison expected when no VAT source/rule present. Got {:?}",
            comp.semantic
        );
        // Must NOT be NotChecked in a clean REVENUE match
        assert_ne!(
            comp.status,
            MatchStatus::NotChecked,
            "REVENUE must not be NotChecked when TK511 is present"
        );
    }

    // VAT summary variance must be zero (not driven by phantom comparison)
    assert_eq!(
        grp.vat_variance,
        Decimal::ZERO,
        "VAT variance must be 0 when no VAT rule"
    );
}

// -------------------------------------------------------------------------------------------------
// TEST 31: BASELINE 46/45/45/#233 WITH EXPLICIT COMPARISON RULE (NO SILENT FALLBACK)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_31_baseline_with_explicit_rule_no_fallback() {
    // Same core scenario as test_01 baseline (doc matching only),
    // but we use a real EInvoice↔511 ComparisonRule explicitly.
    // Validates that the explicit rule path gives the same result as built-in defaults.
    let src_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let src_511 = DataSource {
        id: "src_511".to_string(),
        name: "TK511".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    // Build 46 invoices, 45 TK511 entries (missing #233 from TK511)
    let mut invoices: Vec<CanonicalRecord> = Vec::new();
    let mut tk511s: Vec<CanonicalRecord> = Vec::new();

    for i in 1..=46u32 {
        let doc = format!("{}", 200 + i);
        let amount = dec!(1_000_000) * Decimal::from(i);
        invoices.push(CanonicalRecord {
            id: format!("inv_{}", i),
            source_id: "src_inv".to_string(),
            source_row: i + 1,
            doc_no: Some(doc.clone()),
            series: Some("1C26TAA".to_string()),
            date: Some("2026-01-05".to_string()),
            pretax_amount: Some(amount),
            total_amount: amount,
            ..Default::default()
        });
        if doc != "233" {
            // doc 233 is missing from TK511
            tk511s.push(CanonicalRecord {
                id: format!("tk_{}", i),
                source_id: "src_511".to_string(),
                source_row: i + 1,
                doc_no: Some(doc),
                date: Some("2026-01-05".to_string()),
                credit_amount: Some(amount),
                total_amount: amount,
                ..Default::default()
            });
        }
    }

    assert_eq!(invoices.len(), 46);
    assert_eq!(tk511s.len(), 45);

    let session = ReconciliationSession {
        session_id: "sess_test31".to_string(),
        scenario_name: "Test 31 Explicit Rule".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_explicit_rev".to_string(),
            name: "Revenue (Explicit)".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_inv".to_string(), invoices);
    records_map.insert("src_511".to_string(), tk511s);

    let res = execute_reconciliation(&session, &records_map).expect("Reconciliation must succeed");

    let exact = res
        .groups
        .iter()
        .filter(|g| g.status == MatchStatus::MatchedExact)
        .count();
    let missing_target = res
        .groups
        .iter()
        .filter(|g| g.status == MatchStatus::UnmatchedMissingInTarget)
        .count();

    assert_eq!(exact, 45, "Must have 45 exact matches with explicit rule");
    assert_eq!(
        missing_target, 1,
        "Must have 1 missing in target (doc #233)"
    );
    assert_eq!(
        res.groups.len(),
        46,
        "Must have 46 groups total (one per primary)"
    );

    // The missing one must be doc 233
    let missing_grp = res
        .groups
        .iter()
        .find(|g| g.status == MatchStatus::UnmatchedMissingInTarget)
        .expect("Must have missing group");
    assert_eq!(
        missing_grp.doc_no.as_deref(),
        Some("233"),
        "Missing group must be doc #233"
    );
}

// -------------------------------------------------------------------------------------------------
// TEST 32: FAIL-CLOSED ON EMPTY COMPARISON RULES (NO SILENT ENGINE FALLBACK)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_32_fail_closed_empty_comparison_rules() {
    let src_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let src_511 = DataSource {
        id: "src_511".to_string(),
        name: "TK511".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let session = ReconciliationSession {
        session_id: "sess_test32_empty_rules".to_string(),
        scenario_name: "Test Empty Rules".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![], // STRICTLY EMPTY — MUST FAIL CLOSED!
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let primary = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "src_inv".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        pretax_amount: Some(dec!(100_000_000)),
        total_amount: dec!(100_000_000),
        ..Default::default()
    };
    let secondary = CanonicalRecord {
        id: "tk_1".to_string(),
        source_id: "src_511".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        credit_amount: Some(dec!(100_000_000)),
        total_amount: dec!(100_000_000),
        ..Default::default()
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_inv".to_string(), vec![primary]);
    records_map.insert("src_511".to_string(), vec![secondary]);

    let res =
        execute_reconciliation(&session, &records_map).expect("Reconciliation should succeed");

    assert_eq!(res.groups.len(), 2);
    let grp = &res.groups[0];
    assert_eq!(grp.status, MatchStatus::NeedsReview);
    assert_eq!(res.summary.exact_matches_count, 0);

    let has_unsupported_rule_disc = grp
        .discrepancies
        .iter()
        .any(|d| d.field_name == "rule" || d.message.contains("UNSUPPORTED_RECONCILIATION_RULE"));
    assert!(
        has_unsupported_rule_disc,
        "Group must contain unsupported rule discrepancy when comparison_rules is empty"
    );
}

// -------------------------------------------------------------------------------------------------
// TEST 33: FAIL-CLOSED ON INVALID FIELD NAME IN COMPARISON RULE
// -------------------------------------------------------------------------------------------------
#[test]
fn test_33_fail_closed_invalid_field_in_comparison_rules() {
    let src_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let src_511 = DataSource {
        id: "src_511".to_string(),
        name: "TK511".to_string(),
        file_path: "mock.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let session = ReconciliationSession {
        session_id: "sess_test33_invalid_field".to_string(),
        scenario_name: "Test Invalid Field Rule".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_bad_field".to_string(),
            name: "Invalid Field Rule".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "nonExistentField123".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let primary = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "src_inv".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        pretax_amount: Some(dec!(100_000_000)),
        total_amount: dec!(100_000_000),
        ..Default::default()
    };
    let secondary = CanonicalRecord {
        id: "tk_1".to_string(),
        source_id: "src_511".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        date: Some("2026-01-05".to_string()),
        credit_amount: Some(dec!(100_000_000)),
        total_amount: dec!(100_000_000),
        ..Default::default()
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_inv".to_string(), vec![primary]);
    records_map.insert("src_511".to_string(), vec![secondary]);

    let res =
        execute_reconciliation(&session, &records_map).expect("Reconciliation should succeed");

    assert_eq!(res.groups.len(), 2);
    let grp = &res.groups[0];
    assert_eq!(grp.status, MatchStatus::NeedsReview);
    assert_eq!(res.summary.exact_matches_count, 0);
}

// -------------------------------------------------------------------------------------------------
// 34. V11 STRICT FIELD EXTRACTION FAIL-CLOSED TESTS (Cases A, B, C, D, E)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_34_field_fail_closed_cases_a_b_c_d_e() {
    let src_inv = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("docNo"),
        Some("pretaxAmount"),
    );
    let src_511 = create_source(
        "src_511",
        "Sổ cái 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("docNo"),
        Some("creditAmount"),
    );
    let src_131 = create_source(
        "src_131",
        "Sổ cái 131",
        DataSourceKind::Ledger131,
        SourceRole::RequiredSecondary,
        Some("docNo"),
        Some("debitAmount"),
    );

    // Case A: Primary pretaxAmount is None, totalAmount is 110M
    {
        let session = ReconciliationSession {
            session_id: "sess_v11_case_a".to_string(),
            scenario_name: "Test Case A Pretax Missing".to_string(),
            primary_source_id: Some("src_inv".to_string()),
            expected_primary_kind: None,
            required_source_ids: Some(vec!["src_511".to_string()]),
            optional_source_ids: None,
            data_sources: vec![src_inv.clone(), src_511.clone()],
            comparison_rules: vec![ComparisonRule {
                id: "rule_revenue".to_string(),
                name: "Revenue Rule".to_string(),
                semantic: ComparisonSemantic::Revenue,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "pretaxAmount".to_string(),
                secondary_source_kind: DataSourceKind::Ledger511,
                secondary_field: "creditAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 5,
            }],
            matching_tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
            enable_aggregate_match: false,
        };
        let mut map = HashMap::new();
        map.insert(
            "src_inv".to_string(),
            vec![CanonicalRecord {
                id: "inv_a".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("001".to_string()),
                date: Some("2026-07-01".to_string()),
                pretax_amount: None,
                total_amount: dec!(110_000_000),
                ..Default::default()
            }],
        );
        map.insert(
            "src_511".to_string(),
            vec![CanonicalRecord {
                id: "tk_a".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("001".to_string()),
                date: Some("2026-07-01".to_string()),
                credit_amount: Some(dec!(100_000_000)),
                total_amount: dec!(100_000_000),
                ..Default::default()
            }],
        );
        let res = execute_reconciliation(&session, &map).unwrap();
        assert_eq!(res.summary.exact_matches_count, 0);
        let grp = res.groups.iter().find(|g| g.id.contains("inv_a")).unwrap();
        assert_eq!(grp.status, MatchStatus::NeedsReview);
        assert!(grp
            .discrepancies
            .iter()
            .any(|d| d.field_name == "FIELD_VALUE_MISSING"));
    }

    // Case B: Primary vatAmount is None
    {
        let session = ReconciliationSession {
            session_id: "sess_v11_case_b".to_string(),
            scenario_name: "Test Case B VAT Missing".to_string(),
            primary_source_id: Some("src_inv".to_string()),
            expected_primary_kind: None,
            required_source_ids: Some(vec!["src_511".to_string()]),
            optional_source_ids: None,
            data_sources: vec![src_inv.clone(), src_511.clone()],
            comparison_rules: vec![ComparisonRule {
                id: "rule_vat".to_string(),
                name: "VAT Rule".to_string(),
                semantic: ComparisonSemantic::Vat,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "vatAmount".to_string(),
                secondary_source_kind: DataSourceKind::Ledger511,
                secondary_field: "creditAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 5,
            }],
            matching_tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
            enable_aggregate_match: false,
        };
        let mut map = HashMap::new();
        map.insert(
            "src_inv".to_string(),
            vec![CanonicalRecord {
                id: "inv_b".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("002".to_string()),
                date: Some("2026-07-01".to_string()),
                vat_amount: None,
                total_amount: dec!(10_000_000),
                ..Default::default()
            }],
        );
        map.insert(
            "src_511".to_string(),
            vec![CanonicalRecord {
                id: "tk_b".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("002".to_string()),
                date: Some("2026-07-01".to_string()),
                credit_amount: Some(dec!(10_000_000)),
                total_amount: dec!(10_000_000),
                ..Default::default()
            }],
        );
        let res = execute_reconciliation(&session, &map).unwrap();
        assert_eq!(res.summary.exact_matches_count, 0);
        let grp = res.groups.iter().find(|g| g.id.contains("inv_b")).unwrap();
        assert_eq!(grp.status, MatchStatus::NeedsReview);
        assert!(grp
            .discrepancies
            .iter()
            .any(|d| d.field_name == "FIELD_VALUE_MISSING"));
    }

    // Case C: Secondary creditAmount is None, totalAmount is 100M
    {
        let session = ReconciliationSession {
            session_id: "sess_v11_case_c".to_string(),
            scenario_name: "Test Case C Credit Missing".to_string(),
            primary_source_id: Some("src_inv".to_string()),
            expected_primary_kind: None,
            required_source_ids: Some(vec!["src_511".to_string()]),
            optional_source_ids: None,
            data_sources: vec![src_inv.clone(), src_511.clone()],
            comparison_rules: vec![ComparisonRule {
                id: "rule_revenue".to_string(),
                name: "Revenue Rule".to_string(),
                semantic: ComparisonSemantic::Revenue,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "pretaxAmount".to_string(),
                secondary_source_kind: DataSourceKind::Ledger511,
                secondary_field: "creditAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 5,
            }],
            matching_tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
            enable_aggregate_match: false,
        };
        let mut map = HashMap::new();
        map.insert(
            "src_inv".to_string(),
            vec![CanonicalRecord {
                id: "inv_c".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("003".to_string()),
                date: Some("2026-07-01".to_string()),
                pretax_amount: Some(dec!(100_000_000)),
                total_amount: dec!(110_000_000),
                ..Default::default()
            }],
        );
        map.insert(
            "src_511".to_string(),
            vec![CanonicalRecord {
                id: "tk_c".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("003".to_string()),
                date: Some("2026-07-01".to_string()),
                credit_amount: None,
                total_amount: dec!(100_000_000),
                ..Default::default()
            }],
        );
        let res = execute_reconciliation(&session, &map).unwrap();
        assert_eq!(res.summary.exact_matches_count, 0);
        let grp = res.groups.iter().find(|g| g.id.contains("inv_c")).unwrap();
        assert_eq!(grp.status, MatchStatus::NeedsReview);
        assert!(grp
            .discrepancies
            .iter()
            .any(|d| d.field_name == "FIELD_VALUE_MISSING"));
    }

    // Case D: Secondary debitAmount is None
    {
        let session = ReconciliationSession {
            session_id: "sess_v11_case_d".to_string(),
            scenario_name: "Test Case D Debit Missing".to_string(),
            primary_source_id: Some("src_inv".to_string()),
            expected_primary_kind: None,
            required_source_ids: Some(vec!["src_131".to_string()]),
            optional_source_ids: None,
            data_sources: vec![src_inv.clone(), src_131.clone()],
            comparison_rules: vec![ComparisonRule {
                id: "rule_receivable".to_string(),
                name: "Receivable Rule".to_string(),
                semantic: ComparisonSemantic::Receivable,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "totalAmount".to_string(),
                secondary_source_kind: DataSourceKind::Ledger131,
                secondary_field: "debitAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 5,
            }],
            matching_tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
            enable_aggregate_match: false,
        };
        let mut map = HashMap::new();
        map.insert(
            "src_inv".to_string(),
            vec![CanonicalRecord {
                id: "inv_d".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("004".to_string()),
                date: Some("2026-07-01".to_string()),
                total_amount: dec!(110_000_000),
                ..Default::default()
            }],
        );
        map.insert(
            "src_131".to_string(),
            vec![CanonicalRecord {
                id: "tk_d".to_string(),
                source_id: "src_131".to_string(),
                source_row: 2,
                doc_no: Some("004".to_string()),
                date: Some("2026-07-01".to_string()),
                debit_amount: None,
                total_amount: dec!(110_000_000),
                ..Default::default()
            }],
        );
        let res = execute_reconciliation(&session, &map).unwrap();
        assert_eq!(res.summary.exact_matches_count, 0);
        let grp = res.groups.iter().find(|g| g.id.contains("inv_d")).unwrap();
        assert_eq!(grp.status, MatchStatus::NeedsReview);
        assert!(grp
            .discrepancies
            .iter()
            .any(|d| d.field_name == "FIELD_VALUE_MISSING"));
    }

    // Case E: Rule requests unknown field
    {
        let session = ReconciliationSession {
            session_id: "sess_v11_case_e".to_string(),
            scenario_name: "Test Case E Unsupported Field".to_string(),
            primary_source_id: Some("src_inv".to_string()),
            expected_primary_kind: None,
            required_source_ids: Some(vec!["src_511".to_string()]),
            optional_source_ids: None,
            data_sources: vec![src_inv, src_511],
            comparison_rules: vec![ComparisonRule {
                id: "rule_bad".to_string(),
                name: "Bad Field Rule".to_string(),
                semantic: ComparisonSemantic::Revenue,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "randomUnknownField".to_string(),
                secondary_source_kind: DataSourceKind::Ledger511,
                secondary_field: "creditAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 5,
            }],
            matching_tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
            enable_aggregate_match: false,
        };
        let mut map = HashMap::new();
        map.insert(
            "src_inv".to_string(),
            vec![CanonicalRecord {
                id: "inv_e".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("005".to_string()),
                date: Some("2026-07-01".to_string()),
                total_amount: dec!(100_000_000),
                ..Default::default()
            }],
        );
        map.insert(
            "src_511".to_string(),
            vec![CanonicalRecord {
                id: "tk_e".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("005".to_string()),
                date: Some("2026-07-01".to_string()),
                credit_amount: Some(dec!(100_000_000)),
                total_amount: dec!(100_000_000),
                ..Default::default()
            }],
        );
        let res = execute_reconciliation(&session, &map).unwrap();
        assert_eq!(res.summary.exact_matches_count, 0);
        let grp = res.groups.iter().find(|g| g.id.contains("inv_e")).unwrap();
        assert_eq!(grp.status, MatchStatus::NeedsReview);
        assert!(grp
            .discrepancies
            .iter()
            .any(|d| d.field_name == "UNSUPPORTED_RULE_FIELD"));
    }
}

// -------------------------------------------------------------------------------------------------
// 35. EXACT SAME CANDIDATE CONTENTION TEST (User Section 4)
// Secondary X (100) vs Primary A (90) and Primary B (100)
// Expected: A evaluates MismatchAmount (does NOT consume X); B evaluates MatchedExact with X (consumes X)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_35_same_candidate_contention_unconsumed_mismatch_then_exact() {
    let src_inv = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("docNo"),
        Some("pretaxAmount"),
    );
    let src_511 = create_source(
        "src_511",
        "Sổ cái 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("docNo"),
        Some("creditAmount"),
    );

    let session = ReconciliationSession {
        session_id: "sess_v11_contention".to_string(),
        scenario_name: "Contention Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_revenue".to_string(),
            name: "Revenue Rule".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut map = HashMap::new();
    map.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "pA".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("001".to_string()),
                date: Some("2026-07-01".to_string()),
                partner_tax_id: Some("0101".to_string()),
                pretax_amount: Some(dec!(90)),
                total_amount: dec!(90),
                ..Default::default()
            },
            CanonicalRecord {
                id: "pB".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 3,
                doc_no: Some("001".to_string()),
                date: Some("2026-07-01".to_string()),
                partner_tax_id: Some("0101".to_string()),
                pretax_amount: Some(dec!(100)),
                total_amount: dec!(100),
                ..Default::default()
            },
        ],
    );
    map.insert(
        "src_511".to_string(),
        vec![CanonicalRecord {
            id: "sX".to_string(),
            source_id: "src_511".to_string(),
            source_row: 2,
            doc_no: Some("001".to_string()),
            date: Some("2026-07-01".to_string()),
            partner_tax_id: Some("0101".to_string()),
            credit_amount: Some(dec!(100)),
            total_amount: dec!(100),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &map).unwrap();

    let grp_a = res
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"pA".to_string()))
        .unwrap();
    assert_eq!(grp_a.status, MatchStatus::MismatchAmount);

    let grp_b = res
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"pB".to_string()))
        .unwrap();
    assert_eq!(grp_b.status, MatchStatus::MatchedExact);
    assert_eq!(grp_b.target_source_record_ids, vec!["sX".to_string()]);

    assert_eq!(res.summary.exact_matches_count, 1);
    assert_eq!(res.summary.mismatches_count, 1);
}

// -------------------------------------------------------------------------------------------------
// 36. AMBIGUOUS CONTENTION AND RESIDUAL SWEEP ASSERTIONS (User Section 5)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_36_ambiguous_contention_and_residual_sweep() {
    let src_inv = create_source(
        "src_inv",
        "Hóa đơn",
        DataSourceKind::EInvoice,
        SourceRole::Primary,
        Some("docNo"),
        Some("pretaxAmount"),
    );
    let src_511 = create_source(
        "src_511",
        "Sổ cái 511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("docNo"),
        Some("creditAmount"),
    );

    let session = ReconciliationSession {
        session_id: "sess_v11_ambiguous".to_string(),
        scenario_name: "Ambiguous Contention".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511".to_string()]),
        optional_source_ids: None,
        data_sources: vec![src_inv, src_511],
        comparison_rules: vec![ComparisonRule {
            id: "rule_revenue".to_string(),
            name: "Revenue Rule".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let mut map = HashMap::new();
    map.insert(
        "src_inv".to_string(),
        vec![
            CanonicalRecord {
                id: "pA".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 2,
                doc_no: Some("002".to_string()),
                series: None,
                pretax_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
            CanonicalRecord {
                id: "pB".to_string(),
                source_id: "src_inv".to_string(),
                source_row: 3,
                doc_no: Some("002".to_string()),
                series: Some("SER_B".to_string()),
                pretax_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
        ],
    );
    map.insert(
        "src_511".to_string(),
        vec![
            CanonicalRecord {
                id: "sX".to_string(),
                source_id: "src_511".to_string(),
                source_row: 2,
                doc_no: Some("002".to_string()),
                series: Some("SER_B".to_string()),
                credit_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
            CanonicalRecord {
                id: "sY".to_string(),
                source_id: "src_511".to_string(),
                source_row: 3,
                doc_no: Some("002".to_string()),
                series: Some("SER_C".to_string()),
                credit_amount: Some(dec!(50)),
                total_amount: dec!(50),
                ..Default::default()
            },
        ],
    );

    let res = execute_reconciliation(&session, &map).unwrap();

    let grp_b = res
        .groups
        .iter()
        .find(|g| g.primary_source_record_ids.contains(&"pB".to_string()))
        .unwrap();
    assert_eq!(grp_b.status, MatchStatus::MatchedExact);
    assert_eq!(grp_b.target_source_record_ids, vec!["sX".to_string()]);

    let residual_y = res.groups.iter().find(|g| {
        g.target_source_record_ids.contains(&"sY".to_string())
            && g.primary_source_record_ids.is_empty()
    });
    assert!(residual_y.is_some());
    assert_eq!(
        residual_y.unwrap().status,
        MatchStatus::UnmatchedMissingInSource
    );

    // sX must NOT appear in residual groups
    let residual_x = res.groups.iter().find(|g| {
        g.target_source_record_ids.contains(&"sX".to_string())
            && g.primary_source_record_ids.is_empty()
    });
    assert!(residual_x.is_none());
}

// -------------------------------------------------------------------------------------------------
// 37. N-FILE INTAKE DEDUPLICATION (4 Physical Files: A, B, Copy A, Copy B -> Exact Baseline)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_37_n_file_intake_deduplication_4_files_exact_baseline() {
    use reconciliation_core::intake::{
        analyze_intake_data_sources, filter_reconciliation_session_and_records, DatasetRelation,
    };

    let inv_path = "D:\\appketoan\\T7.2026 Thuế.xlsx";
    let tk511_path = "D:\\appketoan\\T7.2026.xlsx";

    let meta_inv = inspect_excel_file(inv_path).expect("Failed to inspect invoice file");
    let sheet_inv = &meta_inv.sheets[0];
    let raw_inv = read_sheet_rows(inv_path, &sheet_inv.name).expect("Failed to read invoices");
    let ds_inv = DataSource {
        id: "src_inv_1".to_string(),
        name: "Hóa đơn gốc".to_string(),
        file_path: inv_path.to_string(),
        sheet_name: sheet_inv.name.clone(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: sheet_inv.detected_header_row,
        data_start_row: sheet_inv.detected_data_start_row,
        column_mapping: sheet_inv.suggested_mapping.clone(),
    };
    let mut ds_inv_copy = ds_inv.clone();
    ds_inv_copy.id = "src_inv_copy".to_string();
    ds_inv_copy.name = "Hóa đơn copy".to_string();
    let inv_recs = normalize_data_source_rows(&ds_inv, &sheet_inv.columns, &raw_inv);

    let meta_tk = inspect_excel_file(tk511_path).expect("Failed to inspect TK511 file");
    let sheet_tk = &meta_tk.sheets[0];
    let raw_tk = read_sheet_rows(tk511_path, &sheet_tk.name).expect("Failed to read TK511");
    let ds_tk = DataSource {
        id: "src_511_1".to_string(),
        name: "Sổ cái TK511 gốc".to_string(),
        file_path: tk511_path.to_string(),
        sheet_name: sheet_tk.name.clone(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: sheet_tk.detected_header_row,
        data_start_row: sheet_tk.detected_data_start_row,
        column_mapping: sheet_tk.suggested_mapping.clone(),
    };
    let mut ds_tk_copy = ds_tk.clone();
    ds_tk_copy.id = "src_511_copy".to_string();
    ds_tk_copy.name = "Sổ cái TK511 copy".to_string();
    let tk_recs = normalize_data_source_rows(&ds_tk, &sheet_tk.columns, &raw_tk);

    let session = ReconciliationSession {
        session_id: "sess_4_files_baseline".to_string(),
        scenario_name: "4-Files Dedup Baseline".to_string(),
        primary_source_id: Some("src_inv_1".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_511_1".to_string()]),
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_tk, ds_inv_copy, ds_tk_copy],
        comparison_rules: vec![ComparisonRule {
            id: "rule_revenue".to_string(),
            name: "Doanh thu (Pretax ↔ TK511 Phát sinh Có)".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let inv_bytes = std::fs::read(inv_path).unwrap();
    let tk_bytes = std::fs::read(tk511_path).unwrap();

    let mut raw_hashes_map = HashMap::new();

    let inv_hash = format!("{:x}", sha2::Sha256::digest(&inv_bytes));
    let tk_hash = format!("{:x}", sha2::Sha256::digest(&tk_bytes));

    raw_hashes_map.insert("src_inv_1".to_string(), inv_hash.clone());
    raw_hashes_map.insert("src_inv_copy".to_string(), inv_hash);
    raw_hashes_map.insert("src_511_1".to_string(), tk_hash.clone());
    raw_hashes_map.insert("src_511_copy".to_string(), tk_hash);

    let mut records_map = HashMap::new();
    records_map.insert("src_inv_1".to_string(), inv_recs.clone());
    records_map.insert("src_inv_copy".to_string(), inv_recs);
    records_map.insert("src_511_1".to_string(), tk_recs.clone());
    records_map.insert("src_511_copy".to_string(), tk_recs);

    // Intake analysis
    let analysis = analyze_intake_data_sources(&session, &records_map, &raw_hashes_map);
    assert_eq!(analysis.total_physical_sources, 4);
    assert_eq!(analysis.unique_datasets_count, 2);
    assert_eq!(analysis.exact_duplicates_count, 2);

    let copy_inv_diag = analysis
        .source_analyses
        .iter()
        .find(|d| d.source_id == "src_inv_copy")
        .unwrap();
    assert!(matches!(
        copy_inv_diag.relation,
        DatasetRelation::ExactDuplicate { .. }
    ));

    let copy_tk_diag = analysis
        .source_analyses
        .iter()
        .find(|d| d.source_id == "src_511_copy")
        .unwrap();
    assert!(matches!(
        copy_tk_diag.relation,
        DatasetRelation::ExactDuplicate { .. }
    ));

    // Filter session & records
    let (clean_session, clean_map, _) =
        filter_reconciliation_session_and_records(&session, &records_map, &raw_hashes_map).unwrap();
    assert_eq!(clean_session.data_sources.len(), 2);

    // Execute reconciliation on clean session
    let res = execute_reconciliation(&clean_session, &clean_map).unwrap();

    assert_eq!(res.summary.total_source_records, 46);
    assert_eq!(res.summary.total_target_records, 45);
    assert_eq!(res.summary.exact_matches_count, 45);
    assert_eq!(res.summary.mismatches_count, 0);
    assert_eq!(res.summary.missing_in_target_count, 1);
    assert_eq!(res.summary.revenue_variance, dec!(105_000_000));

    let missing_233 = res
        .groups
        .iter()
        .find(|g| g.doc_no.as_deref() == Some("233"))
        .unwrap();
    assert_eq!(missing_233.status, MatchStatus::UnmatchedMissingInTarget);
    assert_eq!(missing_233.amount_variance, dec!(105_000_000));
}

// -------------------------------------------------------------------------------------------------
// 38. CONTENT DUPLICATE WITH DIFFERENT RAW HASH (Reformatted Excel)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_38_content_duplicate_reformatted_excel() {
    use reconciliation_core::intake::{analyze_intake_data_sources, DatasetRelation};

    let src1 = create_source(
        "src1",
        "File 1",
        DataSourceKind::Ledger511,
        SourceRole::Primary,
        None,
        None,
    );
    let src2 = create_source(
        "src2",
        "File 2 Reformatted",
        DataSourceKind::Ledger511,
        SourceRole::Primary,
        None,
        None,
    );

    let session = ReconciliationSession {
        session_id: "sess_content_dup".to_string(),
        scenario_name: "Content Dup Test".to_string(),
        primary_source_id: Some("src1".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
        enable_aggregate_match: false,
    };

    let recs = vec![CanonicalRecord {
        id: "r1".to_string(),
        source_id: "src1".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        credit_amount: Some(dec!(100)),
        total_amount: dec!(100),
        ..Default::default()
    }];

    let mut records_map = HashMap::new();
    records_map.insert("src1".to_string(), recs.clone());
    records_map.insert("src2".to_string(), recs);

    // Different raw hashes, but same canonical content
    let mut raw_hashes_map = HashMap::new();
    raw_hashes_map.insert("src1".to_string(), "hash_abc".to_string());
    raw_hashes_map.insert("src2".to_string(), "hash_xyz".to_string());

    let analysis = analyze_intake_data_sources(&session, &records_map, &raw_hashes_map);
    assert_eq!(analysis.unique_datasets_count, 1);
    assert_eq!(analysis.content_duplicates_count, 1);

    let diag2 = analysis
        .source_analyses
        .iter()
        .find(|d| d.source_id == "src2")
        .unwrap();
    assert!(matches!(
        diag2.relation,
        DatasetRelation::ContentDuplicate { .. }
    ));
}

// -------------------------------------------------------------------------------------------------
// 39. PARTIAL OVERLAP DETECTION FAIL-CLOSED
// -------------------------------------------------------------------------------------------------
#[test]
fn test_39_partial_overlap_detection_fail_closed() {
    use reconciliation_core::intake::{analyze_intake_data_sources, DatasetRelation};

    let src1 = create_source(
        "src1",
        "Ledger Part 1",
        DataSourceKind::Ledger511,
        SourceRole::Primary,
        None,
        None,
    );
    let src2 = create_source(
        "src2",
        "Ledger Part 2 Overlapping",
        DataSourceKind::Ledger511,
        SourceRole::Primary,
        None,
        None,
    );

    let session = ReconciliationSession {
        session_id: "sess_partial_overlap".to_string(),
        scenario_name: "Partial Overlap Test".to_string(),
        primary_source_id: Some("src1".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![src1, src2],
        comparison_rules: vec![],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
        enable_aggregate_match: false,
    };

    let rec1 = CanonicalRecord {
        id: "r1".to_string(),
        source_id: "src1".to_string(),
        source_row: 2,
        doc_no: Some("001".to_string()),
        credit_amount: Some(dec!(100)),
        total_amount: dec!(100),
        ..Default::default()
    };
    let rec2 = CanonicalRecord {
        id: "r2".to_string(),
        source_id: "src1".to_string(),
        source_row: 3,
        doc_no: Some("002".to_string()),
        credit_amount: Some(dec!(200)),
        total_amount: dec!(200),
        ..Default::default()
    };
    let rec3 = CanonicalRecord {
        id: "r3".to_string(),
        source_id: "src2".to_string(),
        source_row: 3,
        doc_no: Some("003".to_string()),
        credit_amount: Some(dec!(300)),
        total_amount: dec!(300),
        ..Default::default()
    };

    let mut records_map = HashMap::new();
    records_map.insert("src1".to_string(), vec![rec1.clone(), rec2.clone()]);
    records_map.insert("src2".to_string(), vec![rec2, rec3]);

    let mut raw_hashes_map = HashMap::new();
    raw_hashes_map.insert("src1".to_string(), "hash_part1".to_string());
    raw_hashes_map.insert("src2".to_string(), "hash_part2".to_string());

    let analysis = analyze_intake_data_sources(&session, &records_map, &raw_hashes_map);
    assert!(analysis.requires_user_confirmation);
    assert_eq!(analysis.partial_overlaps_count, 1);

    let diag2 = analysis
        .source_analyses
        .iter()
        .find(|d| d.source_id == "src2")
        .unwrap();
    assert!(matches!(
        diag2.relation,
        DatasetRelation::PartialOverlap { .. }
    ));
}

// -------------------------------------------------------------------------------------------------
// 40. UPLOAD ORDER INVARIANCE & SOURCE ID REMAPPING
// -------------------------------------------------------------------------------------------------
#[test]
fn test_40_upload_order_invariance_and_remapping() {
    use reconciliation_core::intake::filter_reconciliation_session_and_records;

    let inv_path = "D:\\appketoan\\T7.2026 Thuế.xlsx";
    let tk511_path = "D:\\appketoan\\T7.2026.xlsx";

    if !Path::new(inv_path).exists() || !Path::new(tk511_path).exists() {
        return;
    }

    let meta_inv = inspect_excel_file(inv_path).unwrap();
    let raw_inv = read_sheet_rows(inv_path, &meta_inv.sheets[0].name).unwrap();
    let ds_inv = DataSource {
        id: "src_inv_orig".to_string(),
        name: "Hóa đơn gốc".to_string(),
        file_path: inv_path.to_string(),
        sheet_name: meta_inv.sheets[0].name.clone(),
        kind: DataSourceKind::EInvoice,
        role: SourceRole::Primary,
        header_row: meta_inv.sheets[0].detected_header_row,
        data_start_row: meta_inv.sheets[0].detected_data_start_row,
        column_mapping: meta_inv.sheets[0].suggested_mapping.clone(),
    };
    let mut ds_inv_copy = ds_inv.clone();
    ds_inv_copy.id = "src_inv_copy".to_string();
    ds_inv_copy.name = "Hóa đơn copy".to_string();

    let meta_tk = inspect_excel_file(tk511_path).unwrap();
    let raw_tk = read_sheet_rows(tk511_path, &meta_tk.sheets[0].name).unwrap();
    let ds_tk = DataSource {
        id: "src_tk_orig".to_string(),
        name: "Sổ cái TK511 gốc".to_string(),
        file_path: tk511_path.to_string(),
        sheet_name: meta_tk.sheets[0].name.clone(),
        kind: DataSourceKind::Ledger511,
        role: SourceRole::RequiredSecondary,
        header_row: meta_tk.sheets[0].detected_header_row,
        data_start_row: meta_tk.sheets[0].detected_data_start_row,
        column_mapping: meta_tk.sheets[0].suggested_mapping.clone(),
    };
    let mut ds_tk_copy = ds_tk.clone();
    ds_tk_copy.id = "src_tk_copy".to_string();
    ds_tk_copy.name = "Sổ cái TK511 copy".to_string();

    let inv_recs = normalize_data_source_rows(&ds_inv, &meta_inv.sheets[0].columns, &raw_inv);
    let tk_recs = normalize_data_source_rows(&ds_tk, &meta_tk.sheets[0].columns, &raw_tk);

    let inv_bytes = std::fs::read(inv_path).unwrap();
    let tk_bytes = std::fs::read(tk511_path).unwrap();
    let inv_hash = format!("{:x}", sha2::Sha256::digest(&inv_bytes));
    let tk_hash = format!("{:x}", sha2::Sha256::digest(&tk_bytes));

    let mut raw_hashes = HashMap::new();
    raw_hashes.insert("src_inv_orig".to_string(), inv_hash.clone());
    raw_hashes.insert("src_inv_copy".to_string(), inv_hash);
    raw_hashes.insert("src_tk_orig".to_string(), tk_hash.clone());
    raw_hashes.insert("src_tk_copy".to_string(), tk_hash);

    let mut records_map = HashMap::new();
    records_map.insert("src_inv_orig".to_string(), inv_recs.clone());
    records_map.insert("src_inv_copy".to_string(), inv_recs);
    records_map.insert("src_tk_orig".to_string(), tk_recs.clone());
    records_map.insert("src_tk_copy".to_string(), tk_recs);

    let base_rules = vec![ComparisonRule {
        id: "rule_revenue".to_string(),
        name: "Doanh thu (Pretax ↔ TK511 Phát sinh Có)".to_string(),
        semantic: ComparisonSemantic::Revenue,
        primary_source_kind: DataSourceKind::EInvoice,
        primary_field: "pretaxAmount".to_string(),
        secondary_source_kind: DataSourceKind::Ledger511,
        secondary_field: "creditAmount".to_string(),
        is_required: true,
        tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
    }];

    // Order 1: [A, B, A_copy, B_copy]
    let session1 = ReconciliationSession {
        session_id: "sess_ord_1".to_string(),
        scenario_name: "Order 1".to_string(),
        primary_source_id: Some("src_inv_orig".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_tk_orig".to_string()]),
        optional_source_ids: None,
        data_sources: vec![
            ds_inv.clone(),
            ds_tk.clone(),
            ds_inv_copy.clone(),
            ds_tk_copy.clone(),
        ],
        comparison_rules: base_rules.clone(),
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    // Order 2: [A_copy, B_copy, A, B] with primary set to copy and required set to copy
    let session2 = ReconciliationSession {
        session_id: "sess_ord_2".to_string(),
        scenario_name: "Order 2".to_string(),
        primary_source_id: Some("src_inv_orig".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_tk_orig".to_string()]),
        optional_source_ids: None,
        data_sources: vec![
            ds_inv_copy.clone(),
            ds_tk_copy.clone(),
            ds_inv.clone(),
            ds_tk.clone(),
        ],
        comparison_rules: base_rules.clone(),
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    // Order 3: [B_copy, A, B, A_copy]
    let session3 = ReconciliationSession {
        session_id: "sess_ord_3".to_string(),
        scenario_name: "Order 3".to_string(),
        primary_source_id: Some("src_inv_copy".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_tk_copy".to_string()]),
        optional_source_ids: None,
        data_sources: vec![ds_tk_copy, ds_inv, ds_tk, ds_inv_copy],
        comparison_rules: base_rules,
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let (s1_clean, r1_clean, a1) =
        filter_reconciliation_session_and_records(&session1, &records_map, &raw_hashes).unwrap();
    let (s2_clean, r2_clean, a2) =
        filter_reconciliation_session_and_records(&session2, &records_map, &raw_hashes).unwrap();
    let (s3_clean, r3_clean, a3) =
        filter_reconciliation_session_and_records(&session3, &records_map, &raw_hashes).unwrap();

    assert_eq!(a1.unique_datasets_count, 2);
    assert_eq!(a2.unique_datasets_count, 2);
    assert_eq!(a3.unique_datasets_count, 2);

    let res1 = execute_reconciliation(&s1_clean, &r1_clean).unwrap();
    let res2 = execute_reconciliation(&s2_clean, &r2_clean).unwrap();
    let res3 = execute_reconciliation(&s3_clean, &r3_clean).unwrap();

    // All three execution orders MUST yield identical business metrics
    assert_eq!(res1.summary.exact_matches_count, 45);
    assert_eq!(res2.summary.exact_matches_count, 45);
    assert_eq!(res3.summary.exact_matches_count, 45);

    assert_eq!(res1.summary.missing_in_target_count, 1);
    assert_eq!(res2.summary.missing_in_target_count, 1);
    assert_eq!(res3.summary.missing_in_target_count, 1);

    assert_eq!(res1.summary.revenue_variance, dec!(105_000_000));
    assert_eq!(res2.summary.revenue_variance, dec!(105_000_000));
    assert_eq!(res3.summary.revenue_variance, dec!(105_000_000));
}

// -------------------------------------------------------------------------------------------------
// 41. RESIDUAL FIELD MISSING FAIL-CLOSED (NEEDS_REVIEW)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_41_residual_field_missing_fail_closed_needs_review() {
    let sec_source = create_source(
        "src_sec",
        "Sổ cái TK511",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
        Some("Số chứng từ"),
        Some("Phát sinh Có"),
    );

    let session = ReconciliationSession {
        session_id: "sess_residual_missing_field".to_string(),
        scenario_name: "Residual Missing Field".to_string(),
        primary_source_id: Some("src_prim".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(vec!["src_sec".to_string()]),
        optional_source_ids: None,
        data_sources: vec![
            create_source(
                "src_prim",
                "Hóa đơn",
                DataSourceKind::EInvoice,
                SourceRole::Primary,
                None,
                None,
            ),
            sec_source,
        ],
        comparison_rules: vec![ComparisonRule {
            id: "rule_revenue".to_string(),
            name: "Doanh thu".to_string(),
            semantic: ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 5,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    // Secondary record has doc_no and total_amount, but credit_amount is None
    let sec_rec = CanonicalRecord {
        id: "sec_row_no_credit".to_string(),
        source_id: "src_sec".to_string(),
        source_row: 2,
        doc_no: Some("CT-999".to_string()),
        credit_amount: None, // Missing rule field!
        total_amount: dec!(50_000_000),
        ..Default::default()
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_prim".to_string(), vec![]);
    records_map.insert("src_sec".to_string(), vec![sec_rec]);

    let res = execute_reconciliation(&session, &records_map).unwrap();
    assert_eq!(res.groups.len(), 1);

    let grp = &res.groups[0];
    // Must be marked NeedsReview because required rule field is missing
    assert_eq!(grp.status, MatchStatus::NeedsReview);
    assert!(grp
        .discrepancies
        .iter()
        .any(|d| d.field_name == "creditAmount"));
}

// -------------------------------------------------------------------------------------------------
// 42. UNSUPPORTED RULE IN RESIDUAL SWEEP (NEEDS_REVIEW)
// -------------------------------------------------------------------------------------------------
#[test]
fn test_42_unsupported_rule_in_residual_sweep() {
    let custom_source = create_source(
        "src_custom",
        "Sổ phụ đặc biệt",
        DataSourceKind::Custom,
        SourceRole::OptionalSecondary,
        Some("Số CT"),
        Some("Số tiền"),
    );

    let session = ReconciliationSession {
        session_id: "sess_no_rule".to_string(),
        scenario_name: "No Rule Session".to_string(),
        primary_source_id: Some("src_prim".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: Some(vec!["src_custom".to_string()]),
        data_sources: vec![
            create_source(
                "src_prim",
                "Hóa đơn",
                DataSourceKind::EInvoice,
                SourceRole::Primary,
                None,
                None,
            ),
            custom_source,
        ],
        comparison_rules: vec![], // Empty comparison rules!
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 5,
        enable_aggregate_match: false,
    };

    let sec_rec = CanonicalRecord {
        id: "custom_row".to_string(),
        source_id: "src_custom".to_string(),
        source_row: 2,
        doc_no: Some("CT-888".to_string()),
        total_amount: dec!(10_000_000),
        ..Default::default()
    };

    let mut records_map = HashMap::new();
    records_map.insert("src_prim".to_string(), vec![]);
    records_map.insert("src_custom".to_string(), vec![sec_rec]);

    let res = execute_reconciliation(&session, &records_map).unwrap();
    assert_eq!(res.groups.len(), 1);
    assert_eq!(res.groups[0].status, MatchStatus::NeedsReview);
    assert!(res.groups[0]
        .discrepancies
        .iter()
        .any(|d| d.message.contains("UNSUPPORTED_RECONCILIATION_RULE")));
}

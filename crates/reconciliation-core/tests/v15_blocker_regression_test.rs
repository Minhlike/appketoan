use std::collections::HashMap;

use reconciliation_core::{
    evaluate_partner_identity_cross_source, evaluate_partner_master_control,
    evaluate_sales_analysis_control, execute_reconciliation, execute_reference_controls,
    invoice_lifecycle_for_record, CanonicalRecord, ColumnMapping, ComparisonRule,
    ComparisonSemantic, DataSource, DataSourceKind, InvoiceLifecycle, MatchStatus, PartnerRecord,
    ReconciliationSession, SalesAnalysisRecord, SourceRole,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn source(id: &str, kind: DataSourceKind, role: SourceRole) -> DataSource {
    DataSource {
        id: id.to_string(),
        name: id.to_string(),
        file_path: "synthetic.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind,
        role,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    }
}

fn record(id: &str, source_id: &str, doc_no: Option<&str>) -> CanonicalRecord {
    CanonicalRecord {
        id: id.to_string(),
        source_id: source_id.to_string(),
        source_row: 2,
        doc_no: doc_no.map(str::to_string),
        date: Some("2026-06-15".to_string()),
        partner_tax_id: Some("0100000000".to_string()),
        pretax_amount: Some(dec!(100)),
        vat_amount: Some(dec!(10)),
        total_amount: dec!(110),
        ..Default::default()
    }
}

fn rule(
    id: &str,
    semantic: ComparisonSemantic,
    primary_kind: DataSourceKind,
    secondary_kind: DataSourceKind,
    primary_field: &str,
    secondary_field: &str,
) -> ComparisonRule {
    ComparisonRule {
        id: id.to_string(),
        name: id.to_string(),
        semantic,
        primary_source_kind: primary_kind,
        primary_field: primary_field.to_string(),
        secondary_source_kind: secondary_kind,
        secondary_field: secondary_field.to_string(),
        is_required: true,
        tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
    }
}

fn session(
    sources: Vec<DataSource>,
    rules: Vec<ComparisonRule>,
    aggregate: bool,
) -> ReconciliationSession {
    ReconciliationSession {
        session_id: "v15".to_string(),
        scenario_name: "v15".to_string(),
        primary_source_id: Some("primary".to_string()),
        expected_primary_kind: None,
        required_source_ids: Some(sources.iter().map(|source| source.id.clone()).collect()),
        optional_source_ids: None,
        data_sources: sources,
        comparison_rules: rules,
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
        enable_aggregate_match: aggregate,
    }
}

#[test]
fn execute_reconciliation_rejects_opposite_bank_direction_for_exact_fallback_and_aggregate() {
    let ledger = source("primary", DataSourceKind::Ledger112, SourceRole::Primary);
    let bank = source(
        "bank",
        DataSourceKind::BankStatement,
        SourceRole::RequiredSecondary,
    );
    let payment_rule = rule(
        "bank",
        ComparisonSemantic::BankPayment,
        DataSourceKind::Ledger112,
        DataSourceKind::BankStatement,
        "directionalAmount",
        "directionalAmount",
    );

    let mut exact_ledger = record("ledger_exact", "primary", Some("1"));
    exact_ledger.debit_amount = Some(dec!(100));
    let mut exact_bank = record("bank_exact", "bank", Some("1"));
    exact_bank.debit_amount = Some(dec!(100));
    exact_bank.total_amount = dec!(100);
    let result = execute_reconciliation(
        &session(
            vec![ledger.clone(), bank.clone()],
            vec![payment_rule.clone()],
            false,
        ),
        &HashMap::from([
            ("primary".to_string(), vec![exact_ledger]),
            ("bank".to_string(), vec![exact_bank]),
        ]),
    )
    .expect("execution");
    assert!(result.groups.iter().all(|group| {
        !matches!(
            group.status,
            MatchStatus::MatchedExact
                | MatchStatus::MatchedWithTolerance
                | MatchStatus::MatchedAggregate
        )
    }));

    let mut tolerance_rule = payment_rule.clone();
    tolerance_rule.tolerance_vnd = dec!(1);
    let mut tolerance_ledger = record("ledger_tolerance", "primary", Some("2"));
    tolerance_ledger.debit_amount = Some(dec!(100));
    let mut tolerance_bank = record("bank_tolerance", "bank", Some("2"));
    tolerance_bank.debit_amount = Some(dec!(101));
    tolerance_bank.total_amount = dec!(101);
    let tolerance = execute_reconciliation(
        &session(
            vec![ledger.clone(), bank.clone()],
            vec![tolerance_rule],
            false,
        ),
        &HashMap::from([
            ("primary".to_string(), vec![tolerance_ledger]),
            ("bank".to_string(), vec![tolerance_bank]),
        ]),
    )
    .expect("tolerance execution");
    assert_eq!(tolerance.summary.tolerance_matches_count, 0);

    let mut fallback_ledger = record("ledger_fallback", "primary", None);
    fallback_ledger.debit_amount = Some(dec!(100));
    let mut fallback_bank = record("bank_fallback", "bank", None);
    fallback_bank.debit_amount = Some(dec!(100));
    fallback_bank.total_amount = dec!(100);
    let fallback = execute_reconciliation(
        &session(
            vec![ledger.clone(), bank.clone()],
            vec![payment_rule.clone()],
            false,
        ),
        &HashMap::from([
            ("primary".to_string(), vec![fallback_ledger]),
            ("bank".to_string(), vec![fallback_bank]),
        ]),
    )
    .expect("fallback execution");
    assert_eq!(fallback.summary.exact_matches_count, 0);

    let mut aggregate_ledger = record("ledger_aggregate", "primary", Some("3"));
    aggregate_ledger.debit_amount = Some(dec!(100));
    let mut bank_a = record("bank_a", "bank", Some("3"));
    bank_a.debit_amount = Some(dec!(40));
    bank_a.total_amount = dec!(40);
    let mut bank_b = record("bank_b", "bank", Some("3"));
    bank_b.debit_amount = Some(dec!(60));
    bank_b.total_amount = dec!(60);
    let aggregate = execute_reconciliation(
        &session(vec![ledger, bank], vec![payment_rule], true),
        &HashMap::from([
            ("primary".to_string(), vec![aggregate_ledger]),
            ("bank".to_string(), vec![bank_a, bank_b]),
        ]),
    )
    .expect("aggregate execution");
    assert_eq!(aggregate.summary.aggregate_matches_count, 0);
}

#[test]
fn bank_statement_without_document_or_tax_id_matches_only_with_deterministic_evidence() {
    let ledger = source("primary", DataSourceKind::Ledger112, SourceRole::Primary);
    let bank = source(
        "bank",
        DataSourceKind::BankStatement,
        SourceRole::RequiredSecondary,
    );
    let payment_rule = rule(
        "bank",
        ComparisonSemantic::BankPayment,
        DataSourceKind::Ledger112,
        DataSourceKind::BankStatement,
        "directionalAmount",
        "directionalAmount",
    );
    let mut ledger_record = record("ledger", "primary", None);
    ledger_record.partner_tax_id = None;
    ledger_record.debit_amount = Some(dec!(100));
    ledger_record.voucher_no = Some("PAY-01".to_string());
    let mut bank_record = record("bank", "bank", None);
    bank_record.partner_tax_id = None;
    bank_record.credit_amount = Some(dec!(100)); // bank credit = money in
    bank_record.voucher_no = Some("PAY-01".to_string());

    let result = execute_reconciliation(
        &session(vec![ledger, bank], vec![payment_rule], false),
        &HashMap::from([
            ("primary".to_string(), vec![ledger_record]),
            ("bank".to_string(), vec![bank_record]),
        ]),
    )
    .expect("execution");
    assert_eq!(result.summary.exact_matches_count, 1);
}

#[test]
fn opposite_semantic_variances_are_reported_as_gross_discrepancy() {
    let invoice = source("primary", DataSourceKind::EInvoice, SourceRole::Primary);
    let register = source(
        "register",
        DataSourceKind::SalesRegister,
        SourceRole::RequiredSecondary,
    );
    let mut register_record = record("register", "register", Some("netting"));
    register_record.pretax_amount = Some(dec!(90));
    register_record.vat_amount = Some(dec!(20));
    let result = execute_reconciliation(
        &session(
            vec![invoice, register],
            vec![
                rule(
                    "revenue",
                    ComparisonSemantic::Revenue,
                    DataSourceKind::EInvoice,
                    DataSourceKind::SalesRegister,
                    "pretaxAmount",
                    "pretaxAmount",
                ),
                rule(
                    "vat",
                    ComparisonSemantic::Vat,
                    DataSourceKind::EInvoice,
                    DataSourceKind::SalesRegister,
                    "vatAmount",
                    "vatAmount",
                ),
            ],
            false,
        ),
        &HashMap::from([
            (
                "primary".to_string(),
                vec![record("invoice", "primary", Some("netting"))],
            ),
            ("register".to_string(), vec![register_record]),
        ]),
    )
    .expect("execution");
    assert_eq!(result.summary.revenue_variance, dec!(10));
    assert_eq!(result.summary.vat_variance, dec!(-10));
    assert!(
        result.summary.total_discrepant_amount > Decimal::ZERO,
        "opposite semantic variances must never net to a zero discrepancy"
    );
    assert_eq!(
        result.summary.net_financial_variance,
        result.summary.total_discrepant_amount
    );
}

#[test]
fn aggregate_over_budget_fails_closed_instead_of_truncating_candidates() {
    let invoice = source("primary", DataSourceKind::EInvoice, SourceRole::Primary);
    let ledger = source(
        "ledger",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
    );
    let mut primary = record("invoice", "primary", Some("aggregate"));
    primary.pretax_amount = Some(dec!(100));
    let mut candidates = Vec::new();
    for index in 0..13 {
        let mut candidate = record(&format!("ledger_{index}"), "ledger", Some("aggregate"));
        candidate.credit_amount = Some(if index == 0 || index == 12 {
            dec!(100)
        } else {
            dec!(1000)
        });
        candidates.push(candidate);
    }
    let result = execute_reconciliation(
        &session(
            vec![invoice, ledger],
            vec![rule(
                "revenue",
                ComparisonSemantic::Revenue,
                DataSourceKind::EInvoice,
                DataSourceKind::Ledger511,
                "pretaxAmount",
                "creditAmount",
            )],
            true,
        ),
        &HashMap::from([
            ("primary".to_string(), vec![primary]),
            ("ledger".to_string(), candidates),
        ]),
    )
    .expect("execution");
    assert_eq!(result.groups[0].status, MatchStatus::NeedsReview);
    assert!(result.groups[0]
        .discrepancies
        .iter()
        .any(|discrepancy| discrepancy.message.contains("COMPLEXITY_LIMIT")));
}

#[test]
fn sales_register_requires_revenue_vat_and_receivable_to_pass() {
    let invoice = source("primary", DataSourceKind::EInvoice, SourceRole::Primary);
    let register = source(
        "register",
        DataSourceKind::SalesRegister,
        SourceRole::RequiredSecondary,
    );
    let rules = vec![
        rule(
            "revenue",
            ComparisonSemantic::Revenue,
            DataSourceKind::EInvoice,
            DataSourceKind::SalesRegister,
            "pretaxAmount",
            "pretaxAmount",
        ),
        rule(
            "vat",
            ComparisonSemantic::Vat,
            DataSourceKind::EInvoice,
            DataSourceKind::SalesRegister,
            "vatAmount",
            "vatAmount",
        ),
        rule(
            "receivable",
            ComparisonSemantic::Receivable,
            DataSourceKind::EInvoice,
            DataSourceKind::SalesRegister,
            "totalAmount",
            "totalAmount",
        ),
    ];
    let invoice_record = record("invoice", "primary", Some("200"));
    let mut register_record = record("register", "register", Some("200"));
    register_record.vat_amount = Some(dec!(11));
    register_record.total_amount = dec!(111);
    let result = execute_reconciliation(
        &session(vec![invoice, register], rules, false),
        &HashMap::from([
            ("primary".to_string(), vec![invoice_record]),
            ("register".to_string(), vec![register_record]),
        ]),
    )
    .expect("execution");
    let primary_group = result
        .groups
        .iter()
        .find(|group| group.primary_source_record_ids == ["invoice".to_string()])
        .expect("primary group");
    assert_eq!(primary_group.status, MatchStatus::MismatchAmount);
    assert_eq!(primary_group.semantic_comparisons.len(), 3);
    assert_eq!(
        primary_group.target_source_record_ids,
        vec!["register".to_string()]
    );
    assert!(primary_group
        .semantic_comparisons
        .iter()
        .any(
            |comparison| comparison.semantic == ComparisonSemantic::Revenue
                && comparison.status == MatchStatus::MatchedExact
        ));
    assert!(primary_group
        .semantic_comparisons
        .iter()
        .any(|comparison| comparison.semantic == ComparisonSemantic::Vat
            && comparison.status == MatchStatus::MismatchAmount));
}

#[test]
fn mapped_adjusted_invoice_is_typed_and_fail_closed() {
    let mut invoice = source("primary", DataSourceKind::EInvoice, SourceRole::Primary);
    invoice.column_mapping.invoice_status_column = Some("Trạng thái".to_string());
    let ledger = source(
        "ledger",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
    );
    let mut invoice_record = record("invoice", "primary", Some("300"));
    invoice_record
        .raw_fields
        .insert("Trạng thái".to_string(), "Hóa đơn điều chỉnh".to_string());
    assert_eq!(
        invoice_lifecycle_for_record(&invoice_record, &invoice.column_mapping),
        Some(InvoiceLifecycle::Adjusted)
    );
    let mut ledger_record = record("ledger", "ledger", Some("300"));
    ledger_record.credit_amount = Some(dec!(100));
    let result = execute_reconciliation(
        &session(
            vec![invoice, ledger],
            vec![rule(
                "revenue",
                ComparisonSemantic::Revenue,
                DataSourceKind::EInvoice,
                DataSourceKind::Ledger511,
                "pretaxAmount",
                "creditAmount",
            )],
            false,
        ),
        &HashMap::from([
            ("primary".to_string(), vec![invoice_record]),
            ("ledger".to_string(), vec![ledger_record]),
        ]),
    )
    .expect("execution");
    assert_eq!(result.groups[0].status, MatchStatus::NeedsReview);
}

#[test]
fn adjusted_replaced_cancelled_and_unknown_lifecycle_values_are_not_regular() {
    for status in ["điều chỉnh", "thay thế", "hủy", "unrecognized status"] {
        assert!(!InvoiceLifecycle::from_status(Some(status)).is_regular());
    }
    assert_eq!(
        InvoiceLifecycle::from_status(None),
        InvoiceLifecycle::Unknown
    );
}

#[test]
fn partner_master_runs_as_one_source_typed_control() {
    let partner_source = source(
        "primary",
        DataSourceKind::PartnerMaster,
        SourceRole::ReferenceMaster,
    );
    let partner = PartnerRecord {
        id: "partner_1".to_string(),
        source_id: "primary".to_string(),
        source_row: 2,
        partner_code: Some("P1".to_string()),
        partner_name: Some("Synthetic partner".to_string()),
        partner_tax_id: Some("0100000000".to_string()),
        address: None,
        is_customer: Some(true),
        is_supplier: Some(false),
        status: Some("active".to_string()),
        raw_fields: HashMap::new(),
    };
    let control = evaluate_partner_master_control("primary".to_string(), &[partner]);
    let result =
        execute_reference_controls(&session(vec![partner_source], vec![], false), vec![control])
            .expect("one-source reference control");
    assert!(result.groups.is_empty());
    assert_eq!(result.reference_controls.len(), 1);
    assert_eq!(
        result.reference_controls[0].status,
        MatchStatus::MatchedExact
    );
}

#[test]
fn partner_identity_uses_mst_then_code_without_mutating_transactions() {
    let master = PartnerRecord {
        id: "partner_0502".to_string(),
        source_id: "master".to_string(),
        source_row: 2,
        partner_code: Some("0502".to_string()),
        partner_name: Some("Synthetic".to_string()),
        partner_tax_id: Some("0100000000".to_string()),
        address: None,
        is_customer: Some(true),
        is_supplier: Some(false),
        status: None,
        raw_fields: HashMap::new(),
    };
    let mut invoice = record("invoice", "invoice", Some("partner"));
    invoice.partner_code = Some("0502".to_string());
    let before = invoice.clone();
    let control = evaluate_partner_identity_cross_source(
        "master".to_string(),
        &[master],
        std::iter::once(&invoice),
    );
    assert_eq!(control.status, MatchStatus::MatchedExact);
    assert_eq!(invoice, before);
}

#[test]
fn sales_analysis_runs_as_one_source_typed_control_without_transaction_groups() {
    let analysis_source = source(
        "primary",
        DataSourceKind::SalesAnalysisReport,
        SourceRole::Primary,
    );
    let records: Vec<SalesAnalysisRecord> = vec![];
    let control = evaluate_sales_analysis_control("primary".to_string(), &records);
    let result = execute_reference_controls(
        &session(vec![analysis_source], vec![], false),
        vec![control],
    )
    .expect("one-source sales-analysis control");
    assert!(result.groups.is_empty());
    assert_eq!(
        result.reference_controls[0].status,
        MatchStatus::NeedsReview
    );
}

#[test]
fn tri_source_invoice_sales_register_ledger_missing_is_high_needs_review() {
    let invoice = source("primary", DataSourceKind::EInvoice, SourceRole::Primary);
    let register = source(
        "register",
        DataSourceKind::SalesRegister,
        SourceRole::RequiredSecondary,
    );
    let ledger = source(
        "ledger",
        DataSourceKind::Ledger511,
        SourceRole::RequiredSecondary,
    );
    let rules = vec![
        rule(
            "register_revenue",
            ComparisonSemantic::Revenue,
            DataSourceKind::EInvoice,
            DataSourceKind::SalesRegister,
            "pretaxAmount",
            "pretaxAmount",
        ),
        rule(
            "register_vat",
            ComparisonSemantic::Vat,
            DataSourceKind::EInvoice,
            DataSourceKind::SalesRegister,
            "vatAmount",
            "vatAmount",
        ),
        rule(
            "register_receivable",
            ComparisonSemantic::Receivable,
            DataSourceKind::EInvoice,
            DataSourceKind::SalesRegister,
            "totalAmount",
            "totalAmount",
        ),
        rule(
            "ledger_revenue",
            ComparisonSemantic::Revenue,
            DataSourceKind::EInvoice,
            DataSourceKind::Ledger511,
            "pretaxAmount",
            "creditAmount",
        ),
    ];
    let invoice_record = record("invoice_233", "primary", Some("233"));
    let register_record = record("register_233", "register", Some("233"));
    let result = execute_reconciliation(
        &session(vec![invoice, register, ledger], rules, false),
        &HashMap::from([
            ("primary".to_string(), vec![invoice_record]),
            ("register".to_string(), vec![register_record]),
            ("ledger".to_string(), vec![]),
        ]),
    )
    .expect("execution");
    let primary_group = result
        .groups
        .iter()
        .find(|group| group.primary_source_record_ids == ["invoice_233".to_string()])
        .expect("primary group");
    assert_eq!(primary_group.status, MatchStatus::NeedsReview);
    assert_eq!(primary_group.semantic_comparisons.len(), 4);
    assert!(primary_group
        .discrepancies
        .iter()
        .any(|discrepancy| discrepancy.message.contains("HIGH")));
}

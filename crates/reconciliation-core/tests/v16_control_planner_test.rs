use std::collections::HashMap;

use reconciliation_core::{
    execute_audit_session, plan_controls, prepare_audit_session, AccountingPeriod,
    AnalyticalRowLevel, CanonicalRecord, ControlExecutionStatus, ControlPlanStatus, DataSource,
    DataSourceKind, PartnerRecord, SalesAnalysisRecord, SourceCatalog, SourceRole, BANK_CONTROL_ID,
    PARTNER_CONTROL_ID, RECEIVABLE_CONTROL_ID, REVENUE_CONTROL_ID, SALES_ANALYSIS_CONTROL_ID,
    VAT_CONTROL_ID,
};
use rust_decimal_macros::dec;

fn source(id: &str, kind: DataSourceKind) -> DataSource {
    DataSource {
        id: id.to_string(),
        name: id.to_string(),
        file_path: "synthetic.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: Default::default(),
    }
}

fn record(id: &str, source_id: &str, date: Option<&str>) -> CanonicalRecord {
    CanonicalRecord {
        id: id.to_string(),
        source_id: source_id.to_string(),
        source_row: 2,
        date: date.map(str::to_string),
        doc_no: Some("101".to_string()),
        partner_tax_id: Some("0100000001".to_string()),
        pretax_amount: Some(dec!(100)),
        vat_amount: Some(dec!(10)),
        total_amount: dec!(110),
        ..Default::default()
    }
}

fn period() -> AccountingPeriod {
    AccountingPeriod {
        start_date: "2026-07-01".to_string(),
        end_date: "2026-07-31".to_string(),
    }
}

fn plan<'a>(
    plans: &'a [reconciliation_core::ControlPlan],
    id: &str,
) -> &'a reconciliation_core::ControlPlan {
    plans
        .iter()
        .find(|plan| plan.control_id == id)
        .expect("control plan")
}

#[test]
fn planner_discovers_four_ready_controls_and_future_missing_sources() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger511", DataSourceKind::Ledger511),
        source("ledger112", DataSourceKind::Ledger112),
        source("bank", DataSourceKind::BankStatement),
        source("partners", DataSourceKind::PartnerMaster),
        source("analysis", DataSourceKind::SalesAnalysisReport),
    ];
    let mut transactional = HashMap::new();
    transactional.insert(
        "invoice".to_string(),
        vec![record("i1", "invoice", Some("2026-07-10"))],
    );
    transactional.insert(
        "register".to_string(),
        vec![record("r1", "register", Some("2026-07-10"))],
    );
    transactional.insert(
        "ledger511".to_string(),
        vec![CanonicalRecord {
            credit_amount: Some(dec!(100)),
            ..record("l511", "ledger511", Some("2026-07-10"))
        }],
    );
    transactional.insert(
        "ledger112".to_string(),
        vec![CanonicalRecord {
            debit_amount: Some(dec!(110)),
            transaction_number: Some("TX-1".to_string()),
            ..record("l112", "ledger112", Some("2026-07-11"))
        }],
    );
    transactional.insert(
        "bank".to_string(),
        vec![CanonicalRecord {
            credit_amount: Some(dec!(110)),
            transaction_number: Some("TX-1".to_string()),
            ..record("b1", "bank", Some("2026-07-11"))
        }],
    );
    let partner = PartnerRecord {
        id: "p1".to_string(),
        source_id: "partners".to_string(),
        source_row: 2,
        partner_code: Some("C001".to_string()),
        partner_name: Some("Synthetic Partner".to_string()),
        partner_tax_id: Some("0100000001".to_string()),
        address: None,
        is_customer: Some(true),
        is_supplier: Some(false),
        status: Some("ACTIVE".to_string()),
        raw_fields: HashMap::new(),
    };
    let analysis_row = |id: &str, level| SalesAnalysisRecord {
        id: id.to_string(),
        source_id: "analysis".to_string(),
        source_row: 2,
        row_level: level,
        group_key: Some("G1".to_string()),
        product_code: Some("P1".to_string()),
        product_name: Some("Synthetic".to_string()),
        quantity: Some(dec!(1)),
        unit_price: Some(dec!(100)),
        revenue: Some(dec!(100)),
        vat: Some(dec!(10)),
        discount: Some(dec!(0)),
        receivable: Some(dec!(110)),
        unit_cost: Some(dec!(60)),
        cost: Some(dec!(60)),
        profit: Some(dec!(40)),
        raw_fields: HashMap::new(),
    };
    let audit = prepare_audit_session(
        "v16-synthetic".to_string(),
        period(),
        sources,
        transactional,
        HashMap::from([("partners".to_string(), vec![partner])]),
        HashMap::from([(
            "analysis".to_string(),
            vec![
                analysis_row("group", AnalyticalRowLevel::Group),
                analysis_row("detail", AnalyticalRowLevel::Detail),
            ],
        )]),
        HashMap::new(),
    )
    .expect("prepare audit session");

    for id in [
        REVENUE_CONTROL_ID,
        BANK_CONTROL_ID,
        PARTNER_CONTROL_ID,
        SALES_ANALYSIS_CONTROL_ID,
    ] {
        assert_eq!(
            plan(&audit.control_plans, id).status,
            ControlPlanStatus::Ready
        );
    }
    assert_eq!(
        plan(&audit.control_plans, VAT_CONTROL_ID).status,
        ControlPlanStatus::MissingSource
    );
    assert_eq!(
        plan(&audit.control_plans, RECEIVABLE_CONTROL_ID).status,
        ControlPlanStatus::MissingSource
    );
    assert!(audit.source_reuse.iter().all(|source| {
        source.read_count == 1 && source.normalize_count == 1 && source.index_count == 1
    }));

    let report = execute_audit_session(audit, dec!(0), 3).expect("execute all ready controls");
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == REVENUE_CONTROL_ID)
            .expect("revenue result")
            .status,
        ControlExecutionStatus::Pass
    );
}

#[test]
fn period_scope_excludes_cross_period_rows_before_matching() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger", DataSourceKind::Ledger511),
    ];
    let records = HashMap::from([
        (
            "invoice".to_string(),
            vec![record("i", "invoice", Some("2026-07-10"))],
        ),
        (
            "register".to_string(),
            vec![record("r", "register", Some("2026-01-10"))],
        ),
        (
            "ledger".to_string(),
            vec![record("l", "ledger", Some("2026-07-10"))],
        ),
    ]);
    let audit = prepare_audit_session(
        "period".to_string(),
        period(),
        sources,
        records,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    assert_eq!(
        audit.normalized_datasets.transactional["register"]
            .records
            .len(),
        0
    );
    assert_eq!(
        plan(&audit.control_plans, REVENUE_CONTROL_ID).status,
        ControlPlanStatus::NeedsReview
    );
}

#[test]
fn missing_required_transaction_date_is_fail_closed_for_review() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger", DataSourceKind::Ledger511),
    ];
    let records = HashMap::from([
        ("invoice".to_string(), vec![record("i", "invoice", None)]),
        (
            "register".to_string(),
            vec![record("r", "register", Some("2026-07-10"))],
        ),
        (
            "ledger".to_string(),
            vec![record("l", "ledger", Some("2026-07-10"))],
        ),
    ]);
    let audit = prepare_audit_session(
        "missing-date".to_string(),
        period(),
        sources,
        records,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    assert_eq!(
        plan(&audit.control_plans, REVENUE_CONTROL_ID).status,
        ControlPlanStatus::NeedsReview
    );
}

#[test]
fn planner_supports_empty_catalog_without_guessing_accounts() {
    let plans = plan_controls(&SourceCatalog::default());
    assert_eq!(
        plan(&plans, RECEIVABLE_CONTROL_ID).status,
        ControlPlanStatus::MissingSource
    );
    assert_eq!(
        plan(&plans, VAT_CONTROL_ID).status,
        ControlPlanStatus::MissingSource
    );
}

#[test]
fn generic_ledger_content_adapts_to_legacy_matching_without_filename_rules() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("journal", DataSourceKind::Custom),
    ];
    let records = HashMap::from([
        (
            "invoice".to_string(),
            vec![record("i", "invoice", Some("2026-07-10"))],
        ),
        (
            "register".to_string(),
            vec![record("r", "register", Some("2026-07-10"))],
        ),
        (
            "journal".to_string(),
            vec![CanonicalRecord {
                credit_account: Some("5111".to_string()),
                debit_account: Some("1311".to_string()),
                credit_amount: Some(dec!(100)),
                ..record("j", "journal", Some("2026-07-10"))
            }],
        ),
    ]);
    let audit = prepare_audit_session(
        "generic-ledger".to_string(),
        period(),
        sources,
        records,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    assert_eq!(
        plan(&audit.control_plans, REVENUE_CONTROL_ID).status,
        ControlPlanStatus::Ready
    );
    let report = execute_audit_session(audit, dec!(0), 3).expect("execute");
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == REVENUE_CONTROL_ID)
            .expect("revenue")
            .status,
        ControlExecutionStatus::Pass
    );
}

#[test]
fn duplicate_required_capability_needs_review_instead_of_picking_first() {
    let sources = vec![
        source("invoice-a", DataSourceKind::EInvoice),
        source("invoice-b", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger", DataSourceKind::Ledger511),
    ];
    let records = HashMap::from([
        (
            "invoice-a".to_string(),
            vec![record("ia", "invoice-a", Some("2026-07-10"))],
        ),
        (
            "invoice-b".to_string(),
            vec![record("ib", "invoice-b", Some("2026-07-10"))],
        ),
        (
            "register".to_string(),
            vec![record("r", "register", Some("2026-07-10"))],
        ),
        (
            "ledger".to_string(),
            vec![record("l", "ledger", Some("2026-07-10"))],
        ),
    ]);
    let audit = prepare_audit_session(
        "duplicate-capability".to_string(),
        period(),
        sources,
        records,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    assert_eq!(
        plan(&audit.control_plans, REVENUE_CONTROL_ID).status,
        ControlPlanStatus::NeedsReview
    );
}

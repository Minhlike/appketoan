use std::collections::HashMap;
use std::time::Instant;

use reconciliation_core::{
    execute_audit_session, execute_reconciliation, prepare_audit_session, AccountingPeriod,
    AnalyticalRowLevel, CanonicalRecord, ComparisonRule, ComparisonSemantic, ControlPlanStatus,
    DataSource, DataSourceKind, PartnerRecord, ReconciliationSession, SalesAnalysisRecord,
    SourceRole, REVENUE_CONTROL_ID,
};
use rust_decimal::Decimal;

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
        column_mapping: Default::default(),
    }
}

fn records(source_id: &str, ledger: bool) -> Vec<CanonicalRecord> {
    (0..100_000)
        .map(|index| CanonicalRecord {
            id: format!("{source_id}-{index}"),
            source_id: source_id.to_string(),
            source_row: index + 2,
            date: Some("2026-07-15".to_string()),
            doc_no: Some(format!("V16-{index:06}")),
            partner_tax_id: Some("0100000001".to_string()),
            pretax_amount: Some(Decimal::from(1_000_000u64 + index as u64)),
            vat_amount: Some(Decimal::from(100_000u64 + index as u64)),
            total_amount: Decimal::from(1_100_000u64 + 2 * index as u64),
            credit_amount: ledger.then(|| Decimal::from(1_000_000u64 + index as u64)),
            ..Default::default()
        })
        .collect()
}

fn rule(
    id: &str,
    semantic: ComparisonSemantic,
    secondary_kind: DataSourceKind,
    primary_field: &str,
    secondary_field: &str,
) -> ComparisonRule {
    ComparisonRule {
        id: id.to_string(),
        name: id.to_string(),
        semantic,
        primary_source_kind: DataSourceKind::EInvoice,
        primary_field: primary_field.to_string(),
        secondary_source_kind: secondary_kind,
        secondary_field: secondary_field.to_string(),
        is_required: true,
        tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
    }
}

#[test]
fn benchmark_v15_tri_source_vs_v16_reused_workspace() {
    let invoice = source("invoice", DataSourceKind::EInvoice, SourceRole::Primary);
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
    let source_records = HashMap::from([
        (invoice.id.clone(), records(&invoice.id, false)),
        (register.id.clone(), records(&register.id, false)),
        (ledger.id.clone(), records(&ledger.id, true)),
    ]);
    let rules = vec![
        rule(
            "register-revenue",
            ComparisonSemantic::Revenue,
            DataSourceKind::SalesRegister,
            "pretaxAmount",
            "pretaxAmount",
        ),
        rule(
            "register-vat",
            ComparisonSemantic::Vat,
            DataSourceKind::SalesRegister,
            "vatAmount",
            "vatAmount",
        ),
        rule(
            "register-receivable",
            ComparisonSemantic::Receivable,
            DataSourceKind::SalesRegister,
            "totalAmount",
            "totalAmount",
        ),
        rule(
            "ledger-revenue",
            ComparisonSemantic::Revenue,
            DataSourceKind::Ledger511,
            "pretaxAmount",
            "creditAmount",
        ),
    ];
    let v15_session = ReconciliationSession {
        session_id: "v15-tri-benchmark".to_string(),
        scenario_name: "v15-tri-benchmark".to_string(),
        primary_source_id: Some(invoice.id.clone()),
        expected_primary_kind: Some(DataSourceKind::EInvoice),
        required_source_ids: Some(vec![
            invoice.id.clone(),
            register.id.clone(),
            ledger.id.clone(),
        ]),
        optional_source_ids: None,
        data_sources: vec![invoice.clone(), register.clone(), ledger.clone()],
        comparison_rules: rules,
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
        enable_aggregate_match: false,
    };
    let v15_started = Instant::now();
    let v15 = execute_reconciliation(&v15_session, &source_records).expect("V15 tri baseline");
    let v15_elapsed = v15_started.elapsed();
    assert_eq!(v15.summary.exact_matches_count, 100_000);

    let partner_source = source(
        "partners",
        DataSourceKind::PartnerMaster,
        SourceRole::ReferenceMaster,
    );
    let analysis_source = source(
        "analysis",
        DataSourceKind::SalesAnalysisReport,
        SourceRole::ReferenceMaster,
    );
    let partner = PartnerRecord {
        id: "partner-1".to_string(),
        source_id: partner_source.id.clone(),
        source_row: 2,
        partner_code: Some("P001".to_string()),
        partner_name: Some("Synthetic".to_string()),
        partner_tax_id: Some("0100000001".to_string()),
        address: None,
        is_customer: Some(true),
        is_supplier: None,
        status: Some("ACTIVE".to_string()),
        raw_fields: HashMap::new(),
    };
    let analysis = |id: &str, row_level| SalesAnalysisRecord {
        id: id.to_string(),
        source_id: analysis_source.id.clone(),
        source_row: 2,
        row_level,
        group_key: Some("G1".to_string()),
        product_code: Some("P1".to_string()),
        product_name: Some("Synthetic".to_string()),
        quantity: Some(Decimal::ONE),
        unit_price: Some(Decimal::ONE),
        revenue: Some(Decimal::ONE),
        vat: Some(Decimal::ONE),
        discount: Some(Decimal::ZERO),
        receivable: Some(Decimal::ONE),
        unit_cost: Some(Decimal::ONE),
        cost: Some(Decimal::ONE),
        profit: Some(Decimal::ONE),
        raw_fields: HashMap::new(),
    };
    let analysis_records = vec![
        analysis("analysis-group", AnalyticalRowLevel::Group),
        analysis("analysis-detail", AnalyticalRowLevel::Detail),
    ];
    let v16_started = Instant::now();
    let audit = prepare_audit_session(
        "v16-reuse-benchmark".to_string(),
        AccountingPeriod {
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        vec![invoice, register, ledger, partner_source, analysis_source],
        source_records,
        HashMap::from([("partners".to_string(), vec![partner])]),
        HashMap::from([("analysis".to_string(), analysis_records)]),
        HashMap::new(),
    )
    .expect("prepare V16 workspace");
    assert!(audit.source_reuse.iter().all(|source| {
        source.read_count == 1 && source.normalize_count == 1 && source.index_count == 1
    }));
    assert_eq!(
        audit
            .control_plans
            .iter()
            .find(|plan| plan.control_id == REVENUE_CONTROL_ID)
            .expect("revenue plan")
            .status,
        ControlPlanStatus::Ready
    );
    let v16 = execute_audit_session(audit, Decimal::ZERO, 0).expect("V16 reused workspace");
    let v16_elapsed = v16_started.elapsed();
    let v16_revenue = v16
        .control_results
        .iter()
        .find(|result| result.control_id == REVENUE_CONTROL_ID)
        .and_then(|result| result.reconciliation_result.as_ref())
        .expect("V16 revenue result");
    assert_eq!(v16_revenue.summary.exact_matches_count, 100_000);
    assert_eq!(
        v16.control_plans
            .iter()
            .filter(|plan| plan.status == ControlPlanStatus::Ready)
            .count(),
        3
    );
    println!(
        "V16_BENCHMARK records_per_transaction_source=100000 v15_tri_ms={} v16_reused_three_controls_ms={} exact=100000 reuse=1/1/1",
        v15_elapsed.as_millis(),
        v16_elapsed.as_millis()
    );
}

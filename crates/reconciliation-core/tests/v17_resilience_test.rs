use std::collections::HashMap;

use reconciliation_core::{
    execute_audit_session_with_cancellation, prepare_audit_session, AccountingPeriod,
    AnalyticalRowLevel, AuditCancellationToken, AuditRunStatus, ControlExecutionStatus,
    ControlPlan, ControlPlanStatus, DataSource, DataSourceKind, SalesAnalysisRecord, SourceRole,
    SALES_ANALYSIS_CONTROL_ID,
};
use rust_decimal::Decimal;

fn analysis_source() -> DataSource {
    DataSource {
        id: "analysis".to_string(),
        name: "Synthetic sales analysis".to_string(),
        file_path: "synthetic.xlsx".to_string(),
        sheet_name: "Data".to_string(),
        kind: DataSourceKind::SalesAnalysisReport,
        role: SourceRole::Primary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: Default::default(),
    }
}

fn analysis_row(id: &str, row_level: AnalyticalRowLevel) -> SalesAnalysisRecord {
    SalesAnalysisRecord {
        id: id.to_string(),
        source_id: "analysis".to_string(),
        source_row: 2,
        row_level,
        group_key: Some("G1".to_string()),
        product_code: Some("P1".to_string()),
        product_name: Some("Synthetic".to_string()),
        quantity: Some(Decimal::ONE),
        unit_price: Some(Decimal::new(100, 0)),
        revenue: Some(Decimal::new(100, 0)),
        vat: Some(Decimal::TEN),
        discount: Some(Decimal::ZERO),
        receivable: Some(Decimal::new(110, 0)),
        unit_cost: Some(Decimal::new(60, 0)),
        cost: Some(Decimal::new(60, 0)),
        profit: Some(Decimal::new(40, 0)),
        raw_fields: HashMap::new(),
    }
}

fn session() -> reconciliation_core::AuditSession {
    prepare_audit_session(
        "v17-resilience".to_string(),
        AccountingPeriod {
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        vec![analysis_source()],
        HashMap::new(),
        HashMap::new(),
        HashMap::from([(
            "analysis".to_string(),
            vec![
                analysis_row("group", AnalyticalRowLevel::Group),
                analysis_row("detail", AnalyticalRowLevel::Detail),
            ],
        )]),
        HashMap::new(),
    )
    .expect("prepare")
}

#[test]
fn one_control_failure_does_not_erase_independent_results() {
    let mut audit = session();
    audit.control_plans.insert(
        0,
        ControlPlan {
            control_id: "SYNTHETIC_FAILING_CONTROL".to_string(),
            title: "Synthetic failing control".to_string(),
            status: ControlPlanStatus::Ready,
            source_ids: vec!["analysis".to_string()],
            missing_capabilities: vec![],
            warnings: vec![],
            effective_period: None,
        },
    );

    let report = execute_audit_session_with_cancellation(
        audit,
        Decimal::ZERO,
        0,
        &AuditCancellationToken::default(),
    )
    .expect("session returns partial report");

    assert_eq!(report.run_status, AuditRunStatus::Partial);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(
        report.control_results[0].status,
        ControlExecutionStatus::Failed
    );
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == SALES_ANALYSIS_CONTROL_ID)
            .expect("independent control result")
            .status,
        ControlExecutionStatus::Pass
    );
}

#[test]
fn cancelled_session_never_reports_ready_control_as_pass() {
    let audit = session();
    let token = AuditCancellationToken::default();
    token.cancel();
    let report = execute_audit_session_with_cancellation(audit, Decimal::ZERO, 0, &token)
        .expect("cancelled report");

    assert_eq!(report.run_status, AuditRunStatus::Cancelled);
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == SALES_ANALYSIS_CONTROL_ID)
            .expect("cancelled control")
            .status,
        ControlExecutionStatus::Cancelled
    );
    assert!(report.control_results.iter().all(|result| {
        result.status != ControlExecutionStatus::Pass
            && result.status != ControlExecutionStatus::NeedsReview
    }));
}

#[test]
fn structured_metrics_keep_control_timings_separate() {
    let report = execute_audit_session_with_cancellation(
        session(),
        Decimal::ZERO,
        0,
        &AuditCancellationToken::default(),
    )
    .expect("execute");
    assert_eq!(report.metrics.source_count, 1);
    assert_eq!(report.metrics.normalized_record_count, 2);
    assert!(report
        .metrics
        .control_execution
        .iter()
        .any(|metric| metric.control_id == SALES_ANALYSIS_CONTROL_ID));
    assert!(matches!(
        report.source_catalog.sources.as_slice(),
        [reconciliation_core::SourceCatalogEntry { .. }]
    ));
}

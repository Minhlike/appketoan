use std::collections::HashMap;

use reconciliation_core::{
    evaluate_document_integrity, execute_audit_session_with_cancellation,
    matcher::candidate_strategy::{select_bank_candidate, BankCandidateDecision},
    normalize_data_source_rows, prepare_audit_session, AccountingPeriod, AuditCancellationToken,
    CanonicalRecord, ColumnMapping, ComparisonRule, ComparisonSemantic, ControlExecutionStatus,
    ControlPlan, ControlPlanStatus, DataSource, DataSourceKind, DocumentComparisonScope,
    DocumentErrorCode, DocumentField, DocumentIntegritySource, FieldCheckStatus, SourceRole,
    ValueOrigin, REVENUE_CONTROL_ID,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn source(id: &str, kind: DataSourceKind) -> DataSource {
    DataSource {
        id: id.to_string(),
        name: format!("Synthetic {id}"),
        file_path: format!("synthetic-{id}.xlsx"),
        sheet_name: "Data".to_string(),
        kind,
        role: SourceRole::RequiredSecondary,
        header_row: 1,
        data_start_row: 2,
        column_mapping: Default::default(),
    }
}

fn transaction(
    id: &str,
    source_id: &str,
    doc_no: Option<&str>,
    date: Option<&str>,
    pretax: Option<Decimal>,
    vat: Option<Decimal>,
    total: Decimal,
) -> CanonicalRecord {
    CanonicalRecord {
        id: id.to_string(),
        source_id: source_id.to_string(),
        source_row: 2,
        doc_no: doc_no.map(str::to_string),
        date: date.map(str::to_string),
        pretax_amount: pretax,
        vat_amount: vat,
        total_amount: total,
        total_amount_origin: ValueOrigin::Source,
        partner_tax_id: Some("0100000000".to_string()),
        ..Default::default()
    }
}

fn ledger(id: &str, doc_no: &str, date: &str, credit: Decimal) -> CanonicalRecord {
    CanonicalRecord {
        credit_amount: Some(credit),
        ..transaction(
            id,
            "ledger",
            Some(doc_no),
            Some(date),
            Some(credit),
            None,
            credit,
        )
    }
}

fn evaluate(
    invoices: &[CanonicalRecord],
    registers: &[CanonicalRecord],
    ledgers: &[CanonicalRecord],
) -> reconciliation_core::DocumentIntegrityResult {
    let invoice_source = source("invoice", DataSourceKind::EInvoice);
    let register_source = source("register", DataSourceKind::SalesRegister);
    let ledger_source = source("ledger", DataSourceKind::Ledger511);
    evaluate_document_integrity(
        DocumentIntegritySource {
            source: &invoice_source,
            records: invoices,
        },
        DocumentIntegritySource {
            source: &register_source,
            records: registers,
        },
        DocumentIntegritySource {
            source: &ledger_source,
            records: ledgers,
        },
    )
}

fn codes(result: &reconciliation_core::DocumentIntegrityResult) -> Vec<DocumentErrorCode> {
    result
        .documents
        .iter()
        .flat_map(|document| document.errors.iter().map(|error| error.code))
        .collect()
}

#[test]
fn duplicate_bk_invoice_numbers_mark_every_row_and_never_accept_one() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let register_a = transaction(
        "register-a",
        "register",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let mut register_b = register_a.clone();
    register_b.id = "register-b".to_string();
    register_b.source_row = 3;
    let result = evaluate(
        &[invoice],
        &[register_a, register_b],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );

    assert!(!result.documents_pass);
    assert_eq!(result.summary.duplicate_invoice_number, 2);
    assert_eq!(result.summary.ambiguous_match, 1);
    for id in ["register-a", "register-b"] {
        assert!(result.documents.iter().any(|document| {
            document
                .sales_register
                .as_ref()
                .is_some_and(|record| record.provenance.record_id == id)
                && document
                    .errors
                    .iter()
                    .any(|error| error.code == DocumentErrorCode::DuplicateInvoiceNumber)
        }));
    }
}

#[test]
fn same_number_and_money_with_wrong_date_is_diagnostic_only() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let register = transaction(
        "register-1",
        "register",
        Some("001"),
        Some("2026-07-03"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let result = evaluate(
        &[invoice],
        &[register],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );
    assert_eq!(result.summary.date_mismatch, 1);
    assert!(!result.documents_pass);
    assert!(codes(&result).contains(&DocumentErrorCode::DateMismatch));
}

#[test]
fn wrong_number_uses_unique_strong_diagnostic_link_but_never_passes() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let register = transaction(
        "register-1",
        "register",
        Some("999"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let result = evaluate(
        &[invoice],
        &[register],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );
    assert_eq!(result.summary.invoice_number_mismatch, 1);
    assert_eq!(result.summary.missing_in_bk, 0);
    assert_eq!(result.summary.extra_in_bk, 0);
    assert!(!result.documents_pass);
}

#[test]
fn wrong_date_and_number_need_unique_partner_and_money_evidence() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let register = transaction(
        "register-1",
        "register",
        Some("999"),
        Some("2026-07-03"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let result = evaluate(
        &[invoice],
        &[register],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );
    assert_eq!(result.summary.date_mismatch, 1);
    assert_eq!(result.summary.invoice_number_mismatch, 1);
    assert_eq!(result.summary.missing_in_bk, 0);
    assert!(!result.documents_pass);
}

#[test]
fn monetary_fields_report_independent_and_combined_errors() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    for (register, expected) in [
        (
            transaction(
                "pretax",
                "register",
                Some("001"),
                Some("2026-07-02"),
                Some(dec!(99)),
                Some(dec!(10)),
                dec!(110),
            ),
            vec![DocumentErrorCode::PretaxMismatch],
        ),
        (
            transaction(
                "vat",
                "register",
                Some("001"),
                Some("2026-07-02"),
                Some(dec!(100)),
                Some(dec!(9)),
                dec!(110),
            ),
            vec![DocumentErrorCode::VatMismatch],
        ),
        (
            transaction(
                "total",
                "register",
                Some("001"),
                Some("2026-07-02"),
                Some(dec!(100)),
                Some(dec!(10)),
                dec!(109),
            ),
            vec![DocumentErrorCode::TotalMismatch],
        ),
        (
            transaction(
                "all",
                "register",
                Some("001"),
                Some("2026-07-02"),
                Some(dec!(99)),
                Some(dec!(9)),
                dec!(108),
            ),
            vec![
                DocumentErrorCode::PretaxMismatch,
                DocumentErrorCode::VatMismatch,
                DocumentErrorCode::TotalMismatch,
            ],
        ),
    ] {
        let result = evaluate(
            std::slice::from_ref(&invoice),
            &[register],
            &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
        );
        let actual = codes(&result);
        for code in expected {
            assert!(actual.contains(&code), "missing {code:?}");
        }
        assert!(!result.documents_pass);
    }
}

#[test]
fn missing_extra_and_ambiguous_candidates_are_conserved_without_guessing() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let candidate_a = transaction(
        "register-a",
        "register",
        Some("900"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let mut candidate_b = candidate_a.clone();
    candidate_b.id = "register-b".to_string();
    candidate_b.doc_no = Some("901".to_string());
    let result = evaluate(
        std::slice::from_ref(&invoice),
        &[candidate_a, candidate_b],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );
    assert_eq!(result.summary.ambiguous_match, 1);
    assert_eq!(result.summary.missing_in_bk, 0);
    assert_eq!(result.summary.extra_in_bk, 0);

    let missing = evaluate(
        std::slice::from_ref(&invoice),
        &[],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );
    assert_eq!(missing.summary.missing_in_bk, 1);

    let extra = evaluate(
        &[],
        &[transaction(
            "register-extra",
            "register",
            Some("777"),
            Some("2026-07-02"),
            Some(dec!(50)),
            Some(dec!(5)),
            dec!(55),
        )],
        &[],
    );
    assert_eq!(extra.summary.extra_in_bk, 1);
}

#[test]
fn equal_totals_do_not_hide_wrong_documents() {
    let invoices = vec![
        transaction(
            "invoice-1",
            "invoice",
            Some("001"),
            Some("2026-07-02"),
            Some(dec!(100)),
            Some(dec!(10)),
            dec!(110),
        ),
        transaction(
            "invoice-2",
            "invoice",
            Some("002"),
            Some("2026-07-02"),
            Some(dec!(200)),
            Some(dec!(20)),
            dec!(220),
        ),
    ];
    let registers = vec![
        transaction(
            "register-1",
            "register",
            Some("001"),
            Some("2026-07-02"),
            Some(dec!(200)),
            Some(dec!(20)),
            dec!(220),
        ),
        transaction(
            "register-2",
            "register",
            Some("002"),
            Some("2026-07-02"),
            Some(dec!(100)),
            Some(dec!(10)),
            dec!(110),
        ),
    ];
    let ledgers = vec![
        ledger("ledger-1", "001", "2026-07-02", dec!(100)),
        ledger("ledger-2", "002", "2026-07-02", dec!(200)),
    ];
    let result = evaluate(&invoices, &registers, &ledgers);
    assert!(result.totals_equal);
    assert!(!result.documents_pass);
    assert_eq!(result.summary.pretax_mismatch, 2);
    assert_eq!(result.summary.vat_mismatch, 2);
    assert_eq!(result.summary.total_mismatch, 2);
}

#[test]
fn oracle_233_is_review_high_and_ledger_vat_receivable_are_not_checked() {
    let mut invoices = Vec::new();
    let mut registers = Vec::new();
    let mut ledgers = Vec::new();
    for number in 1..=46 {
        let document = if number == 46 {
            "233".to_string()
        } else {
            number.to_string()
        };
        let pretax = if document == "233" {
            dec!(105000000)
        } else {
            dec!(1000)
        };
        invoices.push(transaction(
            &format!("invoice-{document}"),
            "invoice",
            Some(&document),
            Some("2026-07-06"),
            Some(pretax),
            Some(pretax / dec!(10)),
            pretax + pretax / dec!(10),
        ));
        registers.push(transaction(
            &format!("register-{document}"),
            "register",
            Some(&document),
            Some("2026-07-06"),
            Some(pretax),
            Some(pretax / dec!(10)),
            pretax + pretax / dec!(10),
        ));
        if document != "233" {
            ledgers.push(ledger(
                &format!("ledger-{document}"),
                &document,
                "2026-07-06",
                pretax,
            ));
        }
    }
    let result = evaluate(&invoices, &registers, &ledgers);
    assert_eq!(result.summary.fully_matched, 45);
    assert_eq!(result.summary.missing_in_tk511, 1);
    let document_233 = result
        .documents
        .iter()
        .find(|document| {
            document
                .invoice
                .as_ref()
                .and_then(|invoice| invoice.invoice_number.as_deref())
                == Some("233")
        })
        .expect("#233");
    assert!(document_233.errors.iter().any(|error| {
        error.code == DocumentErrorCode::MissingInTk511 && error.severity == "HIGH"
    }));
    assert!(document_233.field_checks.iter().any(|check| {
        check.scope == DocumentComparisonScope::InvoiceToLedger511
            && check.field == DocumentField::Vat
            && check.status == FieldCheckStatus::NotChecked
    }));
    assert!(document_233.field_checks.iter().any(|check| {
        check.scope == DocumentComparisonScope::InvoiceToLedger511
            && check.field == DocumentField::Total
            && check.status == FieldCheckStatus::NotChecked
    }));
}

#[test]
fn invalid_bk_fields_return_all_validation_errors() {
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let mut invalid = transaction(
        "register-invalid",
        "register",
        None,
        Some("not-a-date"),
        None,
        None,
        Decimal::ZERO,
    );
    invalid.total_amount_origin = ValueOrigin::Derived;
    let result = evaluate(
        &[invoice],
        &[invalid],
        &[ledger("ledger-1", "001", "2026-07-02", dec!(100))],
    );
    let actual = codes(&result);
    assert!(actual.contains(&DocumentErrorCode::MissingInvoiceNumber));
    assert!(actual.contains(&DocumentErrorCode::InvalidDate));
    assert!(
        actual
            .iter()
            .filter(|code| **code == DocumentErrorCode::InvalidAmount)
            .count()
            >= 1
    );
}

#[test]
fn cancelled_workspace_and_bank_review_policy_do_not_regress() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger", DataSourceKind::Ledger511),
    ];
    let invoice = transaction(
        "invoice-1",
        "invoice",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let register = transaction(
        "register-1",
        "register",
        Some("001"),
        Some("2026-07-02"),
        Some(dec!(100)),
        Some(dec!(10)),
        dec!(110),
    );
    let audit = prepare_audit_session(
        "cancel-v18".to_string(),
        AccountingPeriod {
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        sources,
        HashMap::from([
            ("invoice".to_string(), vec![invoice]),
            ("register".to_string(), vec![register]),
            (
                "ledger".to_string(),
                vec![ledger("ledger-1", "001", "2026-07-02", dec!(100))],
            ),
        ]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    let token = AuditCancellationToken::default();
    token.cancel();
    let report = execute_audit_session_with_cancellation(audit, Decimal::ZERO, 0, &token)
        .expect("cancelled report");
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == REVENUE_CONTROL_ID)
            .expect("revenue")
            .status,
        ControlExecutionStatus::Cancelled
    );

    let ledger_record = CanonicalRecord {
        id: "ledger-bank".to_string(),
        source_id: "ledger112".to_string(),
        date: Some("2026-07-02".to_string()),
        debit_amount: Some(dec!(100)),
        total_amount: dec!(100),
        ..Default::default()
    };
    let bank = CanonicalRecord {
        id: "bank".to_string(),
        source_id: "bank".to_string(),
        date: Some("2026-07-02".to_string()),
        credit_amount: Some(dec!(100)),
        total_amount: dec!(100),
        ..Default::default()
    };
    let rule = ComparisonRule {
        id: "bank".to_string(),
        name: "bank".to_string(),
        semantic: ComparisonSemantic::BankPayment,
        primary_source_kind: DataSourceKind::Ledger112,
        primary_field: "directionalAmount".to_string(),
        secondary_source_kind: DataSourceKind::BankStatement,
        secondary_field: "directionalAmount".to_string(),
        is_required: true,
        tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
    };
    assert!(matches!(
        select_bank_candidate(
            &ledger_record,
            &DataSourceKind::Ledger112,
            &[&bank],
            &DataSourceKind::BankStatement,
            &rule,
            Decimal::ZERO,
            0,
        ),
        BankCandidateDecision::Suggested { .. }
    ));
}

#[test]
fn audit_workspace_enforces_strict_document_date_despite_session_tolerance() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger", DataSourceKind::Ledger511),
    ];
    let audit = prepare_audit_session(
        "strict-date".to_string(),
        AccountingPeriod {
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        sources,
        HashMap::from([
            (
                "invoice".to_string(),
                vec![transaction(
                    "invoice-1",
                    "invoice",
                    Some("001"),
                    Some("2026-07-02"),
                    Some(dec!(100)),
                    Some(dec!(10)),
                    dec!(110),
                )],
            ),
            (
                "register".to_string(),
                vec![transaction(
                    "register-1",
                    "register",
                    Some("001"),
                    Some("2026-07-03"),
                    Some(dec!(100)),
                    Some(dec!(10)),
                    dec!(110),
                )],
            ),
            (
                "ledger".to_string(),
                vec![ledger("ledger-1", "001", "2026-07-02", dec!(100))],
            ),
        ]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    let report =
        reconciliation_core::execute_audit_session(audit, dec!(999999), 30).expect("execute");
    let revenue = report
        .control_results
        .iter()
        .find(|result| result.control_id == REVENUE_CONTROL_ID)
        .expect("revenue");
    assert_eq!(revenue.status, ControlExecutionStatus::NeedsReview);
    assert_eq!(
        revenue
            .document_integrity_result
            .as_ref()
            .expect("document result")
            .summary
            .date_mismatch,
        1
    );
}

#[test]
fn malformed_bk_row_survives_normalization_and_review_plan_executes_validation() {
    let mut register_source = source("register", DataSourceKind::SalesRegister);
    register_source.column_mapping = ColumnMapping {
        date_column: Some("Ngày ct".to_string()),
        doc_no_column: Some("Số ct".to_string()),
        pretax_amount_column: Some("Tiền".to_string()),
        vat_amount_column: Some("Thuế".to_string()),
        total_amount_column: Some("Phải thu".to_string()),
        ..Default::default()
    };
    let headers = vec![
        "Ngày ct".to_string(),
        "Số ct".to_string(),
        "Tiền".to_string(),
        "Thuế".to_string(),
        "Phải thu".to_string(),
    ];
    let rows = vec![
        headers.clone(),
        vec![
            "not-a-date".to_string(),
            "001".to_string(),
            "not-money".to_string(),
            "bad".to_string(),
            "bad".to_string(),
        ],
    ];
    let normalized = normalize_data_source_rows(&register_source, &headers, &rows);
    assert_eq!(
        normalized.len(),
        1,
        "malformed BK row must retain provenance"
    );

    let invoice_source = source("invoice", DataSourceKind::EInvoice);
    let ledger_source = source("ledger", DataSourceKind::Ledger511);
    let audit = prepare_audit_session(
        "invalid-bk".to_string(),
        AccountingPeriod {
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        vec![invoice_source, register_source, ledger_source],
        HashMap::from([
            (
                "invoice".to_string(),
                vec![transaction(
                    "invoice-1",
                    "invoice",
                    Some("001"),
                    Some("2026-07-02"),
                    Some(dec!(100)),
                    Some(dec!(10)),
                    dec!(110),
                )],
            ),
            ("register".to_string(), normalized),
            (
                "ledger".to_string(),
                vec![ledger("ledger-1", "001", "2026-07-02", dec!(100))],
            ),
        ]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    assert_eq!(
        audit
            .control_plans
            .iter()
            .find(|plan| plan.control_id == REVENUE_CONTROL_ID)
            .expect("plan")
            .status,
        ControlPlanStatus::NeedsReview
    );
    let report = reconciliation_core::execute_audit_session(audit, Decimal::ZERO, 0)
        .expect("validation-only execution");
    let revenue = report
        .control_results
        .iter()
        .find(|result| result.control_id == REVENUE_CONTROL_ID)
        .expect("revenue result");
    assert_eq!(revenue.status, ControlExecutionStatus::NeedsReview);
    let document_result = revenue
        .document_integrity_result
        .as_ref()
        .expect("document result");
    assert!(document_result.summary.invalid_date >= 1);
    assert!(document_result.summary.invalid_amount >= 1);
}

#[test]
fn partial_control_failure_cannot_turn_document_error_into_pass() {
    let sources = vec![
        source("invoice", DataSourceKind::EInvoice),
        source("register", DataSourceKind::SalesRegister),
        source("ledger", DataSourceKind::Ledger511),
    ];
    let mut audit = prepare_audit_session(
        "partial-v18".to_string(),
        AccountingPeriod {
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        sources,
        HashMap::from([
            (
                "invoice".to_string(),
                vec![transaction(
                    "invoice-1",
                    "invoice",
                    Some("001"),
                    Some("2026-07-02"),
                    Some(dec!(100)),
                    Some(dec!(10)),
                    dec!(110),
                )],
            ),
            (
                "register".to_string(),
                vec![transaction(
                    "register-1",
                    "register",
                    Some("001"),
                    Some("2026-07-03"),
                    Some(dec!(100)),
                    Some(dec!(10)),
                    dec!(110),
                )],
            ),
            (
                "ledger".to_string(),
                vec![ledger("ledger-1", "001", "2026-07-02", dec!(100))],
            ),
        ]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .expect("prepare");
    audit.control_plans.insert(
        0,
        ControlPlan {
            control_id: "SYNTHETIC_FAILING_CONTROL".to_string(),
            title: "Synthetic failure".to_string(),
            status: ControlPlanStatus::Ready,
            source_ids: vec!["invoice".to_string()],
            missing_capabilities: vec![],
            warnings: vec![],
            effective_period: None,
        },
    );
    let report = reconciliation_core::execute_audit_session(audit, Decimal::ZERO, 30)
        .expect("partial report");
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == "SYNTHETIC_FAILING_CONTROL")
            .expect("failed control")
            .status,
        ControlExecutionStatus::Failed
    );
    assert_eq!(
        report
            .control_results
            .iter()
            .find(|result| result.control_id == REVENUE_CONTROL_ID)
            .expect("revenue")
            .status,
        ControlExecutionStatus::NeedsReview
    );
}

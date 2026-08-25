//! Local-only acceptance harness. It never embeds, copies, serializes, or
//! commits workbook data. Run explicitly with `--ignored --nocapture`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use reconciliation_core::{
    detect_header_and_mapping, execute_reconciliation, normalize_data_source_rows,
    normalize_partner_master_rows, normalize_sales_analysis_rows, read_sheet_rows, CanonicalRecord,
    DataSource, DataSourceKind, ReconciliationSession, SourceRole,
};
use rust_decimal::Decimal;

fn local_directory() -> PathBuf {
    std::env::var_os("APPKETOAN_LOCAL_TESTDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\appketoan\.local-testdata"))
}

fn source_from_rows(
    path: &Path,
    id: String,
) -> (
    DataSource,
    Vec<CanonicalRecord>,
    Vec<String>,
    Vec<Vec<String>>,
) {
    let metadata = reconciliation_core::inspect_excel_file(path.to_string_lossy().as_ref())
        .expect("local workbook metadata");
    let sheet = metadata.sheets.first().expect("at least one sheet");
    let rows =
        read_sheet_rows(path.to_string_lossy().as_ref(), &sheet.name).expect("local worksheet");
    let (header, start, columns, mapping, kind, _) = detect_header_and_mapping(&rows);
    let source = DataSource {
        id,
        name: "local".to_string(),
        file_path: path.to_string_lossy().to_string(),
        sheet_name: sheet.name.clone(),
        kind,
        role: SourceRole::RequiredSecondary,
        header_row: header,
        data_start_row: start,
        column_mapping: mapping,
    };
    let records = normalize_data_source_rows(&source, &columns, &rows);
    (source, records, columns, rows)
}

fn running_balance_equation_holds(records: &[CanonicalRecord]) -> Option<bool> {
    let balanced: Vec<_> = records
        .iter()
        .filter(|record| record.balance.is_some())
        .collect();
    if balanced.len() < 2 {
        return None;
    }

    Some(balanced.windows(2).all(|pair| {
        let previous = pair[0];
        let current = pair[1];
        let movement = current.debit_amount.unwrap_or(Decimal::ZERO)
            - current.credit_amount.unwrap_or(Decimal::ZERO);
        current.balance.expect("filtered") - previous.balance.expect("filtered") == movement
    }))
}

#[test]
#[ignore = "requires confidential workbooks under APPKETOAN_LOCAL_TESTDATA"]
fn local_real_acceptance_reports_sanitized_counts() {
    let directory = local_directory();
    let files: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("local testdata directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let is_excel_lock_file = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("~$"));
            matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("xls") | Some("xlsx")
            ) && !is_excel_lock_file
        })
        .collect();
    assert!(
        files.len() >= 7,
        "the local acceptance set must contain at least seven workbooks"
    );

    let mut sources = Vec::new();
    let mut records = HashMap::new();
    let mut partner_count = None;
    let mut partner_0502 = None;
    let mut sales_layers = None;
    for (index, path) in files.iter().enumerate() {
        let (source, normalized, columns, rows) = source_from_rows(path, format!("local-{index}"));
        match source.kind {
            DataSourceKind::PartnerMaster => {
                let partners = normalize_partner_master_rows(&source, &columns, &rows);
                partner_count = Some(partners.len());
                partner_0502 = Some(
                    partners
                        .iter()
                        .any(|partner| partner.partner_code.as_deref() == Some("0502")),
                );
            }
            DataSourceKind::SalesAnalysisReport => {
                let report = normalize_sales_analysis_rows(&source, &columns, &rows);
                let group = report
                    .iter()
                    .filter(|row| {
                        matches!(
                            row.row_level,
                            reconciliation_core::AnalyticalRowLevel::Group
                        )
                    })
                    .count();
                let detail = report
                    .iter()
                    .filter(|row| {
                        matches!(
                            row.row_level,
                            reconciliation_core::AnalyticalRowLevel::Detail
                        )
                    })
                    .count();
                sales_layers = Some((group, detail));
            }
            _ => {
                records.insert(source.id.clone(), normalized);
                sources.push(source);
            }
        }
    }

    for source in &mut sources {
        source.role = if source.kind == DataSourceKind::EInvoice {
            SourceRole::Primary
        } else {
            SourceRole::RequiredSecondary
        };
    }
    let invoice = sources
        .iter()
        .find(|source| source.kind == DataSourceKind::EInvoice)
        .expect("invoice source");
    let ledger = sources
        .iter()
        .find(|source| source.kind == DataSourceKind::Ledger511)
        .expect("ledger source");
    let session = ReconciliationSession {
        session_id: "local-baseline".to_string(),
        scenario_name: "local-baseline".to_string(),
        primary_source_id: Some(invoice.id.clone()),
        expected_primary_kind: Some(DataSourceKind::EInvoice),
        required_source_ids: Some(vec![invoice.id.clone(), ledger.id.clone()]),
        optional_source_ids: None,
        data_sources: vec![invoice.clone(), ledger.clone()],
        comparison_rules: vec![reconciliation_core::ComparisonRule {
            id: "revenue".to_string(),
            name: "revenue".to_string(),
            semantic: reconciliation_core::ComparisonSemantic::Revenue,
            primary_source_kind: DataSourceKind::EInvoice,
            primary_field: "pretaxAmount".to_string(),
            secondary_source_kind: DataSourceKind::Ledger511,
            secondary_field: "creditAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 3,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };
    let result = execute_reconciliation(&session, &records).expect("local baseline reconciliation");
    assert_eq!(records.get(&invoice.id).map_or(0, Vec::len), 46);
    assert_eq!(records.get(&ledger.id).map_or(0, Vec::len), 45);
    assert_eq!(result.summary.total_source_records, 46);
    assert_eq!(result.summary.total_target_records, 45);
    assert_eq!(result.summary.exact_matches_count, 45);
    assert_eq!(result.summary.missing_in_target_count, 1);
    assert_eq!(
        result.summary.total_discrepant_amount,
        Decimal::from(105_000_000u64)
    );
    let missing_233 = result
        .groups
        .iter()
        .find(|group| {
            group.doc_no.as_deref() == Some("233")
                && group.status == reconciliation_core::MatchStatus::UnmatchedMissingInTarget
        })
        .expect("invoice 233 must be missing from TK511");
    assert_eq!(missing_233.primary_source_record_ids.len(), 1);
    println!(
        "LOCAL_ACCEPTANCE baseline source={} target={} exact={} missing_target={} gross={}",
        result.summary.total_source_records,
        result.summary.total_target_records,
        result.summary.exact_matches_count,
        result.summary.missing_in_target_count,
        result.summary.total_discrepant_amount
    );
    let ledger112 = sources
        .iter()
        .find(|source| source.kind == DataSourceKind::Ledger112)
        .expect("TK112 source");
    let bank = sources
        .iter()
        .find(|source| source.kind == DataSourceKind::BankStatement)
        .expect("bank source");
    let bank_session = ReconciliationSession {
        session_id: "local-bank".to_string(),
        scenario_name: "local-bank".to_string(),
        primary_source_id: Some(ledger112.id.clone()),
        expected_primary_kind: Some(DataSourceKind::Ledger112),
        required_source_ids: Some(vec![ledger112.id.clone(), bank.id.clone()]),
        optional_source_ids: None,
        data_sources: vec![ledger112.clone(), bank.clone()],
        comparison_rules: vec![reconciliation_core::ComparisonRule {
            id: "bank".to_string(),
            name: "bank".to_string(),
            semantic: reconciliation_core::ComparisonSemantic::BankPayment,
            primary_source_kind: DataSourceKind::Ledger112,
            primary_field: "directionalAmount".to_string(),
            secondary_source_kind: DataSourceKind::BankStatement,
            secondary_field: "directionalAmount".to_string(),
            is_required: true,
            tolerance_vnd: Decimal::ZERO,
            date_tolerance_days: 3,
        }],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };
    let bank_result =
        execute_reconciliation(&bank_session, &records).expect("local bank reconciliation");
    assert_eq!(records.get(&bank.id).map_or(0, Vec::len), 225);
    let ledger112_records = records.get(&ledger112.id).expect("TK112 records");
    let balance_value_count = ledger112_records
        .iter()
        .filter(|record| record.balance.is_some())
        .count();
    let balance_equation = running_balance_equation_holds(ledger112_records);
    match balance_equation {
        Some(holds) => assert!(holds, "TK112 running balance equation must hold"),
        None => assert!(
            balance_value_count < 2,
            "NOT_VERIFIED is valid only without enough normalized balance values"
        ),
    }
    let count_reason = |reason: &str| {
        bank_result
            .groups
            .iter()
            .filter(|group| {
                group
                    .discrepancies
                    .iter()
                    .any(|discrepancy| discrepancy.message.contains(reason))
            })
            .count()
    };
    let ids_for_reason = |reason: &str| -> HashSet<String> {
        bank_result
            .groups
            .iter()
            .filter(|group| {
                group
                    .discrepancies
                    .iter()
                    .any(|discrepancy| discrepancy.message.contains(reason))
            })
            .flat_map(|group| group.target_source_record_ids.iter().cloned())
            .collect()
    };
    let accepted_secondary_ids = ids_for_reason("BANK_MATCH_EVIDENCE");
    let suggested_candidates = ids_for_reason("SUGGESTED_DIRECTION_AMOUNT_DATE");
    let ambiguous_candidates = ids_for_reason("AMBIGUOUS_BANK_CANDIDATES");
    let ambiguous_secondary_ids: HashSet<String> = ambiguous_candidates
        .difference(&accepted_secondary_ids)
        .cloned()
        .collect();
    let suggested_secondary_ids: HashSet<String> = suggested_candidates
        .difference(&accepted_secondary_ids)
        .filter(|id| !ambiguous_secondary_ids.contains(*id))
        .cloned()
        .collect();
    let review_linked_secondary_ids: HashSet<String> = suggested_secondary_ids
        .union(&ambiguous_secondary_ids)
        .cloned()
        .collect();
    let truly_unlinked_secondary_ids: HashSet<String> = bank_result
        .groups
        .iter()
        .filter(|group| group.status == reconciliation_core::MatchStatus::UnmatchedMissingInSource)
        .flat_map(|group| group.target_source_record_ids.iter().cloned())
        .collect();
    assert!(accepted_secondary_ids.is_disjoint(&truly_unlinked_secondary_ids));
    assert!(review_linked_secondary_ids.is_disjoint(&truly_unlinked_secondary_ids));
    let classified_secondary_ids: HashSet<String> = accepted_secondary_ids
        .union(&review_linked_secondary_ids)
        .cloned()
        .chain(truly_unlinked_secondary_ids.iter().cloned())
        .collect();
    let all_bank_ids: HashSet<String> = records
        .get(&bank.id)
        .expect("bank records")
        .iter()
        .map(|record| record.id.clone())
        .collect();
    assert_eq!(classified_secondary_ids, all_bank_ids);
    let ledger_only = bank_result
        .groups
        .iter()
        .filter(|group| group.status == reconciliation_core::MatchStatus::UnmatchedMissingInTarget)
        .count();
    println!(
        "LOCAL_ACCEPTANCE bank parsed={} tk112={} balance_equation={:?} strong_accepted={} suggested_review_linked={} ambiguous_review_linked={} true_bank_only={} true_ledger_only={} direction_conflict={} insufficient={}",
        records.get(&bank.id).map_or(0, Vec::len), records.get(&ledger112.id).map_or(0, Vec::len),
        balance_equation,
        accepted_secondary_ids.len(), suggested_secondary_ids.len(), ambiguous_secondary_ids.len(),
        truly_unlinked_secondary_ids.len(), ledger_only, count_reason("DIRECTION_CONFLICT"), count_reason("INSUFFICIENT_BANK_EVIDENCE")
    );
    let register = sources
        .iter()
        .filter(|source| source.kind == DataSourceKind::SalesRegister)
        .max_by_key(|source| records.get(&source.id).map_or(0, Vec::len))
        .expect("sales register source");
    let tri_session = ReconciliationSession {
        session_id: "local-tri".to_string(),
        scenario_name: "local-tri".to_string(),
        primary_source_id: Some(invoice.id.clone()),
        expected_primary_kind: Some(DataSourceKind::EInvoice),
        required_source_ids: Some(vec![
            invoice.id.clone(),
            register.id.clone(),
            ledger.id.clone(),
        ]),
        optional_source_ids: None,
        data_sources: vec![invoice.clone(), register.clone(), ledger.clone()],
        comparison_rules: vec![
            reconciliation_core::ComparisonRule {
                id: "bk-revenue".to_string(),
                name: "bk-revenue".to_string(),
                semantic: reconciliation_core::ComparisonSemantic::Revenue,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "pretaxAmount".to_string(),
                secondary_source_kind: DataSourceKind::SalesRegister,
                secondary_field: "pretaxAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 3,
            },
            reconciliation_core::ComparisonRule {
                id: "bk-vat".to_string(),
                name: "bk-vat".to_string(),
                semantic: reconciliation_core::ComparisonSemantic::Vat,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "vatAmount".to_string(),
                secondary_source_kind: DataSourceKind::SalesRegister,
                secondary_field: "vatAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 3,
            },
            reconciliation_core::ComparisonRule {
                id: "bk-receivable".to_string(),
                name: "bk-receivable".to_string(),
                semantic: reconciliation_core::ComparisonSemantic::Receivable,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "totalAmount".to_string(),
                secondary_source_kind: DataSourceKind::SalesRegister,
                secondary_field: "totalAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 3,
            },
            reconciliation_core::ComparisonRule {
                id: "ledger-revenue".to_string(),
                name: "ledger-revenue".to_string(),
                semantic: reconciliation_core::ComparisonSemantic::Revenue,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "pretaxAmount".to_string(),
                secondary_source_kind: DataSourceKind::Ledger511,
                secondary_field: "creditAmount".to_string(),
                is_required: true,
                tolerance_vnd: Decimal::ZERO,
                date_tolerance_days: 3,
            },
        ],
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 3,
        enable_aggregate_match: false,
    };
    let tri = execute_reconciliation(&tri_session, &records).expect("local tri reconciliation");
    let document_233 = tri
        .groups
        .iter()
        .find(|group| {
            group.doc_no.as_deref() == Some("233") && !group.primary_source_record_ids.is_empty()
        })
        .expect("tri-source invoice 233");
    assert_eq!(
        document_233.status,
        reconciliation_core::MatchStatus::NeedsReview
    );
    assert!(document_233
        .discrepancies
        .iter()
        .any(|discrepancy| discrepancy.message.contains("HIGH")));
    assert_eq!(partner_0502, Some(true));
    assert_eq!(sales_layers, Some((35, 54)));
    println!(
        "LOCAL_ACCEPTANCE tri_233={:?} partner_0502={:?}",
        document_233.status, partner_0502
    );
    println!(
        "LOCAL_ACCEPTANCE partner_records={:?} sales_layers={:?}",
        partner_count, sales_layers
    );
}

//! Local-only V16 planner acceptance. Workbook content never leaves the local
//! process and no row value is serialized or committed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use reconciliation_core::{
    compute_record_fingerprint, detect_header_and_mapping, execute_audit_session,
    normalize_data_source_rows, normalize_partner_master_rows, normalize_sales_analysis_rows,
    prepare_audit_session, read_sheet_rows, AccountingPeriod, AnalyticalRowLevel, CanonicalRecord,
    ControlExecutionStatus, ControlPlanStatus, DataSource, DataSourceKind, PartnerRecord,
    SalesAnalysisRecord, SourceRole, BANK_CONTROL_ID, PARTNER_CONTROL_ID, RECEIVABLE_CONTROL_ID,
    REVENUE_CONTROL_ID, SALES_ANALYSIS_CONTROL_ID, VAT_CONTROL_ID,
};
use rust_decimal::Decimal;

fn local_directory() -> PathBuf {
    std::env::var_os("APPKETOAN_LOCAL_TESTDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\appketoan\.local-testdata"))
}

fn read_once(path: &Path, id: String) -> (DataSource, Vec<String>, Vec<Vec<String>>) {
    let metadata = reconciliation_core::inspect_excel_file(path.to_string_lossy().as_ref())
        .expect("local workbook metadata");
    let sheet = metadata.sheets.first().expect("at least one sheet");
    let rows = read_sheet_rows(path.to_string_lossy().as_ref(), &sheet.name)
        .expect("local worksheet read once");
    let (header, start, columns, mapping, kind, _) = detect_header_and_mapping(&rows);
    (
        DataSource {
            id,
            name: "local-source".to_string(),
            file_path: path.to_string_lossy().to_string(),
            sheet_name: sheet.name.clone(),
            kind,
            role: SourceRole::RequiredSecondary,
            header_row: header,
            data_start_row: start,
            column_mapping: mapping,
        },
        columns,
        rows,
    )
}

fn plan<'a>(
    report: &'a reconciliation_core::AuditWorkspaceReport,
    id: &str,
) -> &'a reconciliation_core::ControlPlan {
    report
        .control_plans
        .iter()
        .find(|plan| plan.control_id == id)
        .expect("control plan")
}

#[test]
#[ignore = "requires confidential workbooks under APPKETOAN_LOCAL_TESTDATA"]
fn local_v16_planner_acceptance() {
    let files: Vec<PathBuf> = std::fs::read_dir(local_directory())
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
    assert!(files.len() >= 7);

    let mut sources = Vec::new();
    let mut transactional: HashMap<String, Vec<CanonicalRecord>> = HashMap::new();
    let mut partner_masters: HashMap<String, Vec<PartnerRecord>> = HashMap::new();
    let mut sales_analysis: HashMap<String, Vec<SalesAnalysisRecord>> = HashMap::new();
    let mut partner_0502 = false;
    let mut sales_layers = None;

    for (index, path) in files.iter().enumerate() {
        let (source, columns, rows) = read_once(path, format!("local-{index}"));
        match source.kind {
            DataSourceKind::PartnerMaster => {
                let records = normalize_partner_master_rows(&source, &columns, &rows);
                partner_0502 = records
                    .iter()
                    .any(|partner| partner.partner_code.as_deref() == Some("0502"));
                partner_masters.insert(source.id.clone(), records);
            }
            DataSourceKind::SalesAnalysisReport => {
                let records = normalize_sales_analysis_rows(&source, &columns, &rows);
                sales_layers = Some((
                    records
                        .iter()
                        .filter(|row| row.row_level == AnalyticalRowLevel::Group)
                        .count(),
                    records
                        .iter()
                        .filter(|row| row.row_level == AnalyticalRowLevel::Detail)
                        .count(),
                ));
                sales_analysis.insert(source.id.clone(), records);
            }
            _ => {
                transactional.insert(
                    source.id.clone(),
                    normalize_data_source_rows(&source, &columns, &rows),
                );
            }
        }
        sources.push(source);
    }

    // A local folder may contain a derived subset workbook. Collapse only a
    // proven same-kind content subset; unrelated duplicate capabilities remain
    // visible to the production planner as NEEDS_REVIEW.
    let mut subset_source_ids = std::collections::HashSet::new();
    for (left_index, left) in sources.iter().enumerate() {
        let Some(left_records) = transactional.get(&left.id) else {
            continue;
        };
        let left_fingerprints: std::collections::HashSet<_> = left_records
            .iter()
            .map(|record| compute_record_fingerprint(record, &left.kind))
            .collect();
        for right in sources.iter().skip(left_index + 1) {
            if left.kind != right.kind {
                continue;
            }
            let Some(right_records) = transactional.get(&right.id) else {
                continue;
            };
            let right_fingerprints: std::collections::HashSet<_> = right_records
                .iter()
                .map(|record| compute_record_fingerprint(record, &right.kind))
                .collect();
            if left_fingerprints == right_fingerprints {
                subset_source_ids.insert(right.id.clone());
            } else if left_fingerprints.len() < right_fingerprints.len()
                && left_fingerprints.is_subset(&right_fingerprints)
            {
                subset_source_ids.insert(left.id.clone());
            } else if right_fingerprints.len() < left_fingerprints.len()
                && right_fingerprints.is_subset(&left_fingerprints)
            {
                subset_source_ids.insert(right.id.clone());
            }
        }
    }
    sources.retain(|source| !subset_source_ids.contains(&source.id));
    transactional.retain(|source_id, _| !subset_source_ids.contains(source_id));
    assert_eq!(sources.len(), 7, "expected seven logical local sources");

    let audit = prepare_audit_session(
        "local-v16".to_string(),
        AccountingPeriod {
            start_date: "2026-06-01".to_string(),
            end_date: "2026-07-31".to_string(),
        },
        sources,
        transactional,
        partner_masters,
        sales_analysis,
        HashMap::new(),
    )
    .expect("prepare V16 audit session");
    let report = execute_audit_session(audit, Decimal::ZERO, 3).expect("execute V16 controls");

    for source in &report.source_catalog.sources {
        println!(
            "V16_PERIOD kind={:?} records={} in_period={} outside={} missing_date={} earliest={:?} latest={:?}",
            source.source_kind,
            source.record_count,
            source.period_evidence.records_in_period,
            source.period_evidence.records_outside_period,
            source.period_evidence.missing_or_unparseable_dates,
            source.period_evidence.earliest_date,
            source.period_evidence.latest_date
        );
    }

    for id in [
        REVENUE_CONTROL_ID,
        BANK_CONTROL_ID,
        PARTNER_CONTROL_ID,
        SALES_ANALYSIS_CONTROL_ID,
    ] {
        assert_eq!(plan(&report, id).status, ControlPlanStatus::Ready, "{id}");
    }
    for id in [RECEIVABLE_CONTROL_ID, VAT_CONTROL_ID] {
        assert_eq!(
            plan(&report, id).status,
            ControlPlanStatus::MissingSource,
            "{id}"
        );
    }
    assert!(report.source_reuse.iter().all(|source| {
        source.read_count == 1 && source.normalize_count == 1 && source.index_count == 1
    }));

    let revenue = report
        .control_results
        .iter()
        .find(|result| result.control_id == REVENUE_CONTROL_ID)
        .and_then(|result| result.reconciliation_result.as_ref())
        .expect("revenue reconciliation result");
    assert_eq!(revenue.summary.total_source_records, 46);
    assert_eq!(revenue.summary.exact_matches_count, 45);
    let document_233 = revenue
        .groups
        .iter()
        .find(|group| {
            group.doc_no.as_deref() == Some("233") && !group.primary_source_record_ids.is_empty()
        })
        .expect("document 233");
    assert_eq!(
        document_233.status,
        reconciliation_core::MatchStatus::NeedsReview
    );
    assert!(document_233.semantic_comparisons.iter().any(|comparison| {
        comparison.secondary_source_kind == DataSourceKind::Ledger511
            && comparison.variance == Decimal::from(105_000_000u64)
    }));
    let revenue_control = report
        .control_results
        .iter()
        .find(|result| result.control_id == REVENUE_CONTROL_ID)
        .expect("revenue control");
    assert_eq!(revenue_control.status, ControlExecutionStatus::NeedsReview);
    assert!(revenue_control
        .findings
        .iter()
        .any(|finding| finding.severity == "HIGH"));

    let bank = report
        .control_results
        .iter()
        .find(|result| result.control_id == BANK_CONTROL_ID)
        .and_then(|result| result.reconciliation_result.as_ref())
        .expect("bank reconciliation result");
    let strong_accepted = bank
        .groups
        .iter()
        .filter(|group| {
            group
                .discrepancies
                .iter()
                .any(|item| item.message.contains("BANK_MATCH_EVIDENCE"))
        })
        .count();
    assert_eq!(strong_accepted, 0);
    assert!(bank.groups.iter().all(|group| {
        let review_link = group.discrepancies.iter().any(|item| {
            item.message.contains("SUGGESTED_DIRECTION_AMOUNT_DATE")
                || item.message.contains("AMBIGUOUS_BANK_CANDIDATES")
        });
        !review_link || group.status == reconciliation_core::MatchStatus::NeedsReview
    }));
    assert!(partner_0502);
    assert_eq!(sales_layers, Some((35, 54)));

    let ready = report
        .control_plans
        .iter()
        .filter(|plan| plan.status == ControlPlanStatus::Ready)
        .count();
    let missing = report
        .control_plans
        .iter()
        .filter(|plan| plan.status == ControlPlanStatus::MissingSource)
        .count();
    println!(
        "V16_LOCAL_ACCEPTANCE ready={ready} missing={missing} baseline=46/45/45 missing_doc=1 gross=105000000 bank_strong={strong_accepted} partner_0502={partner_0502} sales_layers=35/54"
    );
}

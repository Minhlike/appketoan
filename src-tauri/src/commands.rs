use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use reconciliation_core::{
    evaluate_partner_master_control, evaluate_sales_analysis_control, execute_audit_session,
    execute_reconciliation, execute_reference_controls, export_reconciliation_to_excel,
    filter_reconciliation_session_and_records, inspect_excel_bytes, inspect_excel_file,
    normalize_data_source_rows, normalize_partner_master_rows, normalize_sales_analysis_rows,
    prepare_audit_session, read_sheet_rows, read_sheet_rows_from_bytes, AccountingPeriod,
    AuditWorkspaceReport, CanonicalRecord, DataSource, DataSourceKind, ExcelFileMetadata,
    ExportSummary, PartnerRecord, ReconciliationResult, ReconciliationSession, SalesAnalysisRecord,
};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

fn sha256_file_streaming(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Không thể mở file để băm: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Không thể đọc file để băm: {}", e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub fn cmd_inspect_excel_file(file_path: String) -> Result<ExcelFileMetadata, String> {
    inspect_excel_file(&file_path)
}

#[tauri::command]
pub fn cmd_inspect_excel_bytes(
    bytes: Vec<u8>,
    file_name: String,
) -> Result<ExcelFileMetadata, String> {
    inspect_excel_bytes(&bytes, &file_name)
}

#[tauri::command]
pub fn cmd_run_reconciliation(
    session: ReconciliationSession,
    file_bytes_map: Option<HashMap<String, Vec<u8>>>,
) -> Result<ReconciliationResult, String> {
    let mut source_records_map: HashMap<String, Vec<CanonicalRecord>> = HashMap::new();
    let mut raw_file_hashes: HashMap<String, String> = HashMap::new();
    let mut reference_controls = Vec::new();
    let mut partner_masters = Vec::new();
    let bytes_map = file_bytes_map.unwrap_or_default();

    for source in &session.data_sources {
        // Read raw rows and record raw SHA256
        let (raw_rows, header_cols, raw_hash) = if let Some(bytes) = bytes_map
            .get(&source.id)
            .or_else(|| bytes_map.get(&source.file_path))
        {
            let hash = format!("{:x}", Sha256::digest(bytes));
            let meta = inspect_excel_bytes(bytes, &source.name)
                .map_err(|e| format!("Lỗi kiểm tra file {}: {}", source.name, e))?;
            let sheet_meta = if source.sheet_name.trim().is_empty() {
                meta.sheets.first().ok_or_else(|| {
                    format!(
                        "SHEET_NOT_FOUND: File {} không chứa bất kỳ sheet nào",
                        source.name
                    )
                })?
            } else {
                meta.sheets
                    .iter()
                    .find(|s| s.name.trim().eq_ignore_ascii_case(source.sheet_name.trim()))
                    .ok_or_else(|| {
                        format!(
                            "SHEET_NOT_FOUND: Không tìm thấy sheet '{}' trong file {}",
                            source.sheet_name, source.name
                        )
                    })?
            };

            let rows = read_sheet_rows_from_bytes(bytes, &sheet_meta.name).map_err(|e| {
                format!(
                    "Lỗi đọc sheet '{}' trong file {}: {}",
                    sheet_meta.name, source.name, e
                )
            })?;

            (rows, sheet_meta.columns.clone(), Some(hash))
        } else if Path::new(&source.file_path).exists() {
            let hash = sha256_file_streaming(Path::new(&source.file_path))?;
            let meta = inspect_excel_file(&source.file_path)
                .map_err(|e| format!("Lỗi kiểm tra file {}: {}", source.name, e))?;
            let sheet_meta = if source.sheet_name.trim().is_empty() {
                meta.sheets.first().ok_or_else(|| {
                    format!(
                        "SHEET_NOT_FOUND: File {} không chứa bất kỳ sheet nào",
                        source.name
                    )
                })?
            } else {
                meta.sheets
                    .iter()
                    .find(|s| s.name.trim().eq_ignore_ascii_case(source.sheet_name.trim()))
                    .ok_or_else(|| {
                        format!(
                            "SHEET_NOT_FOUND: Không tìm thấy sheet '{}' trong file {}",
                            source.sheet_name, source.name
                        )
                    })?
            };

            let rows = read_sheet_rows(&source.file_path, &sheet_meta.name).map_err(|e| {
                format!(
                    "Lỗi đọc sheet '{}' trong file {}: {}",
                    sheet_meta.name, source.name, e
                )
            })?;

            (rows, sheet_meta.columns.clone(), Some(hash))
        } else {
            return Err(format!(
                "Không tìm thấy file nguồn '{}' tại '{}'",
                source.name, source.file_path
            ));
        };

        if let Some(h) = raw_hash {
            raw_file_hashes.insert(source.id.clone(), h);
        }

        match source.kind {
            DataSourceKind::PartnerMaster => {
                let records = normalize_partner_master_rows(source, &header_cols, &raw_rows);
                reference_controls
                    .push(evaluate_partner_master_control(source.id.clone(), &records));
                partner_masters.push((source.id.clone(), records));
            }
            DataSourceKind::SalesAnalysisReport => {
                let records = normalize_sales_analysis_rows(source, &header_cols, &raw_rows);
                reference_controls
                    .push(evaluate_sales_analysis_control(source.id.clone(), &records));
            }
            _ => {
                let records = normalize_data_source_rows(source, &header_cols, &raw_rows);
                source_records_map.insert(source.id.clone(), records);
            }
        }
    }

    // Cross-source partner identity is a read-only control. It does not merge
    // or rewrite canonical transaction records and therefore preserves source
    // provenance while allowing master + transaction audit sessions.
    if !source_records_map.is_empty() {
        let partner_identity_source_ids: std::collections::HashSet<&str> = session
            .data_sources
            .iter()
            .filter(|source| {
                matches!(
                    source.kind,
                    DataSourceKind::EInvoice
                        | DataSourceKind::SalesRegister
                        | DataSourceKind::Ledger511
                        | DataSourceKind::Ledger112
                        | DataSourceKind::Ledger131
                        | DataSourceKind::Ledger133
                        | DataSourceKind::Ledger3331
                )
            })
            .map(|source| source.id.as_str())
            .collect();
        for (master_source_id, masters) in &partner_masters {
            reference_controls.push(reconciliation_core::evaluate_partner_identity_cross_source(
                master_source_id.clone(),
                masters,
                source_records_map
                    .iter()
                    .filter(|(source_id, _)| {
                        partner_identity_source_ids.contains(source_id.as_str())
                    })
                    .flat_map(|(_, records)| records.iter()),
            ));
        }
    }

    if source_records_map.is_empty() {
        return execute_reference_controls(&session, reference_controls);
    }

    // Reference sources remain typed controls, but are allowed to coexist with
    // transactional sources in one audit session. They are deliberately not
    // injected into the transaction matcher as empty pseudo-datasets.
    let transactional_source_ids: std::collections::HashSet<&str> =
        source_records_map.keys().map(String::as_str).collect();
    let transactional_session =
        reconciliation_core::transactional_session_from(&session, &transactional_source_ids);

    // 1. Pass through intake dedup & dataset identity gate
    let (filtered_session, filtered_records_map, intake_analysis) =
        filter_reconciliation_session_and_records(
            &transactional_session,
            &source_records_map,
            &raw_file_hashes,
        )?;

    // 2. Execute core reconciliation on clean logical datasets
    let mut result = execute_reconciliation(&filtered_session, &filtered_records_map)?;
    result.intake_analysis = Some(intake_analysis);
    result.reference_controls = reference_controls;

    Ok(result)
}

#[tauri::command]
pub fn cmd_run_audit_workspace(
    session_id: String,
    accounting_period: AccountingPeriod,
    data_sources: Vec<DataSource>,
    matching_tolerance_vnd: Decimal,
    date_tolerance_days: u32,
    file_bytes_map: Option<HashMap<String, Vec<u8>>>,
) -> Result<AuditWorkspaceReport, String> {
    let bytes_map = file_bytes_map.unwrap_or_default();
    let mut transactional: HashMap<String, Vec<CanonicalRecord>> = HashMap::new();
    let mut partner_masters: HashMap<String, Vec<PartnerRecord>> = HashMap::new();
    let mut sales_analysis: HashMap<String, Vec<SalesAnalysisRecord>> = HashMap::new();
    let mut provenance = HashMap::new();
    let header_columns =
        |source: &DataSource, rows: &[Vec<String>]| -> Result<Vec<String>, String> {
            rows.get(source.header_row.saturating_sub(1) as usize)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "HEADER_ROW_NOT_FOUND: dòng tiêu đề {} không tồn tại trong {}",
                        source.header_row, source.name
                    )
                })
        };

    // This is the only physical ingestion loop for the audit workspace. Every
    // normalized dataset is retained in AuditSession and reused by all plans.
    for source in &data_sources {
        let (raw_rows, header_cols, raw_hash) = if let Some(bytes) = bytes_map
            .get(&source.id)
            .or_else(|| bytes_map.get(&source.file_path))
        {
            let rows = read_sheet_rows_from_bytes(bytes, &source.sheet_name).map_err(|error| {
                format!(
                    "Lỗi đọc sheet '{}' trong {}: {error}",
                    source.sheet_name, source.name
                )
            })?;
            let columns = header_columns(source, &rows)?;
            (rows, columns, format!("{:x}", Sha256::digest(bytes)))
        } else if Path::new(&source.file_path).exists() {
            let rows = read_sheet_rows(&source.file_path, &source.sheet_name).map_err(|error| {
                format!(
                    "Lỗi đọc sheet '{}' trong {}: {error}",
                    source.sheet_name, source.name
                )
            })?;
            let columns = header_columns(source, &rows)?;
            (
                rows,
                columns,
                sha256_file_streaming(Path::new(&source.file_path))?,
            )
        } else {
            return Err(format!(
                "Không tìm thấy file nguồn '{}' tại '{}'",
                source.name, source.file_path
            ));
        };

        provenance.insert(source.id.clone(), raw_hash);
        match source.kind {
            DataSourceKind::PartnerMaster => {
                partner_masters.insert(
                    source.id.clone(),
                    normalize_partner_master_rows(source, &header_cols, &raw_rows),
                );
            }
            DataSourceKind::SalesAnalysisReport => {
                sales_analysis.insert(
                    source.id.clone(),
                    normalize_sales_analysis_rows(source, &header_cols, &raw_rows),
                );
            }
            _ => {
                transactional.insert(
                    source.id.clone(),
                    normalize_data_source_rows(source, &header_cols, &raw_rows),
                );
            }
        }
    }

    let audit = prepare_audit_session(
        session_id,
        accounting_period,
        data_sources,
        transactional,
        partner_masters,
        sales_analysis,
        provenance,
    )?;
    execute_audit_session(audit, matching_tolerance_vnd, date_tolerance_days)
}

#[tauri::command]
pub fn cmd_export_reconciliation_report(
    result: ReconciliationResult,
    output_path: Option<String>,
) -> Result<ExportSummary, String> {
    let target_path = if let Some(p) = output_path {
        PathBuf::from(p)
    } else {
        let download_dir = std::env::var("USERPROFILE")
            .map(|p| PathBuf::from(p).join("Downloads"))
            .unwrap_or_else(|_| std::env::temp_dir());
        let file_name = format!("Bao_Cao_Doi_Chieu_{}.xlsx", result.session_id);
        download_dir.join(file_name)
    };

    export_reconciliation_to_excel(&result, &target_path)
}

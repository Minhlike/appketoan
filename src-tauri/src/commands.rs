use std::collections::HashMap;
use std::path::{Path, PathBuf};

use reconciliation_core::{
    execute_reconciliation, export_reconciliation_to_excel,
    filter_reconciliation_session_and_records, inspect_excel_bytes, inspect_excel_file,
    normalize_data_source_rows, read_sheet_rows, read_sheet_rows_from_bytes, CanonicalRecord,
    ExcelFileMetadata, ExportSummary, ReconciliationResult, ReconciliationSession,
};
use sha2::{Digest, Sha256};

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
            let file_bytes = std::fs::read(&source.file_path)
                .map_err(|e| format!("Lỗi đọc file {}: {}", source.file_path, e))?;
            let hash = format!("{:x}", Sha256::digest(&file_bytes));
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

        let records = normalize_data_source_rows(source, &header_cols, &raw_rows);
        source_records_map.insert(source.id.clone(), records);
    }

    // 1. Pass through intake dedup & dataset identity gate
    let (filtered_session, filtered_records_map, intake_analysis) =
        filter_reconciliation_session_and_records(&session, &source_records_map, &raw_file_hashes)?;

    // 2. Execute core reconciliation on clean logical datasets
    let mut result = execute_reconciliation(&filtered_session, &filtered_records_map)?;
    result.intake_analysis = Some(intake_analysis);

    Ok(result)
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

use std::collections::HashMap;
use std::path::PathBuf;

use reconciliation_core::{
    execute_reconciliation, export_reconciliation_to_excel, inspect_excel_bytes,
    inspect_excel_file, normalize_data_source_rows, read_sheet_rows, read_sheet_rows_from_bytes,
    CanonicalRecord, ExcelFileMetadata, ExportSummary, ReconciliationResult, ReconciliationSession,
};

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
    let bytes_map = file_bytes_map.unwrap_or_default();

    for source in &session.data_sources {
        // Read raw rows
        let (raw_rows, header_cols) = if let Some(bytes) = bytes_map.get(&source.id).or_else(|| bytes_map.get(&source.file_path)) {
            let meta = inspect_excel_bytes(bytes, &source.name)
                .map_err(|e| format!("Lỗi kiểm tra file {}: {}", source.name, e))?;
            let sheet_meta = meta
                .sheets
                .iter()
                .find(|s| s.name == source.sheet_name)
                .or_else(|| meta.sheets.first())
                .ok_or_else(|| format!("Không tìm thấy sheet '{}' trong file {}", source.sheet_name, source.name))?;

            let rows = read_sheet_rows_from_bytes(bytes, &sheet_meta.name)
                .map_err(|e| format!("Lỗi đọc sheet '{}' trong file {}: {}", sheet_meta.name, source.name, e))?;

            (rows, sheet_meta.columns.clone())
        } else {
            let meta = inspect_excel_file(&source.file_path)
                .map_err(|e| format!("Lỗi kiểm tra file {}: {}", source.name, e))?;
            let sheet_meta = meta
                .sheets
                .iter()
                .find(|s| s.name == source.sheet_name)
                .or_else(|| meta.sheets.first())
                .ok_or_else(|| format!("Không tìm thấy sheet '{}' trong file {}", source.sheet_name, source.name))?;

            let rows = read_sheet_rows(&source.file_path, &sheet_meta.name)
                .map_err(|e| format!("Lỗi đọc sheet '{}' trong file {}: {}", sheet_meta.name, source.name, e))?;

            (rows, sheet_meta.columns.clone())
        };

        let records = normalize_data_source_rows(source, &header_cols, &raw_rows);
        source_records_map.insert(source.id.clone(), records);
    }

    Ok(execute_reconciliation(&session, &source_records_map))
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

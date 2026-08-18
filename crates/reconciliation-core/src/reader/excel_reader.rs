use std::path::Path;
use calamine::{open_workbook_auto, open_workbook_auto_from_rs, Data, Range, Reader};
use std::io::Cursor;

use crate::models::{ExcelFileMetadata, SheetMetadata};
use crate::reader::header_detector::detect_header_and_mapping_with_context;

/// Formats a Calamine cell value into a clean string representation
pub fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => format!("{}", b),
        Data::DateTime(d) => {
            format!("{}", d)
        }
        Data::DateTimeIso(iso) => iso.clone(),
        Data::DurationIso(dur) => dur.clone(),
        Data::Error(e) => format!("ERR:{:?}", e),
    }
}

/// Converts a 2D Range into a Vec of string rows
pub fn range_to_string_rows(range: &Range<Data>, max_rows: Option<usize>) -> Vec<Vec<String>> {
    let height = if let Some(max) = max_rows {
        range.height().min(max)
    } else {
        range.height()
    };

    let width = range.width();
    let mut rows = Vec::with_capacity(height);

    for r in 0..height {
        let mut row_vec = Vec::with_capacity(width);
        for c in 0..width {
            if let Some(cell) = range.get((r, c)) {
                row_vec.push(cell_to_string(cell));
            } else {
                row_vec.push(String::new());
            }
        }
        rows.push(row_vec);
    }

    rows
}

/// Inspects an Excel workbook file from a file path
pub fn inspect_excel_file<P: AsRef<Path>>(path: P) -> Result<ExcelFileMetadata, String> {
    let p = path.as_ref();
    let file_name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown.xlsx".to_string());

    let file_size_bytes = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);

    let mut workbook = open_workbook_auto(p)
        .map_err(|e| format!("Không thể mở file Excel: {}. Hãy kiểm tra định dạng file.", e))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut sheets = Vec::with_capacity(sheet_names.len());

    for sheet_name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            let total_rows = range.height();
            let total_cols = range.width();

            let preview_string_rows = range_to_string_rows(&range, Some(25));
            let (detected_header, detected_start, columns, suggested_mapping, suggested_kind, conf) =
                detect_header_and_mapping_with_context(sheet_name, &preview_string_rows);

            let preview_rows: Vec<Vec<String>> = preview_string_rows
                .into_iter()
                .take(10)
                .collect();

            sheets.push(SheetMetadata {
                name: sheet_name.clone(),
                total_rows,
                total_cols,
                detected_header_row: detected_header,
                detected_data_start_row: detected_start,
                columns,
                suggested_mapping,
                suggested_kind,
                confidence_score: conf,
                preview_rows,
            });
        }
    }

    Ok(ExcelFileMetadata {
        file_path: p.to_string_lossy().to_string(),
        file_name,
        file_size_bytes,
        sheets,
    })
}

/// Inspects an Excel workbook from memory bytes
pub fn inspect_excel_bytes(bytes: &[u8], file_name: &str) -> Result<ExcelFileMetadata, String> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("Không thể đọc dữ liệu Excel: {}", e))?;

    let sheet_names = workbook.sheet_names().to_vec();
    let mut sheets = Vec::with_capacity(sheet_names.len());

    for sheet_name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            let total_rows = range.height();
            let total_cols = range.width();

            let preview_string_rows = range_to_string_rows(&range, Some(25));
            let (detected_header, detected_start, columns, suggested_mapping, suggested_kind, conf) =
                detect_header_and_mapping_with_context(sheet_name, &preview_string_rows);

            let preview_rows: Vec<Vec<String>> = preview_string_rows
                .into_iter()
                .take(10)
                .collect();

            sheets.push(SheetMetadata {
                name: sheet_name.clone(),
                total_rows,
                total_cols,
                detected_header_row: detected_header,
                detected_data_start_row: detected_start,
                columns,
                suggested_mapping,
                suggested_kind,
                confidence_score: conf,
                preview_rows,
            });
        }
    }

    Ok(ExcelFileMetadata {
        file_path: file_name.to_string(),
        file_name: file_name.to_string(),
        file_size_bytes: bytes.len() as u64,
        sheets,
    })
}

/// Reads all rows from a specific worksheet in an Excel workbook
pub fn read_sheet_rows<P: AsRef<Path>>(
    path: P,
    sheet_name: &str,
) -> Result<Vec<Vec<String>>, String> {
    let mut workbook = open_workbook_auto(path.as_ref())
        .map_err(|e| format!("Không thể mở file: {}", e))?;

    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| format!("Không tìm thấy sheet '{}': {}", sheet_name, e))?;

    Ok(range_to_string_rows(&range, None))
}

/// Reads all rows from a specific worksheet in an Excel byte buffer
pub fn read_sheet_rows_from_bytes(
    bytes: &[u8],
    sheet_name: &str,
) -> Result<Vec<Vec<String>>, String> {
    let cursor = Cursor::new(bytes);
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|e| format!("Không thể đọc dữ liệu: {}", e))?;

    let range = workbook
        .worksheet_range(sheet_name)
        .map_err(|e| format!("Không tìm thấy sheet '{}': {}", sheet_name, e))?;

    Ok(range_to_string_rows(&range, None))
}

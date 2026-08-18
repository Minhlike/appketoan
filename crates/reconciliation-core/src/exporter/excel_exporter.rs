use std::path::Path;
use rust_xlsxwriter::{Format, FormatBorder, Workbook};

use crate::models::{ExportSummary, MatchStatus, ReconciliationResult};

/// Generates a structured multi-tab audit reconciliation Excel report
pub fn export_reconciliation_to_excel<P: AsRef<Path>>(
    result: &ReconciliationResult,
    output_path: P,
) -> Result<ExportSummary, String> {
    let mut workbook = Workbook::new();

    // Formats
    let title_format = Format::new()
        .set_bold()
        .set_font_size(14)
        .set_font_name("Segoe UI");

    let header_format = Format::new()
        .set_bold()
        .set_font_size(10)
        .set_font_name("Segoe UI")
        .set_border(FormatBorder::Thin);

    let cell_format = Format::new()
        .set_font_size(10)
        .set_font_name("Segoe UI")
        .set_border(FormatBorder::Thin);

    let num_format = Format::new()
        .set_font_size(10)
        .set_font_name("Segoe UI")
        .set_border(FormatBorder::Thin)
        .set_num_format("#,##0");

    let status_tag = |status: &MatchStatus| -> &'static str {
        match status {
            MatchStatus::MatchedExact => "Khớp hoàn toàn",
            MatchStatus::MatchedWithTolerance => "Khớp có dung sai",
            MatchStatus::MatchedAggregate => "Khớp gộp tổng",
            MatchStatus::MismatchAmount => "Sai lệch số tiền",
            MatchStatus::MismatchMetadata => "Sai lệch thông tin",
            MatchStatus::UnmatchedMissingInTarget => "Thiếu bên đối chiếu",
            MatchStatus::UnmatchedMissingInSource => "Thiếu bên nguồn chính",
            MatchStatus::DuplicateSuspect => "Nghi ngờ trùng lặp",
            MatchStatus::AmbiguousMatch => "Cần kiểm tra lại",
        }
    };

    // -------------------------------------------------------------
    // SHEET 1: Tong quan
    // -------------------------------------------------------------
    let ws_summary = workbook
        .add_worksheet()
        .set_name("Tong quan")
        .map_err(|e| format!("Lỗi tạo sheet Tong quan: {}", e))?;

    ws_summary
        .write_string_with_format(0, 0, "BÁO CÁO KẾT QUẢ ĐỐI CHIẾU KẾ TOÁN", &title_format)
        .map_err(|e| e.to_string())?;

    ws_summary.write_string_with_format(2, 0, "Mã phiên đối chiếu:", &header_format).map_err(|e| e.to_string())?;
    ws_summary.write_string_with_format(2, 1, &result.session_id, &cell_format).map_err(|e| e.to_string())?;

    ws_summary.write_string_with_format(3, 0, "Kịch bản đối chiếu:", &header_format).map_err(|e| e.to_string())?;
    ws_summary.write_string_with_format(3, 1, &result.profile_id, &cell_format).map_err(|e| e.to_string())?;

    ws_summary.write_string_with_format(4, 0, "Thời gian thực hiện:", &header_format).map_err(|e| e.to_string())?;
    ws_summary.write_string_with_format(4, 1, &result.executed_at, &cell_format).map_err(|e| e.to_string())?;

    let sum = &result.summary;
    let metrics = [
        ("Tổng số dòng Nguồn chính", sum.total_source_records as f64),
        ("Tổng số dòng Nguồn đối chiếu", sum.total_target_records as f64),
        ("Số nhóm Khớp hoàn toàn (100%)", sum.exact_matches_count as f64),
        ("Số nhóm Khớp có dung sai", sum.tolerance_matches_count as f64),
        ("Số nhóm Khớp gộp (1-N / N-1)", sum.aggregate_matches_count as f64),
        ("Số nhóm Sai lệch số tiền / thuế", sum.mismatches_count as f64),
        ("Số chứng từ Thiếu bên đối chiếu", sum.missing_in_target_count as f64),
        ("Số chứng từ Thiếu bên nguồn chính", sum.missing_in_source_count as f64),
        ("Số bản ghi Trùng lặp", sum.duplicates_count as f64),
        ("Số nhóm Cần kiểm tra lại", sum.ambiguous_count as f64),
        ("Tổng chênh lệch tài chính (VND)", sum.net_financial_variance),
    ];

    ws_summary.write_string_with_format(6, 0, "CHỈ TIÊU ĐỐI CHIẾU", &header_format).map_err(|e| e.to_string())?;
    ws_summary.write_string_with_format(6, 1, "GIÁ TRỊ", &header_format).map_err(|e| e.to_string())?;

    for (idx, (label, val)) in metrics.iter().enumerate() {
        let row = (7 + idx) as u32;
        ws_summary.write_string_with_format(row, 0, *label, &cell_format).map_err(|e| e.to_string())?;
        ws_summary.write_number_with_format(row, 1, *val, &num_format).map_err(|e| e.to_string())?;
    }

    ws_summary.set_column_width(0, 35).map_err(|e| e.to_string())?;
    ws_summary.set_column_width(1, 25).map_err(|e| e.to_string())?;

    // -------------------------------------------------------------
    // SHEET 2: Sai lech & Can chu y
    // -------------------------------------------------------------
    let ws_diff = workbook
        .add_worksheet()
        .set_name("Sai lech & Can chu y")
        .map_err(|e| format!("Lỗi tạo sheet Sai lech: {}", e))?;

    let diff_headers = [
        "STT",
        "Mã nhóm",
        "Trạng thái",
        "Tiền Nguồn A",
        "Tiền Nguồn B",
        "Chênh lệch (A - B)",
        "Lý do & Chi tiết sai lệch",
    ];

    for (col, h) in diff_headers.iter().enumerate() {
        ws_diff
            .write_string_with_format(0, col as u16, *h, &header_format)
            .map_err(|e| e.to_string())?;
    }

    let mut diff_row_idx = 1u32;
    for g in &result.groups {
        if g.status == MatchStatus::MatchedExact {
            continue;
        }

        let reason = if !g.discrepancies.is_empty() {
            g.discrepancies
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<&str>>()
                .join("; ")
        } else {
            status_tag(&g.status).to_string()
        };

        ws_diff.write_number_with_format(diff_row_idx, 0, diff_row_idx as f64, &cell_format).map_err(|e| e.to_string())?;
        ws_diff.write_string_with_format(diff_row_idx, 1, &g.id, &cell_format).map_err(|e| e.to_string())?;
        ws_diff.write_string_with_format(diff_row_idx, 2, status_tag(&g.status), &cell_format).map_err(|e| e.to_string())?;
        ws_diff.write_number_with_format(diff_row_idx, 3, g.total_source_amount, &num_format).map_err(|e| e.to_string())?;
        ws_diff.write_number_with_format(diff_row_idx, 4, g.total_target_amount, &num_format).map_err(|e| e.to_string())?;
        ws_diff.write_number_with_format(diff_row_idx, 5, g.amount_variance, &num_format).map_err(|e| e.to_string())?;
        ws_diff.write_string_with_format(diff_row_idx, 6, &reason, &cell_format).map_err(|e| e.to_string())?;

        diff_row_idx += 1;
    }

    ws_diff.set_column_width(0, 8).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(1, 20).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(2, 22).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(3, 18).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(4, 18).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(5, 18).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(6, 60).map_err(|e| e.to_string())?;

    // -------------------------------------------------------------
    // SHEET 3: Chi tiet tat ca
    // -------------------------------------------------------------
    let ws_all = workbook
        .add_worksheet()
        .set_name("Chi tiet tat ca")
        .map_err(|e| format!("Lỗi tạo sheet Chi tiet tat ca: {}", e))?;

    for (col, h) in diff_headers.iter().enumerate() {
        ws_all
            .write_string_with_format(0, col as u16, *h, &header_format)
            .map_err(|e| e.to_string())?;
    }

    for (idx, g) in result.groups.iter().enumerate() {
        let row = (idx + 1) as u32;
        let reason = if !g.discrepancies.is_empty() {
            g.discrepancies
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<&str>>()
                .join("; ")
        } else {
            status_tag(&g.status).to_string()
        };

        ws_all.write_number_with_format(row, 0, (idx + 1) as f64, &cell_format).map_err(|e| e.to_string())?;
        ws_all.write_string_with_format(row, 1, &g.id, &cell_format).map_err(|e| e.to_string())?;
        ws_all.write_string_with_format(row, 2, status_tag(&g.status), &cell_format).map_err(|e| e.to_string())?;
        ws_all.write_number_with_format(row, 3, g.total_source_amount, &num_format).map_err(|e| e.to_string())?;
        ws_all.write_number_with_format(row, 4, g.total_target_amount, &num_format).map_err(|e| e.to_string())?;
        ws_all.write_number_with_format(row, 5, g.amount_variance, &num_format).map_err(|e| e.to_string())?;
        ws_all.write_string_with_format(row, 6, &reason, &cell_format).map_err(|e| e.to_string())?;
    }

    ws_all.set_column_width(0, 8).map_err(|e| e.to_string())?;
    ws_all.set_column_width(1, 20).map_err(|e| e.to_string())?;
    ws_all.set_column_width(2, 22).map_err(|e| e.to_string())?;
    ws_all.set_column_width(3, 18).map_err(|e| e.to_string())?;
    ws_all.set_column_width(4, 18).map_err(|e| e.to_string())?;
    ws_all.set_column_width(5, 18).map_err(|e| e.to_string())?;
    ws_all.set_column_width(6, 60).map_err(|e| e.to_string())?;

    // Save
    let target_path = output_path.as_ref();
    workbook
        .save(target_path)
        .map_err(|e| format!("Không thể lưu file Excel báo cáo: {}", e))?;

    let file_size_bytes = std::fs::metadata(target_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(ExportSummary {
        output_path: target_path.to_string_lossy().to_string(),
        file_size_bytes,
        total_groups_exported: result.groups.len(),
        created_at: result.executed_at.clone(),
    })
}

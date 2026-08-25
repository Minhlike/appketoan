use rust_decimal::prelude::ToPrimitive;
use rust_xlsxwriter::{Format, FormatBorder, Workbook};
use std::path::Path;

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
            MatchStatus::MatchedWithMissingSource => "Thiếu nguồn thứ cấp",
            MatchStatus::MismatchAmount => "Sai lệch số tiền",
            MatchStatus::MismatchMetadata => "Sai lệch thông tin",
            MatchStatus::UnmatchedMissingInTarget => "Thiếu bên đối chiếu",
            MatchStatus::UnmatchedMissingInSource => "Thiếu bên nguồn chính",
            MatchStatus::DuplicateSuspect => "Nghi ngờ trùng lặp",
            MatchStatus::AmbiguousMatch => "Cần kiểm tra lại",
            MatchStatus::NeedsReview => "Cần kiểm tra thủ công (thiếu định danh)",
            MatchStatus::NotChecked => "Chưa đối chiếu (nguồn chưa tải)",
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

    ws_summary
        .write_string_with_format(2, 0, "Mã phiên đối chiếu:", &header_format)
        .map_err(|e| e.to_string())?;
    ws_summary
        .write_string_with_format(2, 1, &result.session_id, &cell_format)
        .map_err(|e| e.to_string())?;

    ws_summary
        .write_string_with_format(3, 0, "Kịch bản đối chiếu:", &header_format)
        .map_err(|e| e.to_string())?;
    ws_summary
        .write_string_with_format(3, 1, &result.profile_id, &cell_format)
        .map_err(|e| e.to_string())?;

    ws_summary
        .write_string_with_format(4, 0, "Thời gian thực hiện:", &header_format)
        .map_err(|e| e.to_string())?;
    ws_summary
        .write_string_with_format(4, 1, &result.executed_at, &cell_format)
        .map_err(|e| e.to_string())?;

    let sum = &result.summary;
    let rev_var_f64 = sum.revenue_variance.to_f64().unwrap_or(0.0);
    let vat_var_f64 = sum.vat_variance.to_f64().unwrap_or(0.0);
    let rec_var_f64 = sum.receivable_variance.to_f64().unwrap_or(0.0);
    let gross_var_f64 = sum.total_discrepant_amount.to_f64().unwrap_or(0.0);

    let metrics = [
        ("Tổng số dòng Nguồn chính", sum.total_source_records as f64),
        (
            "Tổng số dòng Nguồn đối chiếu",
            sum.total_target_records as f64,
        ),
        (
            "Số nhóm Khớp hoàn toàn (100%)",
            sum.exact_matches_count as f64,
        ),
        (
            "Số nhóm Khớp có dung sai",
            sum.tolerance_matches_count as f64,
        ),
        (
            "Số nhóm Khớp gộp (1-N / N-1)",
            sum.aggregate_matches_count as f64,
        ),
        (
            "Số nhóm Sai lệch số tiền / thuế",
            sum.mismatches_count as f64,
        ),
        (
            "Số chứng từ Thiếu bên đối chiếu",
            sum.missing_in_target_count as f64,
        ),
        (
            "Số chứng từ Thiếu bên nguồn chính",
            sum.missing_in_source_count as f64,
        ),
        ("Số bản ghi Trùng lặp", sum.duplicates_count as f64),
        (
            "Số nhóm Cần kiểm tra lại (Ambiguous)",
            sum.ambiguous_count as f64,
        ),
        ("Chênh lệch Doanh thu (VND)", rev_var_f64),
        ("Chênh lệch Thuế GTGT (VND)", vat_var_f64),
        ("Chênh lệch Công nợ phải thu (VND)", rec_var_f64),
        ("Tổng quy mô sai lệch tuyệt đối (VND)", gross_var_f64),
    ];

    ws_summary
        .write_string_with_format(6, 0, "CHỈ TIÊU ĐỐI CHIẾU", &header_format)
        .map_err(|e| e.to_string())?;
    ws_summary
        .write_string_with_format(6, 1, "GIÁ TRỊ", &header_format)
        .map_err(|e| e.to_string())?;

    for (idx, (label, val)) in metrics.iter().enumerate() {
        let row = (7 + idx) as u32;
        ws_summary
            .write_string_with_format(row, 0, *label, &cell_format)
            .map_err(|e| e.to_string())?;
        ws_summary
            .write_number_with_format(row, 1, *val, &num_format)
            .map_err(|e| e.to_string())?;
    }

    ws_summary
        .set_column_width(0, 35)
        .map_err(|e| e.to_string())?;
    ws_summary
        .set_column_width(1, 25)
        .map_err(|e| e.to_string())?;

    // -------------------------------------------------------------
    // SHEET 2: Sai lech & Can chu y
    // -------------------------------------------------------------
    let ws_diff = workbook
        .add_worksheet()
        .set_name("Sai lech & Can chu y")
        .map_err(|e| format!("Lỗi tạo sheet Sai lech: {}", e))?;

    let diff_headers = [
        "STT",
        "Số chứng từ",
        "Ký hiệu",
        "Ngày",
        "Loại đối chiếu",
        "Nguồn tham chiếu",
        "Nguồn đối chiếu",
        "Giá trị kỳ vọng",
        "Giá trị thực tế",
        "Chênh lệch",
        "Trạng thái",
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

        let doc_no = g.doc_no.as_deref().unwrap_or("N/A");
        let series = g.series.as_deref().unwrap_or("");
        let date = g.date.as_deref().unwrap_or("");

        if !g.semantic_comparisons.is_empty() {
            for comp in &g.semantic_comparisons {
                if comp.status == MatchStatus::MatchedExact {
                    continue;
                }

                let exp_f64 = comp.expected_amount.to_f64().unwrap_or(0.0);
                let act_f64 = comp.actual_amount.to_f64().unwrap_or(0.0);
                let var_f64 = comp.variance.to_f64().unwrap_or(0.0);

                let reason = if !comp.discrepancies.is_empty() {
                    comp.discrepancies
                        .iter()
                        .map(|d| d.message.as_str())
                        .collect::<Vec<&str>>()
                        .join("; ")
                } else {
                    status_tag(&comp.status).to_string()
                };

                ws_diff
                    .write_number_with_format(diff_row_idx, 0, diff_row_idx as f64, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(diff_row_idx, 1, doc_no, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(diff_row_idx, 2, series, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(diff_row_idx, 3, date, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(diff_row_idx, 4, &comp.semantic_name, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(
                        diff_row_idx,
                        5,
                        &comp.primary_source_name,
                        &cell_format,
                    )
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(
                        diff_row_idx,
                        6,
                        &comp.secondary_source_name,
                        &cell_format,
                    )
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_number_with_format(diff_row_idx, 7, exp_f64, &num_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_number_with_format(diff_row_idx, 8, act_f64, &num_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_number_with_format(diff_row_idx, 9, var_f64, &num_format)
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(
                        diff_row_idx,
                        10,
                        status_tag(&comp.status),
                        &cell_format,
                    )
                    .map_err(|e| e.to_string())?;
                ws_diff
                    .write_string_with_format(diff_row_idx, 11, &reason, &cell_format)
                    .map_err(|e| e.to_string())?;

                diff_row_idx += 1;
            }
        } else {
            // Group without semantic comparisons (e.g. missing group)
            let exp_f64 = g.total_source_amount.to_f64().unwrap_or(0.0);
            let act_f64 = g.total_target_amount.to_f64().unwrap_or(0.0);
            let var_f64 = g.amount_variance.to_f64().unwrap_or(0.0);

            let reason = if !g.discrepancies.is_empty() {
                g.discrepancies
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<&str>>()
                    .join("; ")
            } else {
                status_tag(&g.status).to_string()
            };

            ws_diff
                .write_number_with_format(diff_row_idx, 0, diff_row_idx as f64, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 1, doc_no, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 2, series, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 3, date, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 4, "Đối chiếu tổng thể", &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 5, "Nguồn chính", &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 6, "Nguồn đối chiếu", &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_number_with_format(diff_row_idx, 7, exp_f64, &num_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_number_with_format(diff_row_idx, 8, act_f64, &num_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_number_with_format(diff_row_idx, 9, var_f64, &num_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 10, status_tag(&g.status), &cell_format)
                .map_err(|e| e.to_string())?;
            ws_diff
                .write_string_with_format(diff_row_idx, 11, &reason, &cell_format)
                .map_err(|e| e.to_string())?;

            diff_row_idx += 1;
        }
    }

    ws_diff.set_column_width(0, 8).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(1, 16).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(2, 14).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(3, 14).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(4, 25).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(5, 20).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(6, 20).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(7, 18).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(8, 18).map_err(|e| e.to_string())?;
    ws_diff.set_column_width(9, 18).map_err(|e| e.to_string())?;
    ws_diff
        .set_column_width(10, 20)
        .map_err(|e| e.to_string())?;
    ws_diff
        .set_column_width(11, 55)
        .map_err(|e| e.to_string())?;

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

    let mut all_row_idx = 1u32;
    for g in &result.groups {
        let doc_no = g.doc_no.as_deref().unwrap_or("N/A");
        let series = g.series.as_deref().unwrap_or("");
        let date = g.date.as_deref().unwrap_or("");

        if !g.semantic_comparisons.is_empty() {
            for comp in &g.semantic_comparisons {
                let exp_f64 = comp.expected_amount.to_f64().unwrap_or(0.0);
                let act_f64 = comp.actual_amount.to_f64().unwrap_or(0.0);
                let var_f64 = comp.variance.to_f64().unwrap_or(0.0);

                let reason = if !comp.discrepancies.is_empty() {
                    comp.discrepancies
                        .iter()
                        .map(|d| d.message.as_str())
                        .collect::<Vec<&str>>()
                        .join("; ")
                } else {
                    status_tag(&comp.status).to_string()
                };

                ws_all
                    .write_number_with_format(all_row_idx, 0, all_row_idx as f64, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(all_row_idx, 1, doc_no, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(all_row_idx, 2, series, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(all_row_idx, 3, date, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(all_row_idx, 4, &comp.semantic_name, &cell_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(
                        all_row_idx,
                        5,
                        &comp.primary_source_name,
                        &cell_format,
                    )
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(
                        all_row_idx,
                        6,
                        &comp.secondary_source_name,
                        &cell_format,
                    )
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_number_with_format(all_row_idx, 7, exp_f64, &num_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_number_with_format(all_row_idx, 8, act_f64, &num_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_number_with_format(all_row_idx, 9, var_f64, &num_format)
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(
                        all_row_idx,
                        10,
                        status_tag(&comp.status),
                        &cell_format,
                    )
                    .map_err(|e| e.to_string())?;
                ws_all
                    .write_string_with_format(all_row_idx, 11, &reason, &cell_format)
                    .map_err(|e| e.to_string())?;

                all_row_idx += 1;
            }
        } else {
            let exp_f64 = g.total_source_amount.to_f64().unwrap_or(0.0);
            let act_f64 = g.total_target_amount.to_f64().unwrap_or(0.0);
            let var_f64 = g.amount_variance.to_f64().unwrap_or(0.0);

            let reason = if !g.discrepancies.is_empty() {
                g.discrepancies
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<&str>>()
                    .join("; ")
            } else {
                status_tag(&g.status).to_string()
            };

            ws_all
                .write_number_with_format(all_row_idx, 0, all_row_idx as f64, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 1, doc_no, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 2, series, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 3, date, &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 4, "Đối chiếu tổng thể", &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 5, "Nguồn chính", &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 6, "Nguồn đối chiếu", &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_number_with_format(all_row_idx, 7, exp_f64, &num_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_number_with_format(all_row_idx, 8, act_f64, &num_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_number_with_format(all_row_idx, 9, var_f64, &num_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 10, status_tag(&g.status), &cell_format)
                .map_err(|e| e.to_string())?;
            ws_all
                .write_string_with_format(all_row_idx, 11, &reason, &cell_format)
                .map_err(|e| e.to_string())?;

            all_row_idx += 1;
        }
    }

    ws_all.set_column_width(0, 8).map_err(|e| e.to_string())?;
    ws_all.set_column_width(1, 16).map_err(|e| e.to_string())?;
    ws_all.set_column_width(2, 14).map_err(|e| e.to_string())?;
    ws_all.set_column_width(3, 14).map_err(|e| e.to_string())?;
    ws_all.set_column_width(4, 25).map_err(|e| e.to_string())?;
    ws_all.set_column_width(5, 20).map_err(|e| e.to_string())?;
    ws_all.set_column_width(6, 20).map_err(|e| e.to_string())?;
    ws_all.set_column_width(7, 18).map_err(|e| e.to_string())?;
    ws_all.set_column_width(8, 18).map_err(|e| e.to_string())?;
    ws_all.set_column_width(9, 18).map_err(|e| e.to_string())?;
    ws_all.set_column_width(10, 20).map_err(|e| e.to_string())?;
    ws_all.set_column_width(11, 55).map_err(|e| e.to_string())?;

    // Save
    let target_path = output_path.as_ref();
    workbook
        .save(target_path)
        .map_err(|e| format!("Không thể lưu file Excel báo cáo: {}", e))?;

    let file_size_bytes = std::fs::metadata(target_path).map(|m| m.len()).unwrap_or(0);

    Ok(ExportSummary {
        output_path: target_path.to_string_lossy().to_string(),
        file_size_bytes,
        total_groups_exported: result.groups.len(),
        created_at: result.executed_at.clone(),
    })
}

use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;

use crate::models::{CanonicalRecord, DataSource};
use crate::reader::header_detector::remove_diacritics;

/// Normalizes Excel serial date floats (e.g. 45300.0 -> "2024-01-09")
pub fn parse_excel_date(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Try parsing as numeric Excel serial date
    if let Ok(serial) = trimmed.parse::<f64>() {
        if serial > 1000.0 && serial < 100000.0 {
            // Days since 1899-12-30 (accounting for Excel leap year bug in 1900)
            let base_date = NaiveDate::from_ymd_opt(1899, 12, 30)?;
            let days = serial.floor() as i64;
            if let Some(target_date) = base_date.checked_add_signed(chrono::Duration::days(days)) {
                return Some(format!(
                    "{:04}-{:02}-{:02}",
                    target_date.year(),
                    target_date.month(),
                    target_date.day()
                ));
            }
        }
    }

    // Standard string date formats: DD/MM/YYYY, DD-MM-YYYY, YYYY-MM-DD
    let clean = trimmed.split(' ').next().unwrap_or(trimmed);
    let parts: Vec<&str> = if clean.contains('/') {
        clean.split('/').collect()
    } else if clean.contains('-') {
        clean.split('-').collect()
    } else if clean.contains('.') {
        clean.split('.').collect()
    } else {
        vec![clean]
    };

    if parts.len() == 3 {
        // Check if YYYY-MM-DD
        if parts[0].len() == 4 {
            if let (Ok(y), Ok(m), Ok(d)) = (
                parts[0].parse::<i32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                    return Some(format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()));
                }
            }
        }
        // Check if DD/MM/YYYY
        if parts[2].len() == 4 {
            if let (Ok(d), Ok(m), Ok(y)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<i32>(),
            ) {
                if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                    return Some(format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()));
                }
            }
        }
    }

    Some(trimmed.to_string())
}

/// Sanitizes monetary text into a standard float
pub fn parse_amount(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "-" || trimmed == "N/A" || trimmed == "null" {
        return None;
    }

    let mut s = trimmed
        .replace("₫", "")
        .replace("VND", "")
        .replace("VNĐ", "")
        .replace("vnd", "")
        .replace("usd", "")
        .replace("USD", "")
        .replace("đồng", "")
        .trim()
        .to_string();

    let mut is_negative = false;
    if s.starts_with('(') && s.ends_with(')') {
        is_negative = true;
        s = s[1..s.len() - 1].trim().to_string();
    } else if s.starts_with('-') {
        is_negative = true;
        s = s[1..].trim().to_string();
    } else if s.ends_with('-') {
        is_negative = true;
        s = s[..s.len() - 1].trim().to_string();
    }

    // Determine decimal and thousand separators
    // e.g. "1.250.000,50" -> dot is thousand, comma is decimal
    // e.g. "1,250,000.50" -> comma is thousand, dot is decimal
    // e.g. "1.250.000" -> dot is thousand
    let has_comma = s.contains(',');
    let has_dot = s.contains('.');

    let clean_str = if has_comma && has_dot {
        let last_comma = s.rfind(',').unwrap();
        let last_dot = s.rfind('.').unwrap();
        if last_dot > last_comma {
            // US format: comma thousand, dot decimal
            s.replace(',', "")
        } else {
            // European/VN format: dot thousand, comma decimal
            s.replace('.', "").replace(',', ".")
        }
    } else if has_comma {
        // Check if comma is decimal or thousand
        let comma_count = s.chars().filter(|&c| c == ',').count();
        let last_comma_pos = s.rfind(',').unwrap();
        let digits_after = s.len() - 1 - last_comma_pos;
        if comma_count == 1 && digits_after != 3 {
            // Decimal comma e.g. "1250,5"
            s.replace(',', ".")
        } else {
            // Thousand separator e.g. "1,250,000"
            s.replace(',', "")
        }
    } else if has_dot {
        let dot_count = s.chars().filter(|&c| c == '.').count();
        let last_dot_pos = s.rfind('.').unwrap();
        let digits_after = s.len() - 1 - last_dot_pos;
        if dot_count == 1 && digits_after != 3 {
            // Decimal dot e.g. "1250.5"
            s
        } else {
            // Thousand separator e.g. "1.250.000"
            s.replace('.', "")
        }
    } else {
        s
    };

    let val = clean_str.parse::<f64>().ok()?;
    if is_negative {
        Some(-val)
    } else {
        Some(val)
    }
}

/// Checks if a row is a subtotal, summary, or garbage row
pub fn is_garbage_or_subtotal_row(cells: &[String]) -> bool {
    let non_empty: Vec<&str> = cells
        .iter()
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
        .collect();

    if non_empty.is_empty() {
        return true;
    }

    // Check first 3 non-empty cells for subtotal / footer keywords
    for cell in non_empty.iter().take(3) {
        let norm = remove_diacritics(cell).to_lowercase();
        if norm.starts_with("cong")
            || norm.starts_with("tong cong")
            || norm.starts_with("tong so")
            || norm.starts_with("total")
            || norm.starts_with("grand total")
            || norm.starts_with("nguoi lap")
            || norm.starts_with("ke toan truong")
            || norm.starts_with("giam doc")
            || norm.starts_with("thu truong")
            || norm.starts_with("ngay ... thang")
            || norm.starts_with("so du dau ky")
            || norm.starts_with("so du cuoi ky")
        {
            return true;
        }
    }

    false
}

/// Normalizes an array of raw sheet rows into a vector of CanonicalRecords
pub fn normalize_data_source_rows(
    data_source: &DataSource,
    header_columns: &[String],
    raw_rows: &[Vec<String>],
) -> Vec<CanonicalRecord> {
    let mapping = &data_source.column_mapping;
    let data_start_idx = if data_source.data_start_row > 0 {
        (data_source.data_start_row - 1) as usize
    } else {
        1
    };

    let col_index = |target_opt: &Option<String>| -> Option<usize> {
        let target = target_opt.as_ref()?;
        header_columns.iter().position(|col| col.trim() == target.trim())
    };

    let idx_doc_no = col_index(&mapping.doc_no_column);
    let idx_series = col_index(&mapping.series_column);
    let idx_template_code = col_index(&mapping.template_code_column);
    let idx_date = col_index(&mapping.date_column);
    let idx_partner_tax_id = col_index(&mapping.partner_tax_id_column);
    let idx_partner_name = col_index(&mapping.partner_name_column);
    let idx_pretax_amount = col_index(&mapping.pretax_amount_column);
    let idx_vat_amount = col_index(&mapping.vat_amount_column);
    let idx_total_amount = col_index(&mapping.total_amount_column);
    let idx_vat_rate = col_index(&mapping.vat_rate_column);
    let idx_debit_account = col_index(&mapping.debit_account_column);
    let idx_credit_account = col_index(&mapping.credit_account_column);
    let idx_voucher_no = col_index(&mapping.voucher_no_column);
    let idx_description = col_index(&mapping.description_column);
    let idx_bank_account = col_index(&mapping.bank_account_column);

    let mut canonical_records = Vec::new();

    for (row_offset, row) in raw_rows.iter().skip(data_start_idx).enumerate() {
        let source_row = (data_start_idx + row_offset + 1) as u32;

        if is_garbage_or_subtotal_row(row) {
            continue;
        }

        let get_val = |opt_idx: Option<usize>| -> Option<String> {
            let i = opt_idx?;
            let s = row.get(i)?.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        };

        let raw_doc_no = get_val(idx_doc_no);
        let doc_no = raw_doc_no.as_deref().map(CanonicalRecord::normalize_doc_no);

        let series = get_val(idx_series).map(|s| s.trim().to_uppercase());
        let template_code = get_val(idx_template_code);
        let date = get_val(idx_date).and_then(|d| parse_excel_date(&d));

        let partner_tax_id =
            get_val(idx_partner_tax_id).map(|t| CanonicalRecord::normalize_tax_id(&t));
        let partner_name = get_val(idx_partner_name);

        let pretax_amount = get_val(idx_pretax_amount).and_then(|a| parse_amount(&a));
        let vat_amount = get_val(idx_vat_amount).and_then(|a| parse_amount(&a));
        let total_amount = get_val(idx_total_amount)
            .and_then(|a| parse_amount(&a))
            .unwrap_or(0.0);

        let vat_rate = get_val(idx_vat_rate);
        let debit_account = get_val(idx_debit_account);
        let credit_account = get_val(idx_credit_account);
        let voucher_no = get_val(idx_voucher_no);
        let description = get_val(idx_description);
        let bank_account = get_val(idx_bank_account);

        let mut raw_fields = HashMap::new();
        for (i, col_name) in header_columns.iter().enumerate() {
            if let Some(val) = row.get(i) {
                if !val.trim().is_empty() {
                    raw_fields.insert(col_name.clone(), val.clone());
                }
            }
        }

        let mut record = CanonicalRecord {
            id: format!("{}_row_{}", data_source.id, source_row),
            source_id: data_source.id.clone(),
            source_row,
            date,
            doc_no,
            series,
            template_code,
            partner_tax_id,
            partner_name,
            pretax_amount,
            vat_amount,
            total_amount,
            vat_rate,
            debit_account,
            credit_account,
            voucher_no,
            description,
            bank_account,
            raw_fields,
        };

        record.reconcile_monetary_invariants();

        // If doc_no, partner_name, and total_amount are completely empty, skip row
        if record.doc_no.is_none()
            && record.partner_name.is_none()
            && record.total_amount == 0.0
            && record.voucher_no.is_none()
        {
            continue;
        }

        canonical_records.push(record);
    }

    canonical_records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_amount_variations() {
        assert_eq!(parse_amount("10,000,000"), Some(10_000_000.0));
        assert_eq!(parse_amount("10.000.000"), Some(10_000_000.0));
        assert_eq!(parse_amount("1.250.000,50 ₫"), Some(1_250_000.50));
        assert_eq!(parse_amount("(500.000 VND)"), Some(-500_000.0));
        assert_eq!(parse_amount("-150.000"), Some(-150_000.0));
        assert_eq!(parse_amount("150.000-"), Some(-150_000.0));
        assert_eq!(parse_amount("-"), None);
    }

    #[test]
    fn test_parse_excel_dates() {
        assert_eq!(parse_excel_date("45300"), Some("2024-01-09".to_string()));
        assert_eq!(parse_excel_date("15/01/2026"), Some("2026-01-15".to_string()));
        assert_eq!(parse_excel_date("2026-01-15"), Some("2026-01-15".to_string()));
        assert_eq!(parse_excel_date("15-01-2026 14:30:00"), Some("2026-01-15".to_string()));
    }

    #[test]
    fn test_garbage_row_detection() {
        assert!(is_garbage_or_subtotal_row(&[
            "Tổng cộng".to_string(),
            "".to_string(),
            "50,000,000".to_string()
        ]));
        assert!(is_garbage_or_subtotal_row(&[
            "Cộng".to_string(),
            "".to_string(),
            "10,000,000".to_string()
        ]));
        assert!(is_garbage_or_subtotal_row(&[
            "Người lập biểu".to_string(),
            "".to_string()
        ]));
        assert!(!is_garbage_or_subtotal_row(&[
            "1".to_string(),
            "00000101".to_string(),
            "10,000,000".to_string()
        ]));
    }
}

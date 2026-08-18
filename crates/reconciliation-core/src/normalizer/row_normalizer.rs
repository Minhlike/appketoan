use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

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

/// Sanitizes monetary text into a standard Decimal
pub fn parse_amount(input: &str) -> Option<Decimal> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "-" || trimmed == "N/A" || trimmed == "null" || trimmed == "nil" {
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
        .replace([' ', '\u{a0}'], "")
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

    if s.is_empty() {
        return None;
    }

    // Determine decimal and thousand separators
    let has_comma = s.contains(',');
    let has_dot = s.contains('.');

    let clean_str = if has_comma && has_dot {
        let last_comma = s.rfind(',').unwrap();
        let last_dot = s.rfind('.').unwrap();
        if last_dot > last_comma {
            // US format: 1,250,000.50 -> comma thousand, dot decimal
            s.replace(',', "")
        } else {
            // European/VN format: 1.250.000,50 -> dot thousand, comma decimal
            s.replace('.', "").replace(',', ".")
        }
    } else if has_comma {
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

    let mut val = Decimal::from_str(&clean_str).ok()?;
    if is_negative {
        val.set_sign_negative(true);
    }
    Some(val)
}

/// Checks if a cell is an exact summary / subtotal title keyword
fn is_summary_keyword(cell: &str) -> bool {
    let norm = remove_diacritics(cell)
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_lowercase();

    // Critical Safeguard: If the cell contains "cong ty" (company), it is a customer/vendor name, NEVER a summary row!
    if norm.contains("cong ty") || norm.contains("doanh nghiep") || norm.contains("chi nhanh") || norm.contains("hop tac xa") {
        return false;
    }

    // Exact summary tokens
    matches!(
        norm.as_str(),
        "cong"
            | "tong cong"
            | "tong"
            | "tong so"
            | "total"
            | "grand total"
            | "subtotal"
            | "so du dau ky"
            | "phat sinh trong ky"
            | "so phat sinh trong ky"
            | "so du cuoi ky"
            | "cong phat sinh"
            | "tong phat sinh"
            | "cong thang"
            | "cong quy"
            | "cong nam"
            | "luy ke"
            | "so luy ke"
            | "nguoi lap"
            | "nguoi lap bieu"
            | "ke toan truong"
            | "giam doc"
            | "thu truong"
            | "don vi bao cao"
            | "ky tinh thue"
            | "bang ke hoa don"
            | "so chi tiet"
            | "so nhat ky"
    ) || norm.starts_with("ngay    thang")
        || norm.starts_with("ngay ... thang")
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

    // If any non-empty cell matches an exact summary keyword, treat as garbage/subtotal row
    for cell in &non_empty {
        if is_summary_keyword(cell) {
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
    let idx_doc_code = col_index(&mapping.doc_code_column);
    let idx_series = col_index(&mapping.series_column);
    let idx_template_code = col_index(&mapping.template_code_column);
    let idx_date = col_index(&mapping.date_column);
    let idx_partner_tax_id = col_index(&mapping.partner_tax_id_column);
    let idx_buyer_tax_id = col_index(&mapping.buyer_tax_id_column);
    let idx_seller_tax_id = col_index(&mapping.seller_tax_id_column);
    let idx_partner_name = col_index(&mapping.partner_name_column);
    let idx_pretax_amount = col_index(&mapping.pretax_amount_column);
    let idx_vat_amount = col_index(&mapping.vat_amount_column);
    let idx_discount_amount = col_index(&mapping.discount_amount_column);
    let idx_total_amount = col_index(&mapping.total_amount_column);
    let idx_debit_amount = col_index(&mapping.debit_amount_column);
    let idx_credit_amount = col_index(&mapping.credit_amount_column);
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
        let doc_code = get_val(idx_doc_code);

        let series = get_val(idx_series).map(|s| s.trim().to_uppercase());
        let template_code = get_val(idx_template_code);
        let date = get_val(idx_date).and_then(|d| parse_excel_date(&d));

        let buyer_tax_id =
            get_val(idx_buyer_tax_id).map(|t| CanonicalRecord::normalize_tax_id(&t));
        let seller_tax_id =
            get_val(idx_seller_tax_id).map(|t| CanonicalRecord::normalize_tax_id(&t));

        let partner_tax_id = get_val(idx_partner_tax_id)
            .map(|t| CanonicalRecord::normalize_tax_id(&t))
            .or_else(|| buyer_tax_id.clone());

        let partner_name = get_val(idx_partner_name);

        let pretax_amount = get_val(idx_pretax_amount).and_then(|a| parse_amount(&a));
        let vat_amount = get_val(idx_vat_amount).and_then(|a| parse_amount(&a));
        let discount_amount = get_val(idx_discount_amount).and_then(|a| parse_amount(&a));
        let debit_amount = get_val(idx_debit_amount).and_then(|a| parse_amount(&a));
        let credit_amount = get_val(idx_credit_amount).and_then(|a| parse_amount(&a));

        // Read total_amount directly from total_amount_column if mapped
        let total_amount = if let Some(tot) = get_val(idx_total_amount).and_then(|a| parse_amount(&a)) {
            tot
        } else if let (Some(pretax), Some(vat)) = (pretax_amount, vat_amount) {
            // Derived value when total amount column is not present in workbook
            pretax + vat
        } else if let Some(pretax) = pretax_amount {
            pretax
        } else if let Some(credit) = credit_amount {
            credit
        } else if let Some(debit) = debit_amount {
            debit
        } else {
            Decimal::ZERO
        };

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

        let record = CanonicalRecord {
            id: format!("{}_row_{}", data_source.id, source_row),
            source_id: data_source.id.clone(),
            source_row,
            date,
            doc_no,
            doc_code,
            series,
            template_code,
            partner_tax_id,
            buyer_tax_id,
            seller_tax_id,
            partner_name,
            pretax_amount,
            vat_amount,
            discount_amount,
            total_amount,
            debit_amount,
            credit_amount,
            vat_rate,
            debit_account,
            credit_account,
            voucher_no,
            description,
            bank_account,
            raw_fields,
        };

        // STRICT FILTERING:
        // 1. If row has no monetary values at all (empty/zero amount across pretax, vat, debit, credit, total), skip it!
        if record.has_no_monetary_value() {
            continue;
        }

        // 2. If row has no identifier (no doc_no, no voucher_no, no partner_name), skip it
        if record.doc_no.is_none() && record.voucher_no.is_none() && record.partner_name.is_none() {
            continue;
        }

        canonical_records.push(record);
    }

    canonical_records
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_amount_variations() {
        assert_eq!(parse_amount("10,000,000"), Some(dec!(10000000)));
        assert_eq!(parse_amount("10.000.000"), Some(dec!(10000000)));
        assert_eq!(parse_amount("1.250.000,50 ₫"), Some(dec!(1250000.50)));
        assert_eq!(parse_amount("(500.000 VND)"), Some(dec!(-500000)));
        assert_eq!(parse_amount("-150.000"), Some(dec!(-150000)));
        assert_eq!(parse_amount("150.000-"), Some(dec!(-150000)));
        assert_eq!(parse_amount("105000000.00"), Some(dec!(105000000.00)));
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
    fn test_company_names_never_filtered_as_subtotal() {
        // MUST NOT be treated as summary / garbage
        assert!(!is_garbage_or_subtotal_row(&[
            "1".to_string(),
            "00000101".to_string(),
            "CÔNG TY TNHH ABC".to_string(),
            "10,000,000".to_string()
        ]));
        assert!(!is_garbage_or_subtotal_row(&[
            "2".to_string(),
            "00000102".to_string(),
            "Công ty cổ phần XYZ".to_string(),
            "20,000,000".to_string()
        ]));
        assert!(!is_garbage_or_subtotal_row(&[
            "3".to_string(),
            "00000103".to_string(),
            "Doanh nghiệp tư nhân Hưng Thịnh".to_string(),
            "5,000,000".to_string()
        ]));

        // MUST be treated as summary / garbage
        assert!(is_garbage_or_subtotal_row(&[
            "CỘNG".to_string(),
            "".to_string(),
            "50,000,000".to_string()
        ]));
        assert!(is_garbage_or_subtotal_row(&[
            "TỔNG CỘNG".to_string(),
            "".to_string(),
            "50,000,000".to_string()
        ]));
        assert!(is_garbage_or_subtotal_row(&[
            "SỐ DƯ ĐẦU KỲ".to_string(),
            "".to_string(),
            "100,000,000".to_string()
        ]));
        assert!(is_garbage_or_subtotal_row(&[
            "".to_string(),
            "Phát sinh trong kỳ".to_string(),
            "7,223,121,057".to_string()
        ]));
        assert!(is_garbage_or_subtotal_row(&[
            "SỐ DƯ CUỐI KỲ".to_string(),
            "".to_string(),
            "0".to_string()
        ]));
    }
}

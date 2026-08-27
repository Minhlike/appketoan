use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

use crate::models::{
    AnalyticalRowLevel, CanonicalRecord, DataSource, PartnerRecord, SalesAnalysisRecord,
    ValueOrigin,
};
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
                    return Some(format!(
                        "{:04}-{:02}-{:02}",
                        date.year(),
                        date.month(),
                        date.day()
                    ));
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
                    return Some(format!(
                        "{:04}-{:02}-{:02}",
                        date.year(),
                        date.month(),
                        date.day()
                    ));
                }
            }
        }
    }

    Some(trimmed.to_string())
}

/// Sanitizes monetary text into a standard Decimal
pub fn parse_amount(input: &str) -> Option<Decimal> {
    let trimmed = input.trim();
    if trimmed.is_empty()
        || trimmed == "-"
        || trimmed == "N/A"
        || trimmed == "null"
        || trimmed == "nil"
    {
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

    let has_comma = s.contains(',');
    let has_dot = s.contains('.');

    let clean_str = if has_comma && has_dot {
        let last_comma = s.rfind(',').unwrap();
        let last_dot = s.rfind('.').unwrap();
        if last_dot > last_comma {
            s.replace(',', "")
        } else {
            s.replace('.', "").replace(',', ".")
        }
    } else if has_comma {
        let comma_count = s.chars().filter(|&c| c == ',').count();
        let last_comma_pos = s.rfind(',').unwrap();
        let digits_after = s.len() - 1 - last_comma_pos;
        if comma_count == 1 && digits_after != 3 {
            s.replace(',', ".")
        } else {
            s.replace(',', "")
        }
    } else if has_dot {
        let dot_count = s.chars().filter(|&c| c == '.').count();
        let last_dot_pos = s.rfind('.').unwrap();
        let digits_after = s.len() - 1 - last_dot_pos;
        if dot_count == 1 && digits_after != 3 {
            s
        } else {
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

    // Safeguard: company / partner names are never summary rows
    if norm.contains("cong ty")
        || norm.contains("doanh nghiep")
        || norm.contains("chi nhanh")
        || norm.contains("hop tac xa")
        || norm.contains("tong cong ty")
    {
        return false;
    }

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
        header_columns
            .iter()
            .position(|col| col.trim() == target.trim())
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
    let idx_partner_code = col_index(&mapping.partner_code_column);
    let idx_pretax_amount = col_index(&mapping.pretax_amount_column);
    let idx_vat_amount = col_index(&mapping.vat_amount_column);
    let idx_discount_amount = col_index(&mapping.discount_amount_column);
    let idx_fee_amount = col_index(&mapping.fee_amount_column);
    let idx_total_amount = col_index(&mapping.total_amount_column);
    let idx_debit_amount = col_index(&mapping.debit_amount_column);
    let idx_credit_amount = col_index(&mapping.credit_amount_column);
    let idx_vat_rate = col_index(&mapping.vat_rate_column);
    let idx_debit_account = col_index(&mapping.debit_account_column);
    let idx_credit_account = col_index(&mapping.credit_account_column);
    let idx_voucher_no = col_index(&mapping.voucher_no_column);
    let idx_description = col_index(&mapping.description_column);
    let idx_bank_account = col_index(&mapping.bank_account_column);
    let idx_transaction_number = col_index(&mapping.transaction_number_column);
    let idx_accounting_date = col_index(&mapping.accounting_date_column);
    let idx_transaction_date = col_index(&mapping.transaction_date_column);
    let idx_counterparty_account = col_index(&mapping.counterparty_account_column);
    let idx_counterparty_name = col_index(&mapping.counterparty_name_column);
    let idx_balance = col_index(&mapping.balance_column);

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
        let raw_date = get_val(idx_date);
        let date = raw_date.as_deref().and_then(parse_excel_date);

        let buyer_tax_id = get_val(idx_buyer_tax_id).map(|t| CanonicalRecord::normalize_tax_id(&t));
        let seller_tax_id =
            get_val(idx_seller_tax_id).map(|t| CanonicalRecord::normalize_tax_id(&t));

        let partner_tax_id = get_val(idx_partner_tax_id)
            .map(|t| CanonicalRecord::normalize_tax_id(&t))
            .or_else(|| buyer_tax_id.clone());

        let partner_name = get_val(idx_partner_name);
        let partner_code = get_val(idx_partner_code);

        let raw_pretax_amount = get_val(idx_pretax_amount);
        let raw_vat_amount = get_val(idx_vat_amount);
        let raw_total_amount = get_val(idx_total_amount);
        let pretax_amount = raw_pretax_amount.as_deref().and_then(parse_amount);
        let vat_amount = raw_vat_amount.as_deref().and_then(parse_amount);
        let discount_amount = get_val(idx_discount_amount).and_then(|a| parse_amount(&a));
        let fee_amount = get_val(idx_fee_amount).and_then(|a| parse_amount(&a));
        let debit_amount = get_val(idx_debit_amount).and_then(|a| parse_amount(&a));
        let credit_amount = get_val(idx_credit_amount).and_then(|a| parse_amount(&a));

        // Read total_amount directly from total_amount_column if mapped
        let (total_amount, total_amount_origin) =
            if let Some(tot) = raw_total_amount.as_deref().and_then(parse_amount) {
                (tot, ValueOrigin::Source)
            } else if let (Some(pretax), Some(vat)) = (pretax_amount, vat_amount) {
                let disc = discount_amount.unwrap_or(Decimal::ZERO);
                let fee = fee_amount.unwrap_or(Decimal::ZERO);
                (pretax + vat - disc + fee, ValueOrigin::Derived)
            } else if let Some(pretax) = pretax_amount {
                (pretax, ValueOrigin::Derived)
            } else if let Some(credit) = credit_amount {
                (credit, ValueOrigin::Derived)
            } else if let Some(debit) = debit_amount {
                (debit, ValueOrigin::Derived)
            } else {
                (Decimal::ZERO, ValueOrigin::Derived)
            };

        let vat_rate = get_val(idx_vat_rate);
        let debit_account = get_val(idx_debit_account);
        let credit_account = get_val(idx_credit_account);
        let voucher_no = get_val(idx_voucher_no);
        let description = get_val(idx_description);
        let bank_account = get_val(idx_bank_account);
        let transaction_number = get_val(idx_transaction_number);
        let accounting_date =
            get_val(idx_accounting_date).and_then(|value| parse_excel_date(&value));
        let transaction_date =
            get_val(idx_transaction_date).and_then(|value| parse_excel_date(&value));
        let counterparty_account = get_val(idx_counterparty_account);
        let counterparty_name = get_val(idx_counterparty_name);
        let balance = get_val(idx_balance).and_then(|value| parse_amount(&value));

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
            partner_code,
            pretax_amount,
            vat_amount,
            discount_amount,
            fee_amount,
            total_amount,
            total_amount_origin,
            debit_amount,
            credit_amount,
            vat_rate,
            debit_account,
            credit_account,
            voucher_no,
            description,
            bank_account,
            transaction_number,
            accounting_date,
            transaction_date,
            counterparty_account,
            counterparty_name,
            balance,
            raw_fields,
        };

        // V18 document-integrity validation must retain malformed BK rows so
        // INVALID_AMOUNT / INVALID_DATE / MISSING_INVOICE_NUMBER can preserve
        // their original row provenance. Other transactional kinds retain the
        // historical placeholder-row filter.
        let preserve_sales_register_validation_row = data_source.kind
            == crate::models::DataSourceKind::SalesRegister
            && (raw_doc_no.is_some()
                || raw_date.is_some()
                || raw_pretax_amount.is_some()
                || raw_vat_amount.is_some()
                || raw_total_amount.is_some());

        if record.has_no_monetary_value() && !preserve_sales_register_validation_row {
            continue;
        }

        if record.doc_no.is_none()
            && record.voucher_no.is_none()
            && record.partner_name.is_none()
            && !preserve_sales_register_validation_row
        {
            continue;
        }

        canonical_records.push(record);
    }

    canonical_records
}

fn mapped_value(
    row: &[String],
    header_columns: &[String],
    column: &Option<String>,
) -> Option<String> {
    let target = column.as_ref()?.trim();
    let index = header_columns
        .iter()
        .position(|header| header.trim() == target)?;
    row.get(index)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parse_marker(value: Option<String>) -> Option<bool> {
    let value = value?;
    match remove_diacritics(&value)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "1" | "x" | "yes" | "true" | "co" => Some(true),
        "0" | "no" | "false" | "khong" => Some(false),
        _ => None,
    }
}

/// Normalizes master data without applying transactional monetary filters.
pub fn normalize_partner_master_rows(
    data_source: &DataSource,
    header_columns: &[String],
    raw_rows: &[Vec<String>],
) -> Vec<PartnerRecord> {
    let start = data_source.data_start_row.saturating_sub(1) as usize;
    raw_rows
        .iter()
        .enumerate()
        .skip(start)
        .filter(|(_, row)| !is_garbage_or_subtotal_row(row))
        .filter_map(|(index, row)| {
            let partner_code = mapped_value(
                row,
                header_columns,
                &data_source.column_mapping.partner_code_column,
            );
            let partner_name = mapped_value(
                row,
                header_columns,
                &data_source.column_mapping.partner_name_column,
            );
            let partner_tax_id = mapped_value(
                row,
                header_columns,
                &data_source.column_mapping.partner_tax_id_column,
            )
            .map(|value| CanonicalRecord::normalize_tax_id(&value))
            .filter(|value| !value.is_empty());
            if partner_code.is_none() && partner_name.is_none() && partner_tax_id.is_none() {
                return None;
            }
            let raw_fields = header_columns
                .iter()
                .enumerate()
                .filter_map(|(column_index, header)| {
                    row.get(column_index)
                        .map(|value| (header.clone(), value.clone()))
                        .filter(|(_, value)| !value.trim().is_empty())
                })
                .collect();
            Some(PartnerRecord {
                id: format!("{}_row_{}", data_source.id, index + 1),
                source_id: data_source.id.clone(),
                source_row: (index + 1) as u32,
                partner_code,
                partner_name,
                partner_tax_id,
                address: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.address_column,
                ),
                is_customer: parse_marker(mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.is_customer_column,
                )),
                is_supplier: parse_marker(mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.is_supplier_column,
                )),
                status: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.status_column,
                ),
                raw_fields,
            })
        })
        .collect()
}

/// Normalizes the grouped sales-analysis report without treating it as transactional data.
pub fn normalize_sales_analysis_rows(
    data_source: &DataSource,
    header_columns: &[String],
    raw_rows: &[Vec<String>],
) -> Vec<SalesAnalysisRecord> {
    let start = data_source.data_start_row.saturating_sub(1) as usize;
    raw_rows
        .iter()
        .enumerate()
        .skip(start)
        .filter(|(_, row)| !is_garbage_or_subtotal_row(row))
        .filter_map(|(index, row)| {
            let product_code = mapped_value(
                row,
                header_columns,
                &data_source.column_mapping.product_code_column,
            );
            let product_name = mapped_value(
                row,
                header_columns,
                &data_source.column_mapping.product_name_column,
            );
            let revenue = mapped_value(
                row,
                header_columns,
                &data_source.column_mapping.revenue_column,
            )
            .and_then(|value| parse_amount(&value));
            if product_code.is_none() && product_name.is_none() && revenue.is_none() {
                return None;
            }
            let row_level = if product_code.is_some() {
                AnalyticalRowLevel::Detail
            } else if product_name.is_some() {
                AnalyticalRowLevel::Group
            } else {
                AnalyticalRowLevel::NeedsReview
            };
            let raw_fields = header_columns
                .iter()
                .enumerate()
                .filter_map(|(column_index, header)| {
                    row.get(column_index)
                        .map(|value| (header.clone(), value.clone()))
                        .filter(|(_, value)| !value.trim().is_empty())
                })
                .collect();
            Some(SalesAnalysisRecord {
                id: format!("{}_row_{}", data_source.id, index + 1),
                source_id: data_source.id.clone(),
                source_row: (index + 1) as u32,
                row_level,
                group_key: None,
                product_code,
                product_name,
                quantity: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.quantity_column,
                )
                .and_then(|value| parse_amount(&value)),
                unit_price: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.unit_price_column,
                )
                .and_then(|value| parse_amount(&value)),
                revenue,
                vat: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.vat_amount_column,
                )
                .and_then(|value| parse_amount(&value)),
                discount: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.discount_amount_column,
                )
                .and_then(|value| parse_amount(&value)),
                receivable: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.total_amount_column,
                )
                .and_then(|value| parse_amount(&value)),
                unit_cost: None,
                cost: mapped_value(row, header_columns, &data_source.column_mapping.cost_column)
                    .and_then(|value| parse_amount(&value)),
                profit: mapped_value(
                    row,
                    header_columns,
                    &data_source.column_mapping.profit_column,
                )
                .and_then(|value| parse_amount(&value)),
                raw_fields,
            })
        })
        .collect()
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
        assert_eq!(
            parse_excel_date("15/01/2026"),
            Some("2026-01-15".to_string())
        );
        assert_eq!(
            parse_excel_date("2026-01-15"),
            Some("2026-01-15".to_string())
        );
        assert_eq!(
            parse_excel_date("15-01-2026 14:30:00"),
            Some("2026-01-15".to_string())
        );
    }

    #[test]
    fn test_company_names_never_filtered_as_subtotal() {
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

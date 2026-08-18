use chrono::NaiveDate;
use rust_decimal::Decimal;
use crate::models::{CanonicalRecord, FieldDiscrepancy};

/// Formats a Decimal VND amount to Vietnamese thousand separated string (e.g. 10,000,000)
pub fn format_vnd(amount: Decimal) -> String {
    let is_neg = amount.is_sign_negative();
    let abs_val = amount.abs();
    let int_part = abs_val.trunc();
    let s = int_part.to_string();
    let mut res = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            res.push(',');
        }
        res.push(c);
    }

    let fract = abs_val.fract();
    if !fract.is_zero() {
        let fract_str = fract.to_string();
        if let Some(dot_idx) = fract_str.find('.') {
            res.push('.');
            res.push_str(&fract_str[dot_idx + 1..]);
        }
    }

    if is_neg {
        format!("-{}", res)
    } else {
        res
    }
}

/// Computes days between two ISO date strings (YYYY-MM-DD)
pub fn days_between(date_a_opt: &Option<String>, date_b_opt: &Option<String>) -> Option<i64> {
    let a_str = date_a_opt.as_ref()?;
    let b_str = date_b_opt.as_ref()?;
    let a = NaiveDate::parse_from_str(a_str, "%Y-%m-%d").ok()?;
    let b = NaiveDate::parse_from_str(b_str, "%Y-%m-%d").ok()?;
    Some((a - b).num_days().abs())
}

/// Analyzes differences between a primary record and a target record
pub fn analyze_pair_discrepancies(
    primary: &CanonicalRecord,
    target: &CanonicalRecord,
    primary_comp_amount: Decimal,
    target_comp_amount: Decimal,
    amount_tolerance_vnd: Decimal,
    date_tolerance_days: u32,
) -> Vec<FieldDiscrepancy> {
    let mut discrepancies = Vec::new();

    // 1. Compared Amount Check (e.g. Invoice Pretax vs TK511 Credit)
    let amount_diff = (primary_comp_amount - target_comp_amount).abs();
    if amount_diff > Decimal::ZERO {
        let is_within_tolerance = amount_diff <= amount_tolerance_vnd;
        let msg = if is_within_tolerance {
            format!(
                "Chênh lệch làm tròn số tiền {} đ (trong ngưỡng dung sai <= {} đ)",
                format_vnd(amount_diff),
                format_vnd(amount_tolerance_vnd)
            )
        } else {
            format!(
                "Lệch số tiền đối chiếu: Nguồn A ({}) đ vs Nguồn B ({}) đ, chênh lệch {} đ",
                format_vnd(primary_comp_amount),
                format_vnd(target_comp_amount),
                format_vnd(amount_diff)
            )
        };

        discrepancies.push(FieldDiscrepancy {
            field_name: "amount".to_string(),
            source_value: Some(format_vnd(primary_comp_amount)),
            target_value: Some(format_vnd(target_comp_amount)),
            amount_diff: Some(amount_diff),
            message: msg,
        });
    }

    // 2. Partner Tax ID Check
    if let (Some(tax_a), Some(tax_b)) = (&primary.partner_tax_id, &target.partner_tax_id) {
        if tax_a != tax_b && !tax_a.is_empty() && !tax_b.is_empty() {
            discrepancies.push(FieldDiscrepancy {
                field_name: "partnerTaxId".to_string(),
                source_value: Some(tax_a.clone()),
                target_value: Some(tax_b.clone()),
                amount_diff: None,
                message: format!("Khác mã số thuế: Nguồn A [{}] vs Nguồn B [{}]", tax_a, tax_b),
            });
        }
    }

    // 3. Date Difference Check
    if let Some(days) = days_between(&primary.date, &target.date) {
        if days > date_tolerance_days as i64 {
            discrepancies.push(FieldDiscrepancy {
                field_name: "date".to_string(),
                source_value: primary.date.clone(),
                target_value: target.date.clone(),
                amount_diff: None,
                message: format!(
                    "Lệch ngày lập/chứng từ: {} ngày (vượt ngưỡng cho phép {} ngày)",
                    days, date_tolerance_days
                ),
            });
        }
    }

    discrepancies
}

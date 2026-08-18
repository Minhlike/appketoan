use chrono::NaiveDate;
use crate::models::{CanonicalRecord, FieldDiscrepancy};

/// Formats a float VND amount to Vietnamese thousand separated string (e.g. 10,000,000)
pub fn format_vnd(amount: f64) -> String {
    let is_neg = amount < 0.0;
    let abs_val = amount.abs().round() as i64;
    let s = abs_val.to_string();
    let mut res = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            res.push(',');
        }
        res.push(c);
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
    amount_tolerance_vnd: f64,
    date_tolerance_days: u32,
) -> Vec<FieldDiscrepancy> {
    let mut discrepancies = Vec::new();

    // 1. Total Amount Check
    let total_diff = (primary.total_amount - target.total_amount).abs();
    if total_diff > 0.0 {
        let is_within_tolerance = total_diff <= amount_tolerance_vnd;
        let msg = if is_within_tolerance {
            format!(
                "Chênh lệch làm tròn số tiền {} đ (trong ngưỡng dung sai <= {} đ)",
                format_vnd(total_diff),
                format_vnd(amount_tolerance_vnd)
            )
        } else {
            format!(
                "Lệch tổng tiền: Nguồn A ({}) đ vs Nguồn B ({}) đ, chênh lệch {} đ",
                format_vnd(primary.total_amount),
                format_vnd(target.total_amount),
                format_vnd(total_diff)
            )
        };

        discrepancies.push(FieldDiscrepancy {
            field_name: "totalAmount".to_string(),
            source_value: Some(format_vnd(primary.total_amount)),
            target_value: Some(format_vnd(target.total_amount)),
            amount_diff: Some(total_diff),
            message: msg,
        });
    }

    // 2. VAT Amount Check
    if let (Some(vat_a), Some(vat_b)) = (primary.vat_amount, target.vat_amount) {
        let vat_diff = (vat_a - vat_b).abs();
        if vat_diff > amount_tolerance_vnd {
            discrepancies.push(FieldDiscrepancy {
                field_name: "vatAmount".to_string(),
                source_value: Some(format_vnd(vat_a)),
                target_value: Some(format_vnd(vat_b)),
                amount_diff: Some(vat_diff),
                message: format!(
                    "Lệch tiền thuế GTGT: Nguồn A là {} đ, Nguồn B là {} đ (lệch {} đ)",
                    format_vnd(vat_a),
                    format_vnd(vat_b),
                    format_vnd(vat_diff)
                ),
            });
        }
    }

    // 3. Pretax Amount Check
    if let (Some(pretax_a), Some(pretax_b)) = (primary.pretax_amount, target.pretax_amount) {
        let pretax_diff = (pretax_a - pretax_b).abs();
        if pretax_diff > amount_tolerance_vnd {
            discrepancies.push(FieldDiscrepancy {
                field_name: "pretaxAmount".to_string(),
                source_value: Some(format_vnd(pretax_a)),
                target_value: Some(format_vnd(pretax_b)),
                amount_diff: Some(pretax_diff),
                message: format!(
                    "Lệch doanh thu chưa thuế: Nguồn A là {} đ, Nguồn B là {} đ",
                    format_vnd(pretax_a),
                    format_vnd(pretax_b)
                ),
            });
        }
    }

    // 4. Partner Tax ID Check
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

    // 5. Date Difference Check
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

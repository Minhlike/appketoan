use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalRecord {
    pub id: String,
    pub source_id: String,
    pub source_row: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_no: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_tax_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretax_amount: Option<Decimal>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_amount: Option<Decimal>,

    pub total_amount: Decimal,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debit_amount: Option<Decimal>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_amount: Option<Decimal>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_rate: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debit_account: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_account: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub voucher_no: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account: Option<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub raw_fields: HashMap<String, String>,
}

impl CanonicalRecord {
    /// Normalizes document number (e.g. "00000123" -> "123", " 123.0 " -> "123")
    pub fn normalize_doc_no(input: &str) -> String {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let clean = if let Some(dot_pos) = trimmed.find('.') {
            if trimmed[dot_pos + 1..].chars().all(|c| c == '0') {
                &trimmed[..dot_pos]
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        if clean.chars().all(|c| c.is_ascii_digit()) {
            let stripped = clean.trim_start_matches('0');
            if stripped.is_empty() {
                "0".to_string()
            } else {
                stripped.to_string()
            }
        } else {
            clean.to_string()
        }
    }

    /// Normalizes Vietnamese tax code (strips whitespace and non-standard symbols)
    pub fn normalize_tax_id(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '-')
            .collect::<String>()
            .trim()
            .to_uppercase()
    }

    /// Automatically infers totalAmount, pretaxAmount, or vatAmount if one is missing
    pub fn reconcile_monetary_invariants(&mut self) {
        if self.total_amount.is_zero() {
            if let (Some(pretax), Some(vat)) = (self.pretax_amount, self.vat_amount) {
                self.total_amount = pretax + vat;
            } else if let Some(pretax) = self.pretax_amount {
                self.total_amount = pretax;
            } else if let Some(credit) = self.credit_amount {
                self.total_amount = credit;
            } else if let Some(debit) = self.debit_amount {
                self.total_amount = debit;
            }
        } else if self.pretax_amount.is_none() && self.vat_amount.is_none() {
            if self.credit_amount.is_none() && self.debit_amount.is_none() {
                self.pretax_amount = Some(self.total_amount);
                self.vat_amount = Some(Decimal::ZERO);
            }
        }
    }

    /// Checks if all monetary fields in this record are none or zero
    pub fn has_no_monetary_value(&self) -> bool {
        let is_none_or_zero = |opt: Option<Decimal>| opt.map_or(true, |d| d.is_zero());
        self.total_amount.is_zero()
            && is_none_or_zero(self.pretax_amount)
            && is_none_or_zero(self.vat_amount)
            && is_none_or_zero(self.debit_amount)
            && is_none_or_zero(self.credit_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_doc_no_normalization() {
        assert_eq!(CanonicalRecord::normalize_doc_no("00000123"), "123");
        assert_eq!(CanonicalRecord::normalize_doc_no("123.0"), "123");
        assert_eq!(CanonicalRecord::normalize_doc_no("  000456  "), "456");
        assert_eq!(CanonicalRecord::normalize_doc_no("00000000"), "0");
        assert_eq!(CanonicalRecord::normalize_doc_no("INV-2024-001"), "INV-2024-001");
    }

    #[test]
    fn test_tax_id_normalization() {
        assert_eq!(CanonicalRecord::normalize_tax_id(" 0101234567 "), "0101234567");
        assert_eq!(CanonicalRecord::normalize_tax_id("0101234567-001"), "0101234567-001");
        assert_eq!(CanonicalRecord::normalize_tax_id("MST: 0309998888"), "0309998888");
    }

    #[test]
    fn test_monetary_invariants() {
        let mut record = CanonicalRecord {
            id: "rec_1".to_string(),
            source_id: "src_1".to_string(),
            source_row: 5,
            date: Some("2026-01-15".to_string()),
            doc_no: Some("123".to_string()),
            series: Some("1C26TAA".to_string()),
            template_code: None,
            partner_tax_id: Some("0101234567".to_string()),
            partner_name: Some("Công ty TNHH Thử Nghiệm".to_string()),
            pretax_amount: Some(dec!(10000000)),
            vat_amount: Some(dec!(1000000)),
            total_amount: Decimal::ZERO,
            debit_amount: None,
            credit_amount: None,
            vat_rate: Some("10%".to_string()),
            debit_account: Some("131".to_string()),
            credit_account: Some("5111".to_string()),
            voucher_no: Some("PKT-001".to_string()),
            description: Some("Doanh thu bán hàng".to_string()),
            bank_account: None,
            raw_fields: HashMap::new(),
        };

        record.reconcile_monetary_invariants();
        assert_eq!(record.total_amount, dec!(11000000));
    }
}

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::matcher::engine::extract_rule_amount;
use crate::models::{directions_are_compatible, CanonicalRecord, ComparisonRule, DataSourceKind};

/// Explicit outcome for the bank-statement candidate policy.  It is purposely
/// deterministic: there is no score, fuzzy match, or implicit preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BankCandidateDecision {
    Accepted {
        record_id: String,
        evidence: &'static str,
    },
    NeedsReview {
        reason: &'static str,
        record_ids: Vec<String>,
    },
    Suggested {
        record_id: String,
        reason: &'static str,
    },
}

fn normalized_text(value: Option<&str>) -> String {
    value
        .unwrap_or_default()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn same_day_or_within(primary: Option<&str>, secondary: Option<&str>, days: u32) -> bool {
    match (primary, secondary) {
        (Some(left), Some(right)) => match (
            NaiveDate::parse_from_str(left, "%Y-%m-%d"),
            NaiveDate::parse_from_str(right, "%Y-%m-%d"),
        ) {
            (Ok(left), Ok(right)) => (left - right).num_days().abs() <= i64::from(days),
            _ => false,
        },
        _ => false,
    }
}

/// Selects a TK112/bank candidate using an auditable policy:
/// same signed direction, exact rule amount (or explicit tolerance), date
/// window, then a unique deterministic reference/counterparty evidence path.
/// A statement is not required to contain an invoice number or tax id.
pub fn select_bank_candidate(
    primary: &CanonicalRecord,
    primary_kind: &DataSourceKind,
    candidates: &[&CanonicalRecord],
    secondary_kind: &DataSourceKind,
    rule: &ComparisonRule,
    amount_tolerance: Decimal,
    date_tolerance_days: u32,
) -> BankCandidateDecision {
    let Ok(primary_amount) = extract_rule_amount(primary, &rule.primary_field) else {
        return BankCandidateDecision::NeedsReview {
            reason: "PRIMARY_AMOUNT_MISSING",
            record_ids: vec![],
        };
    };

    let compatible: Vec<&CanonicalRecord> = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            directions_are_compatible(primary, primary_kind, candidate, secondary_kind)
                && same_day_or_within(
                    primary
                        .accounting_date
                        .as_deref()
                        .or(primary.transaction_date.as_deref())
                        .or(primary.date.as_deref()),
                    candidate
                        .accounting_date
                        .as_deref()
                        .or(candidate.transaction_date.as_deref())
                        .or(candidate.date.as_deref()),
                    date_tolerance_days,
                )
                && extract_rule_amount(candidate, &rule.secondary_field)
                    .is_ok_and(|amount| (amount - primary_amount).abs() <= amount_tolerance)
        })
        .collect();

    if compatible.is_empty() {
        let has_direction_conflict = candidates.iter().copied().any(|candidate| {
            !directions_are_compatible(primary, primary_kind, candidate, secondary_kind)
                && same_day_or_within(
                    primary
                        .accounting_date
                        .as_deref()
                        .or(primary.transaction_date.as_deref())
                        .or(primary.date.as_deref()),
                    candidate
                        .accounting_date
                        .as_deref()
                        .or(candidate.transaction_date.as_deref())
                        .or(candidate.date.as_deref()),
                    date_tolerance_days,
                )
                && extract_rule_amount(candidate, &rule.secondary_field)
                    .is_ok_and(|amount| (amount - primary_amount).abs() <= amount_tolerance)
        });
        return BankCandidateDecision::NeedsReview {
            reason: if has_direction_conflict {
                "DIRECTION_CONFLICT"
            } else {
                "INSUFFICIENT_BANK_EVIDENCE"
            },
            record_ids: vec![],
        };
    }
    let primary_reference = normalized_text(primary.transaction_number.as_deref())
        .or_else(|| normalized_text(primary.voucher_no.as_deref()))
        .or_else(|| normalized_text(primary.doc_no.as_deref()));
    if !primary_reference.is_empty() {
        let reference_matches: Vec<&CanonicalRecord> = compatible
            .iter()
            .copied()
            .filter(|candidate| {
                primary_reference == normalized_text(candidate.transaction_number.as_deref())
                    || primary_reference == normalized_text(candidate.voucher_no.as_deref())
                    || primary_reference == normalized_text(candidate.doc_no.as_deref())
            })
            .collect();
        if reference_matches.len() == 1 {
            return BankCandidateDecision::Accepted {
                record_id: reference_matches[0].id.clone(),
                evidence: "UNIQUE_REFERENCE",
            };
        }
    }

    let primary_counterparty = normalized_text(primary.counterparty_account.as_deref())
        .or_else(|| normalized_text(primary.counterparty_name.as_deref()))
        .or_else(|| normalized_text(primary.partner_name.as_deref()));
    if !primary_counterparty.is_empty() {
        let partner_matches: Vec<&CanonicalRecord> = compatible
            .iter()
            .copied()
            .filter(|candidate| {
                primary_counterparty == normalized_text(candidate.counterparty_account.as_deref())
                    || primary_counterparty
                        == normalized_text(candidate.counterparty_name.as_deref())
                    || primary_counterparty == normalized_text(candidate.partner_name.as_deref())
            })
            .collect();
        if partner_matches.len() == 1 {
            return BankCandidateDecision::Accepted {
                record_id: partner_matches[0].id.clone(),
                evidence: "UNIQUE_COUNTERPARTY",
            };
        }
    }

    let primary_description = normalized_text(primary.description.as_deref());
    if !primary_description.is_empty() {
        let description_matches: Vec<&CanonicalRecord> = compatible
            .iter()
            .copied()
            .filter(|candidate| {
                primary_description == normalized_text(candidate.description.as_deref())
            })
            .collect();
        if description_matches.len() == 1 {
            return BankCandidateDecision::Accepted {
                record_id: description_matches[0].id.clone(),
                evidence: "UNIQUE_EXACT_DESCRIPTION",
            };
        }
    }

    if compatible.len() == 1 {
        return BankCandidateDecision::Suggested {
            record_id: compatible[0].id.clone(),
            reason: "SUGGESTED_DIRECTION_AMOUNT_DATE",
        };
    }

    BankCandidateDecision::NeedsReview {
        reason: "AMBIGUOUS_BANK_CANDIDATES",
        record_ids: compatible
            .into_iter()
            .map(|candidate| candidate.id.clone())
            .collect(),
    }
}

trait NonEmptyString {
    fn or_else(self, fallback: impl FnOnce() -> String) -> String;
}

impl NonEmptyString for String {
    fn or_else(self, fallback: impl FnOnce() -> String) -> String {
        if self.is_empty() {
            fallback()
        } else {
            self
        }
    }
}

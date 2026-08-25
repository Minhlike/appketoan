use rust_decimal::Decimal;

use crate::models::{CanonicalRecord, DataSourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyDirection {
    MoneyIn,
    MoneyOut,
    Unknown,
}

/// Derives direction from the accounting/bank debit-credit convention. No
/// amount-only fallback is permitted when both sides are populated.
pub fn record_money_direction(record: &CanonicalRecord, kind: &DataSourceKind) -> MoneyDirection {
    let debit = record.debit_amount.unwrap_or(Decimal::ZERO);
    let credit = record.credit_amount.unwrap_or(Decimal::ZERO);
    if !debit.is_zero() && !credit.is_zero() {
        return MoneyDirection::Unknown;
    }
    match kind {
        DataSourceKind::Ledger112 => {
            if !debit.is_zero() {
                MoneyDirection::MoneyIn
            } else if !credit.is_zero() {
                MoneyDirection::MoneyOut
            } else {
                MoneyDirection::Unknown
            }
        }
        DataSourceKind::BankStatement => {
            if !credit.is_zero() {
                MoneyDirection::MoneyIn
            } else if !debit.is_zero() {
                MoneyDirection::MoneyOut
            } else {
                MoneyDirection::Unknown
            }
        }
        _ => MoneyDirection::Unknown,
    }
}

pub fn directional_amount(record: &CanonicalRecord) -> Option<Decimal> {
    let debit = record.debit_amount.unwrap_or(Decimal::ZERO);
    let credit = record.credit_amount.unwrap_or(Decimal::ZERO);
    if !debit.is_zero() && credit.is_zero() {
        Some(debit)
    } else if !credit.is_zero() && debit.is_zero() {
        Some(credit)
    } else {
        None
    }
}

pub fn directions_are_compatible(
    primary: &CanonicalRecord,
    primary_kind: &DataSourceKind,
    secondary: &CanonicalRecord,
    secondary_kind: &DataSourceKind,
) -> bool {
    if !matches!(
        primary_kind,
        DataSourceKind::Ledger112 | DataSourceKind::BankStatement
    ) || !matches!(
        secondary_kind,
        DataSourceKind::Ledger112 | DataSourceKind::BankStatement
    ) {
        return true;
    }
    let primary_direction = record_money_direction(primary, primary_kind);
    let secondary_direction = record_money_direction(secondary, secondary_kind);
    primary_direction != MoneyDirection::Unknown && primary_direction == secondary_direction
}

/// Detects a two-sided transfer between distinct internal bank subaccounts.
/// It is a classification aid, never a revenue/expense match.
pub fn is_internal_bank_transfer(left: &CanonicalRecord, right: &CanonicalRecord) -> bool {
    let left_amount = directional_amount(left);
    let right_amount = directional_amount(right);
    let distinct_accounts = matches!((&left.bank_account, &right.bank_account), (Some(a), Some(b)) if !a.trim().is_empty() && !b.trim().is_empty() && a != b);
    distinct_accounts
        && left_amount.is_some()
        && left_amount == right_amount
        && ((left.debit_amount.unwrap_or(Decimal::ZERO).is_zero())
            != (right.debit_amount.unwrap_or(Decimal::ZERO).is_zero()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn record(debit: Option<Decimal>, credit: Option<Decimal>) -> CanonicalRecord {
        CanonicalRecord {
            debit_amount: debit,
            credit_amount: credit,
            ..Default::default()
        }
    }

    #[test]
    fn bank_direction_never_matches_opposite_sides_with_equal_amounts() {
        let ledger_in = record(Some(dec!(100)), None);
        let bank_out = record(Some(dec!(100)), None);
        assert!(!directions_are_compatible(
            &ledger_in,
            &DataSourceKind::Ledger112,
            &bank_out,
            &DataSourceKind::BankStatement
        ));
        let bank_in = record(None, Some(dec!(100)));
        assert!(directions_are_compatible(
            &ledger_in,
            &DataSourceKind::Ledger112,
            &bank_in,
            &DataSourceKind::BankStatement
        ));
    }
}

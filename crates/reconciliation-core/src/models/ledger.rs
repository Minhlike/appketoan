use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::{CanonicalRecord, DataSourceKind};

/// Generic accounting-ledger representation. Legacy source kinds remain
/// ingestion adapters; control rules should migrate to `account` over time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub source_record_id: String,
    pub account: Option<String>,
    pub counter_account: Option<String>,
    pub debit: Option<Decimal>,
    pub credit: Option<Decimal>,
    pub partner: Option<String>,
    pub document: Option<String>,
    pub posting_date: Option<String>,
    pub description: Option<String>,
}

fn legacy_account_hint(kind: &DataSourceKind) -> Option<&'static str> {
    match kind {
        DataSourceKind::Ledger511 => Some("511"),
        DataSourceKind::Ledger112 => Some("112"),
        DataSourceKind::Ledger131 => Some("131"),
        DataSourceKind::Ledger133 => Some("133"),
        DataSourceKind::Ledger3331 => Some("3331"),
        _ => None,
    }
}

pub fn ledger_entry_from_record(record: &CanonicalRecord, kind: &DataSourceKind) -> LedgerEntry {
    LedgerEntry {
        source_record_id: record.id.clone(),
        account: record
            .debit_account
            .clone()
            .or_else(|| record.credit_account.clone())
            .or_else(|| legacy_account_hint(kind).map(str::to_string)),
        counter_account: record
            .credit_account
            .clone()
            .or_else(|| record.debit_account.clone()),
        debit: record.debit_amount,
        credit: record.credit_amount,
        partner: record.partner_name.clone(),
        document: record.doc_no.clone().or_else(|| record.voucher_no.clone()),
        posting_date: record.date.clone(),
        description: record.description.clone(),
    }
}

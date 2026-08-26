use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{CanonicalRecord, DataSource, ValueOrigin};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentErrorCode {
    DuplicateInvoiceNumber,
    MissingInBk,
    ExtraInBk,
    DateMismatch,
    InvoiceNumberMismatch,
    PretaxMismatch,
    VatMismatch,
    TotalMismatch,
    AmbiguousMatch,
    InvalidDate,
    MissingInvoiceNumber,
    InvalidAmount,
    MissingInTk511,
    ExtraInTk511,
}

impl DocumentErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DuplicateInvoiceNumber => "DUPLICATE_INVOICE_NUMBER",
            Self::MissingInBk => "MISSING_IN_BK",
            Self::ExtraInBk => "EXTRA_IN_BK",
            Self::DateMismatch => "DATE_MISMATCH",
            Self::InvoiceNumberMismatch => "INVOICE_NUMBER_MISMATCH",
            Self::PretaxMismatch => "PRETAX_MISMATCH",
            Self::VatMismatch => "VAT_MISMATCH",
            Self::TotalMismatch => "TOTAL_MISMATCH",
            Self::AmbiguousMatch => "AMBIGUOUS_MATCH",
            Self::InvalidDate => "INVALID_DATE",
            Self::MissingInvoiceNumber => "MISSING_INVOICE_NUMBER",
            Self::InvalidAmount => "INVALID_AMOUNT",
            Self::MissingInTk511 => "MISSING_IN_TK511",
            Self::ExtraInTk511 => "EXTRA_IN_TK511",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentCaseStatus {
    FullyMatched,
    NeedsReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldCheckStatus {
    Match,
    Mismatch,
    NotChecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentField {
    Date,
    InvoiceNumber,
    Pretax,
    Vat,
    Total,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentComparisonScope {
    InvoiceToSalesRegister,
    InvoiceToLedger511,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordProvenance {
    pub source_id: String,
    pub source_name: String,
    pub file_path: String,
    pub sheet_name: String,
    pub record_id: String,
    pub source_row: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSnapshot {
    pub provenance: RecordProvenance,
    pub date: Option<String>,
    pub invoice_number: Option<String>,
    pub partner_identity: Option<String>,
    pub pretax_amount: Option<Decimal>,
    pub vat_amount: Option<Decimal>,
    pub total_amount: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFieldCheck {
    pub scope: DocumentComparisonScope,
    pub field: DocumentField,
    pub status: FieldCheckStatus,
    pub expected_value: Option<String>,
    pub actual_value: Option<String>,
    pub error_code: Option<DocumentErrorCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIntegrityError {
    pub code: DocumentErrorCode,
    pub severity: String,
    pub message: String,
    pub provenance: Vec<RecordProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIntegrityCase {
    pub id: String,
    pub status: DocumentCaseStatus,
    pub invoice: Option<DocumentSnapshot>,
    pub sales_register: Option<DocumentSnapshot>,
    pub ledger_511: Option<DocumentSnapshot>,
    pub field_checks: Vec<DocumentFieldCheck>,
    pub errors: Vec<DocumentIntegrityError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIntegritySummary {
    pub fully_matched: usize,
    pub date_mismatch: usize,
    pub invoice_number_mismatch: usize,
    pub pretax_mismatch: usize,
    pub vat_mismatch: usize,
    pub total_mismatch: usize,
    pub missing_in_bk: usize,
    pub extra_in_bk: usize,
    pub duplicate_invoice_number: usize,
    pub ambiguous_match: usize,
    pub invalid_date: usize,
    pub missing_invoice_number: usize,
    pub invalid_amount: usize,
    pub missing_in_tk511: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TotalsCheckStatus {
    Equal,
    Mismatch,
    NotVerified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTotalCheck {
    pub field: DocumentField,
    pub invoice_total: Decimal,
    pub sales_register_total: Decimal,
    pub variance: Decimal,
    pub status: TotalsCheckStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIntegrityResult {
    pub summary: DocumentIntegritySummary,
    pub totals: Vec<DocumentTotalCheck>,
    pub totals_equal: bool,
    pub documents_pass: bool,
    pub documents: Vec<DocumentIntegrityCase>,
}

pub struct DocumentIntegritySource<'a> {
    pub source: &'a DataSource,
    pub records: &'a [CanonicalRecord],
}

fn provenance(source: &DataSource, record: &CanonicalRecord) -> RecordProvenance {
    RecordProvenance {
        source_id: source.id.clone(),
        source_name: source.name.clone(),
        file_path: source.file_path.clone(),
        sheet_name: source.sheet_name.clone(),
        record_id: record.id.clone(),
        source_row: record.source_row,
    }
}

fn strict_date(record: &CanonicalRecord) -> Option<&str> {
    let value = record.date.as_deref()?;
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .map(|_| value)
}

fn document_number(record: &CanonicalRecord) -> Option<String> {
    record
        .doc_no
        .as_deref()
        .map(CanonicalRecord::normalize_doc_no)
        .filter(|value| !value.is_empty())
}

fn normalized_partner_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn partner_identity(record: &CanonicalRecord) -> Option<String> {
    record
        .partner_tax_id
        .as_deref()
        .map(CanonicalRecord::normalize_tax_id)
        .filter(|value| !value.is_empty())
        .map(|value| format!("tax:{value}"))
        .or_else(|| {
            record
                .partner_code
                .as_deref()
                .map(normalized_partner_text)
                .filter(|value| !value.is_empty())
                .map(|value| format!("code:{value}"))
        })
        .or_else(|| {
            record
                .partner_name
                .as_deref()
                .map(normalized_partner_text)
                .filter(|value| !value.is_empty())
                .map(|value| format!("name:{value}"))
        })
}

fn snapshot(
    source: &DataSource,
    record: &CanonicalRecord,
    total_is_valid: bool,
) -> DocumentSnapshot {
    DocumentSnapshot {
        provenance: provenance(source, record),
        date: strict_date(record).map(str::to_string),
        invoice_number: document_number(record),
        partner_identity: partner_identity(record),
        pretax_amount: record.pretax_amount,
        vat_amount: record.vat_amount,
        total_amount: total_is_valid.then_some(record.total_amount),
    }
}

fn ledger_snapshot(source: &DataSource, record: &CanonicalRecord) -> DocumentSnapshot {
    DocumentSnapshot {
        provenance: provenance(source, record),
        date: strict_date(record).map(str::to_string),
        invoice_number: document_number(record),
        partner_identity: partner_identity(record),
        pretax_amount: record.credit_amount,
        vat_amount: None,
        total_amount: None,
    }
}

fn total_is_source_value(record: &CanonicalRecord) -> bool {
    record.total_amount_origin == ValueOrigin::Source
}

fn error(
    code: DocumentErrorCode,
    severity: &str,
    message: impl Into<String>,
    records: Vec<RecordProvenance>,
) -> DocumentIntegrityError {
    DocumentIntegrityError {
        code,
        severity: severity.to_string(),
        message: message.into(),
        provenance: records,
    }
}

fn validate_document_record(
    source: &DataSource,
    record: &CanonicalRecord,
    require_vat_and_total: bool,
) -> Vec<DocumentIntegrityError> {
    let mut errors = Vec::new();
    let record_provenance = || vec![provenance(source, record)];
    if document_number(record).is_none() {
        errors.push(error(
            DocumentErrorCode::MissingInvoiceNumber,
            "HIGH",
            "MISSING_INVOICE_NUMBER: Số chứng từ bắt buộc có.",
            record_provenance(),
        ));
    }
    if strict_date(record).is_none() {
        errors.push(error(
            DocumentErrorCode::InvalidDate,
            "HIGH",
            "INVALID_DATE: Ngày chứng từ thiếu hoặc không hợp lệ.",
            record_provenance(),
        ));
    }
    if record.pretax_amount.is_none() {
        errors.push(error(
            DocumentErrorCode::InvalidAmount,
            "HIGH",
            "INVALID_AMOUNT: Tiền chưa thuế không đọc được đúng kiểu tiền.",
            record_provenance(),
        ));
    }
    if require_vat_and_total && record.vat_amount.is_none() {
        errors.push(error(
            DocumentErrorCode::InvalidAmount,
            "HIGH",
            "INVALID_AMOUNT: Tiền thuế không đọc được đúng kiểu tiền.",
            record_provenance(),
        ));
    }
    if require_vat_and_total && !total_is_source_value(record) {
        errors.push(error(
            DocumentErrorCode::InvalidAmount,
            "HIGH",
            "INVALID_AMOUNT: Phải thu không đọc được trực tiếp từ trường tiền của chứng từ.",
            record_provenance(),
        ));
    }
    errors
}

fn validate_ledger_record(
    source: &DataSource,
    record: &CanonicalRecord,
) -> Vec<DocumentIntegrityError> {
    let mut errors = Vec::new();
    let record_provenance = || vec![provenance(source, record)];
    if document_number(record).is_none() {
        errors.push(error(
            DocumentErrorCode::MissingInvoiceNumber,
            "HIGH",
            "MISSING_INVOICE_NUMBER: Số chứng từ TK511 bắt buộc có.",
            record_provenance(),
        ));
    }
    if strict_date(record).is_none() {
        errors.push(error(
            DocumentErrorCode::InvalidDate,
            "HIGH",
            "INVALID_DATE: Ngày chứng từ TK511 thiếu hoặc không hợp lệ.",
            record_provenance(),
        ));
    }
    if record.credit_amount.is_none() {
        errors.push(error(
            DocumentErrorCode::InvalidAmount,
            "HIGH",
            "INVALID_AMOUNT: Phát sinh Có TK511 không đọc được đúng kiểu tiền.",
            record_provenance(),
        ));
    }
    errors
}

fn amount_text(value: Option<Decimal>) -> Option<String> {
    value.map(|amount| amount.normalize().to_string())
}

fn text_check(
    scope: DocumentComparisonScope,
    field: DocumentField,
    expected: Option<&str>,
    actual: Option<&str>,
    mismatch_code: DocumentErrorCode,
) -> DocumentFieldCheck {
    let status = if expected.is_some() && expected == actual {
        FieldCheckStatus::Match
    } else {
        FieldCheckStatus::Mismatch
    };
    DocumentFieldCheck {
        scope,
        field,
        status,
        expected_value: expected.map(str::to_string),
        actual_value: actual.map(str::to_string),
        error_code: (status == FieldCheckStatus::Mismatch).then_some(mismatch_code),
    }
}

fn amount_check(
    scope: DocumentComparisonScope,
    field: DocumentField,
    expected: Option<Decimal>,
    actual: Option<Decimal>,
    mismatch_code: DocumentErrorCode,
) -> DocumentFieldCheck {
    let status = if expected.is_some() && expected == actual {
        FieldCheckStatus::Match
    } else {
        FieldCheckStatus::Mismatch
    };
    DocumentFieldCheck {
        scope,
        field,
        status,
        expected_value: amount_text(expected),
        actual_value: amount_text(actual),
        error_code: (status == FieldCheckStatus::Mismatch).then_some(mismatch_code),
    }
}

fn unchecked(
    scope: DocumentComparisonScope,
    field: DocumentField,
    expected: Option<Decimal>,
) -> DocumentFieldCheck {
    DocumentFieldCheck {
        scope,
        field,
        status: FieldCheckStatus::NotChecked,
        expected_value: amount_text(expected),
        actual_value: None,
        error_code: None,
    }
}

fn append_mismatch_errors(
    checks: &[DocumentFieldCheck],
    invoice: &RecordProvenance,
    counterpart: &RecordProvenance,
    errors: &mut Vec<DocumentIntegrityError>,
) {
    for check in checks {
        let Some(code) = check.error_code else {
            continue;
        };
        errors.push(error(
            code,
            "HIGH",
            format!(
                "{}: trường {:?} không khớp giữa hai chứng từ.",
                code.as_str(),
                check.field
            ),
            vec![invoice.clone(), counterpart.clone()],
        ));
    }
}

fn monetary_signature(record: &CanonicalRecord) -> Option<(Decimal, Decimal, Decimal)> {
    Some((
        record.pretax_amount?,
        record.vat_amount?,
        total_is_source_value(record).then_some(record.total_amount)?,
    ))
}

fn document_case(
    id: String,
    invoice: Option<DocumentSnapshot>,
    sales_register: Option<DocumentSnapshot>,
    ledger_511: Option<DocumentSnapshot>,
    field_checks: Vec<DocumentFieldCheck>,
    errors: Vec<DocumentIntegrityError>,
) -> DocumentIntegrityCase {
    DocumentIntegrityCase {
        id,
        status: if errors.is_empty()
            && field_checks
                .iter()
                .all(|check| check.status != FieldCheckStatus::Mismatch)
        {
            DocumentCaseStatus::FullyMatched
        } else {
            DocumentCaseStatus::NeedsReview
        },
        invoice,
        sales_register,
        ledger_511,
        field_checks,
        errors,
    }
}

fn not_checked_ledger_fields(invoice: &CanonicalRecord) -> [DocumentFieldCheck; 2] {
    [
        unchecked(
            DocumentComparisonScope::InvoiceToLedger511,
            DocumentField::Vat,
            invoice.vat_amount,
        ),
        unchecked(
            DocumentComparisonScope::InvoiceToLedger511,
            DocumentField::Total,
            total_is_source_value(invoice).then_some(invoice.total_amount),
        ),
    ]
}

pub fn evaluate_document_integrity(
    invoice_source: DocumentIntegritySource<'_>,
    register_source: DocumentIntegritySource<'_>,
    ledger_source: DocumentIntegritySource<'_>,
) -> DocumentIntegrityResult {
    let mut documents = Vec::new();
    let mut register_by_document: HashMap<String, Vec<usize>> = HashMap::new();
    let mut ledger_by_document: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, record) in register_source.records.iter().enumerate() {
        if let Some(document) = document_number(record) {
            register_by_document
                .entry(document)
                .or_default()
                .push(index);
        }
    }
    for (index, record) in ledger_source.records.iter().enumerate() {
        if let Some(document) = document_number(record) {
            ledger_by_document.entry(document).or_default().push(index);
        }
    }

    let duplicate_register_indices: HashSet<usize> = register_by_document
        .values()
        .filter(|indices| indices.len() > 1)
        .flatten()
        .copied()
        .collect();
    let mut register_validation: Vec<Vec<DocumentIntegrityError>> = register_source
        .records
        .iter()
        .map(|record| validate_document_record(register_source.source, record, true))
        .collect();
    let ledger_validation: Vec<Vec<DocumentIntegrityError>> = ledger_source
        .records
        .iter()
        .map(|record| validate_ledger_record(ledger_source.source, record))
        .collect();
    for indices in register_by_document
        .values()
        .filter(|indices| indices.len() > 1)
    {
        let related = indices
            .iter()
            .map(|index| provenance(register_source.source, &register_source.records[*index]))
            .collect::<Vec<_>>();
        for index in indices {
            register_validation[*index].push(error(
                DocumentErrorCode::DuplicateInvoiceNumber,
                "HIGH",
                "DUPLICATE_INVOICE_NUMBER: Số ct xuất hiện nhiều hơn một lần trong BK; mọi dòng trùng bị loại khỏi auto-match.",
                related.clone(),
            ));
        }
    }

    // Every invalid/duplicate BK row receives its own provenance-preserving case.
    for (index, errors) in register_validation.iter().enumerate() {
        if errors.is_empty() {
            continue;
        }
        let record = &register_source.records[index];
        documents.push(document_case(
            format!("register-validation:{}", record.id),
            None,
            Some(snapshot(
                register_source.source,
                record,
                total_is_source_value(record),
            )),
            None,
            Vec::new(),
            errors.clone(),
        ));
    }

    let mut used_register = HashSet::new();
    let mut review_linked_register = HashSet::new();
    let mut used_ledger = HashSet::new();

    // Strong diagnostic indexes. The date index allows number-only diagnostics;
    // the partner index is required when both number and date differ.
    let mut diagnostic_by_date_money: HashMap<(String, String, String, String), Vec<usize>> =
        HashMap::new();
    let mut diagnostic_by_partner_money: HashMap<(String, String, String, String), Vec<usize>> =
        HashMap::new();
    for (index, record) in register_source.records.iter().enumerate() {
        if duplicate_register_indices.contains(&index) || !register_validation[index].is_empty() {
            continue;
        }
        let Some((pretax, vat, total)) = monetary_signature(record) else {
            continue;
        };
        if let Some(date) = strict_date(record) {
            diagnostic_by_date_money
                .entry((
                    date.to_string(),
                    pretax.normalize().to_string(),
                    vat.normalize().to_string(),
                    total.normalize().to_string(),
                ))
                .or_default()
                .push(index);
        }
        if let Some(partner) = partner_identity(record) {
            diagnostic_by_partner_money
                .entry((
                    partner,
                    pretax.normalize().to_string(),
                    vat.normalize().to_string(),
                    total.normalize().to_string(),
                ))
                .or_default()
                .push(index);
        }
    }

    for invoice in invoice_source.records {
        let invoice_provenance = provenance(invoice_source.source, invoice);
        let invoice_snapshot = snapshot(
            invoice_source.source,
            invoice,
            total_is_source_value(invoice),
        );
        let mut errors = validate_document_record(invoice_source.source, invoice, true);
        let mut checks = Vec::new();
        let mut linked_register = None;
        let mut linked_ledger = None;
        let invoice_document = document_number(invoice);

        let exact_register_candidates = invoice_document
            .as_ref()
            .and_then(|document| register_by_document.get(document))
            .cloned()
            .unwrap_or_default();
        if exact_register_candidates.len() > 1 {
            let related = exact_register_candidates
                .iter()
                .map(|index| provenance(register_source.source, &register_source.records[*index]))
                .collect::<Vec<_>>();
            review_linked_register.extend(exact_register_candidates.iter().copied());
            errors.push(error(
                DocumentErrorCode::AmbiguousMatch,
                "HIGH",
                "AMBIGUOUS_MATCH: nhiều dòng BK có cùng Số ct; không chọn, không cộng và không PASS.",
                related,
            ));
        } else if let Some(index) = exact_register_candidates.first().copied() {
            review_linked_register.insert(index);
            if register_validation[index].is_empty() {
                used_register.insert(index);
            }
            let register = &register_source.records[index];
            let register_provenance = provenance(register_source.source, register);
            linked_register = Some(snapshot(
                register_source.source,
                register,
                total_is_source_value(register),
            ));
            errors.extend(register_validation[index].clone());
            let register_checks = [
                text_check(
                    DocumentComparisonScope::InvoiceToSalesRegister,
                    DocumentField::Date,
                    strict_date(invoice),
                    strict_date(register),
                    DocumentErrorCode::DateMismatch,
                ),
                text_check(
                    DocumentComparisonScope::InvoiceToSalesRegister,
                    DocumentField::InvoiceNumber,
                    invoice_document.as_deref(),
                    document_number(register).as_deref(),
                    DocumentErrorCode::InvoiceNumberMismatch,
                ),
                amount_check(
                    DocumentComparisonScope::InvoiceToSalesRegister,
                    DocumentField::Pretax,
                    invoice.pretax_amount,
                    register.pretax_amount,
                    DocumentErrorCode::PretaxMismatch,
                ),
                amount_check(
                    DocumentComparisonScope::InvoiceToSalesRegister,
                    DocumentField::Vat,
                    invoice.vat_amount,
                    register.vat_amount,
                    DocumentErrorCode::VatMismatch,
                ),
                amount_check(
                    DocumentComparisonScope::InvoiceToSalesRegister,
                    DocumentField::Total,
                    total_is_source_value(invoice).then_some(invoice.total_amount),
                    total_is_source_value(register).then_some(register.total_amount),
                    DocumentErrorCode::TotalMismatch,
                ),
            ];
            append_mismatch_errors(
                &register_checks,
                &invoice_provenance,
                &register_provenance,
                &mut errors,
            );
            checks.extend(register_checks);
        } else {
            let diagnostic_candidates = monetary_signature(invoice)
                .map(|(pretax, vat, total)| {
                    let money = (
                        pretax.normalize().to_string(),
                        vat.normalize().to_string(),
                        total.normalize().to_string(),
                    );
                    let mut candidates = BTreeSet::new();
                    if let Some(date) = strict_date(invoice) {
                        if let Some(indices) = diagnostic_by_date_money.get(&(
                            date.to_string(),
                            money.0.clone(),
                            money.1.clone(),
                            money.2.clone(),
                        )) {
                            candidates.extend(indices.iter().copied());
                        }
                    }
                    if let Some(partner) = partner_identity(invoice) {
                        if let Some(indices) =
                            diagnostic_by_partner_money.get(&(partner, money.0, money.1, money.2))
                        {
                            candidates.extend(indices.iter().copied());
                        }
                    }
                    candidates
                        .into_iter()
                        .filter(|index| {
                            let candidate_partner =
                                partner_identity(&register_source.records[*index]);
                            let partner_is_compatible =
                                match (partner_identity(invoice), candidate_partner) {
                                    (Some(expected), Some(actual)) => expected == actual,
                                    _ => true,
                                };
                            !used_register.contains(index)
                                && !review_linked_register.contains(index)
                                && document_number(&register_source.records[*index])
                                    != invoice_document
                                && partner_is_compatible
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if diagnostic_candidates.len() == 1 {
                let index = diagnostic_candidates[0];
                review_linked_register.insert(index);
                let register = &register_source.records[index];
                let register_provenance = provenance(register_source.source, register);
                linked_register = Some(snapshot(register_source.source, register, true));
                let register_checks = [
                    text_check(
                        DocumentComparisonScope::InvoiceToSalesRegister,
                        DocumentField::Date,
                        strict_date(invoice),
                        strict_date(register),
                        DocumentErrorCode::DateMismatch,
                    ),
                    text_check(
                        DocumentComparisonScope::InvoiceToSalesRegister,
                        DocumentField::InvoiceNumber,
                        invoice_document.as_deref(),
                        document_number(register).as_deref(),
                        DocumentErrorCode::InvoiceNumberMismatch,
                    ),
                    amount_check(
                        DocumentComparisonScope::InvoiceToSalesRegister,
                        DocumentField::Pretax,
                        invoice.pretax_amount,
                        register.pretax_amount,
                        DocumentErrorCode::PretaxMismatch,
                    ),
                    amount_check(
                        DocumentComparisonScope::InvoiceToSalesRegister,
                        DocumentField::Vat,
                        invoice.vat_amount,
                        register.vat_amount,
                        DocumentErrorCode::VatMismatch,
                    ),
                    amount_check(
                        DocumentComparisonScope::InvoiceToSalesRegister,
                        DocumentField::Total,
                        total_is_source_value(invoice).then_some(invoice.total_amount),
                        Some(register.total_amount),
                        DocumentErrorCode::TotalMismatch,
                    ),
                ];
                append_mismatch_errors(
                    &register_checks,
                    &invoice_provenance,
                    &register_provenance,
                    &mut errors,
                );
                checks.extend(register_checks);
            } else if diagnostic_candidates.len() > 1 {
                review_linked_register.extend(diagnostic_candidates.iter().copied());
                errors.push(error(
                    DocumentErrorCode::AmbiguousMatch,
                    "HIGH",
                    "AMBIGUOUS_MATCH: nhiều counterpart có cùng evidence mạnh; không đoán chứng từ.",
                    diagnostic_candidates
                        .iter()
                        .map(|index| {
                            provenance(register_source.source, &register_source.records[*index])
                        })
                        .collect(),
                ));
            } else {
                errors.push(error(
                    DocumentErrorCode::MissingInBk,
                    "HIGH",
                    "MISSING_IN_BK: không tìm thấy chứng từ tương ứng trong bảng kê bán hàng.",
                    vec![invoice_provenance.clone()],
                ));
            }
        }

        if let Some(document) = invoice_document.as_ref() {
            let candidates = ledger_by_document
                .get(document)
                .cloned()
                .unwrap_or_default();
            if candidates.len() == 1 {
                let index = candidates[0];
                used_ledger.insert(index);
                let ledger = &ledger_source.records[index];
                let ledger_provenance = provenance(ledger_source.source, ledger);
                linked_ledger = Some(ledger_snapshot(ledger_source.source, ledger));
                errors.extend(ledger_validation[index].clone());
                let ledger_checks = [
                    text_check(
                        DocumentComparisonScope::InvoiceToLedger511,
                        DocumentField::Date,
                        strict_date(invoice),
                        strict_date(ledger),
                        DocumentErrorCode::DateMismatch,
                    ),
                    text_check(
                        DocumentComparisonScope::InvoiceToLedger511,
                        DocumentField::InvoiceNumber,
                        Some(document),
                        document_number(ledger).as_deref(),
                        DocumentErrorCode::InvoiceNumberMismatch,
                    ),
                    amount_check(
                        DocumentComparisonScope::InvoiceToLedger511,
                        DocumentField::Pretax,
                        invoice.pretax_amount,
                        ledger.credit_amount,
                        DocumentErrorCode::PretaxMismatch,
                    ),
                ];
                append_mismatch_errors(
                    &ledger_checks,
                    &invoice_provenance,
                    &ledger_provenance,
                    &mut errors,
                );
                checks.extend(ledger_checks);
            } else if candidates.len() > 1 {
                errors.push(error(
                    DocumentErrorCode::AmbiguousMatch,
                    "HIGH",
                    "AMBIGUOUS_MATCH: nhiều dòng TK511 có cùng Số ct; không đoán.",
                    candidates
                        .iter()
                        .map(|index| {
                            provenance(ledger_source.source, &ledger_source.records[*index])
                        })
                        .collect(),
                ));
            } else {
                errors.push(error(
                    DocumentErrorCode::MissingInTk511,
                    "HIGH",
                    format!(
                        "HIGH: MISSING_IN_TK511: chứng từ #{} có trên Thuế và BK nhưng thiếu trong TK511.",
                        document
                    ),
                    vec![invoice_provenance.clone()],
                ));
            }
        }
        checks.extend(not_checked_ledger_fields(invoice));

        documents.push(document_case(
            format!("invoice:{}", invoice.id),
            Some(invoice_snapshot),
            linked_register,
            linked_ledger,
            checks,
            errors,
        ));
    }

    for (index, register) in register_source.records.iter().enumerate() {
        if used_register.contains(&index)
            || review_linked_register.contains(&index)
            || duplicate_register_indices.contains(&index)
            || !register_validation[index].is_empty()
        {
            continue;
        }
        documents.push(document_case(
            format!("register-extra:{}", register.id),
            None,
            Some(snapshot(register_source.source, register, true)),
            None,
            Vec::new(),
            vec![error(
                DocumentErrorCode::ExtraInBk,
                "HIGH",
                "EXTRA_IN_BK: chứng từ chỉ có trong bảng kê bán hàng.",
                vec![provenance(register_source.source, register)],
            )],
        ));
    }
    for (index, ledger) in ledger_source.records.iter().enumerate() {
        if used_ledger.contains(&index) {
            continue;
        }
        documents.push(document_case(
            format!("ledger-extra:{}", ledger.id),
            None,
            None,
            Some(ledger_snapshot(ledger_source.source, ledger)),
            Vec::new(),
            {
                let mut errors = ledger_validation[index].clone();
                errors.push(error(
                    DocumentErrorCode::ExtraInTk511,
                    "MEDIUM",
                    "EXTRA_IN_TK511: chứng từ chỉ có trong TK511.",
                    vec![provenance(ledger_source.source, ledger)],
                ));
                errors
            },
        ));
    }

    let summary = summarize(&documents);
    let totals = totals(invoice_source.records, register_source.records);
    let totals_equal = totals
        .iter()
        .all(|check| check.status == TotalsCheckStatus::Equal);
    let documents_pass = documents
        .iter()
        .all(|document| document.status == DocumentCaseStatus::FullyMatched);
    DocumentIntegrityResult {
        summary,
        totals,
        totals_equal,
        documents_pass,
        documents,
    }
}

fn summarize(documents: &[DocumentIntegrityCase]) -> DocumentIntegritySummary {
    let count_cases = |code| {
        documents
            .iter()
            .filter(|document| document.errors.iter().any(|error| error.code == code))
            .count()
    };
    DocumentIntegritySummary {
        fully_matched: documents
            .iter()
            .filter(|document| {
                document.invoice.is_some() && document.status == DocumentCaseStatus::FullyMatched
            })
            .count(),
        date_mismatch: count_cases(DocumentErrorCode::DateMismatch),
        invoice_number_mismatch: count_cases(DocumentErrorCode::InvoiceNumberMismatch),
        pretax_mismatch: count_cases(DocumentErrorCode::PretaxMismatch),
        vat_mismatch: count_cases(DocumentErrorCode::VatMismatch),
        total_mismatch: count_cases(DocumentErrorCode::TotalMismatch),
        missing_in_bk: count_cases(DocumentErrorCode::MissingInBk),
        extra_in_bk: count_cases(DocumentErrorCode::ExtraInBk),
        duplicate_invoice_number: documents
            .iter()
            .filter(|document| {
                document.sales_register.is_some()
                    && document
                        .errors
                        .iter()
                        .any(|error| error.code == DocumentErrorCode::DuplicateInvoiceNumber)
            })
            .count(),
        ambiguous_match: count_cases(DocumentErrorCode::AmbiguousMatch),
        invalid_date: count_cases(DocumentErrorCode::InvalidDate),
        missing_invoice_number: count_cases(DocumentErrorCode::MissingInvoiceNumber),
        invalid_amount: count_cases(DocumentErrorCode::InvalidAmount),
        missing_in_tk511: count_cases(DocumentErrorCode::MissingInTk511),
    }
}

fn totals(invoices: &[CanonicalRecord], registers: &[CanonicalRecord]) -> Vec<DocumentTotalCheck> {
    let invoice_pretax = invoices
        .iter()
        .filter_map(|record| record.pretax_amount)
        .sum();
    let register_pretax = registers
        .iter()
        .filter_map(|record| record.pretax_amount)
        .sum();
    let invoice_vat = invoices.iter().filter_map(|record| record.vat_amount).sum();
    let register_vat = registers
        .iter()
        .filter_map(|record| record.vat_amount)
        .sum();
    let invoice_total: Decimal = invoices
        .iter()
        .filter(|record| total_is_source_value(record))
        .map(|record| record.total_amount)
        .sum();
    let register_total: Decimal = registers
        .iter()
        .filter(|record| total_is_source_value(record))
        .map(|record| record.total_amount)
        .sum();
    let has_unscoped_date = invoices.iter().any(|record| strict_date(record).is_none())
        || registers.iter().any(|record| strict_date(record).is_none());
    let invalid_pretax = has_unscoped_date
        || invoices.iter().any(|record| record.pretax_amount.is_none())
        || registers
            .iter()
            .any(|record| record.pretax_amount.is_none());
    let invalid_vat = has_unscoped_date
        || invoices.iter().any(|record| record.vat_amount.is_none())
        || registers.iter().any(|record| record.vat_amount.is_none());
    let invalid_total = has_unscoped_date
        || invoices.iter().any(|record| !total_is_source_value(record))
        || registers
            .iter()
            .any(|record| !total_is_source_value(record));
    [
        (
            DocumentField::Pretax,
            invoice_pretax,
            register_pretax,
            invalid_pretax,
        ),
        (DocumentField::Vat, invoice_vat, register_vat, invalid_vat),
        (
            DocumentField::Total,
            invoice_total,
            register_total,
            invalid_total,
        ),
    ]
    .into_iter()
    .map(|(field, invoice_total, sales_register_total, invalid)| {
        let variance = invoice_total - sales_register_total;
        DocumentTotalCheck {
            field,
            invoice_total,
            sales_register_total,
            variance,
            status: if invalid {
                TotalsCheckStatus::NotVerified
            } else if variance.is_zero() {
                TotalsCheckStatus::Equal
            } else {
                TotalsCheckStatus::Mismatch
            },
        }
    })
    .collect()
}

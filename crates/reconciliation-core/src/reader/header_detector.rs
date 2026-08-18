use crate::models::{ColumnMapping, DataSourceKind};

/// Removes Vietnamese diacritics / accents for robust fuzzy keyword matching
pub fn remove_diacritics(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    for c in input.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        let mapped = match lower {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ'
            | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => 'e',
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ'
            | 'ợ' | 'ở' | 'ỡ' => 'o',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => 'u',
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'đ' => 'd',
            other => other,
        };
        s.push(mapped);
    }
    s
}

fn normalize_token(s: &str) -> String {
    let no_accent = remove_diacritics(s);
    no_accent
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_lowercase()
}

fn matches_any(header: &str, keywords: &[&str]) -> bool {
    let norm = normalize_token(header);
    keywords.iter().any(|&kw| {
        let norm_kw = normalize_token(kw);
        norm == norm_kw || norm.contains(&norm_kw)
    })
}

const DOC_NO_KEYWORDS: &[&str] = &[
    "so hoa don", "so hd", "so ct", "so chung tu", "so phieu", "invoice no", "inv no",
    "so van ban", "ma giao dich", "so ref", "ma ct", "so hdon",
];

const SERIES_KEYWORDS: &[&str] = &[
    "ky hieu", "mau so", "series", "mau hd", "mau so ky hieu", "ky hieu hoa don",
];

const DATE_KEYWORDS: &[&str] = &[
    "ngay hoa don", "ngay hd", "ngay ct", "ngay chung tu", "ngay lap", "ngay gd",
    "ngay giao dich", "date", "invoice date", "ngay ghi so", "ngay phat hanh",
];

const TAX_ID_KEYWORDS: &[&str] = &[
    "ma so thue", "mst", "mst nguoi mua", "mst nguoi ban", "tax id", "tax code",
    "ma so thue doi tac", "mst khach hang", "mst dv",
];

const PARTNER_NAME_KEYWORDS: &[&str] = &[
    "ten khach hang", "ten nguoi mua", "ten don vi", "ten doi tac", "customer name",
    "buyer name", "ten doi tuong", "don vi mua hang", "ten cong ty", "nguoi nop tien", "doi tuong",
];

const PRETAX_AMOUNT_KEYWORDS: &[&str] = &[
    "tien chua thue", "tong tien chua thue", "doanh so", "tien hang", "doanh thu", "chua thue",
    "doanh thu ban hang", "pretax", "amount before tax", "thanh tien chua thue", "gia tri chua thue",
];

const VAT_AMOUNT_KEYWORDS: &[&str] = &[
    "tien thue", "tong tien thue", "thue gtgt", "tien thue gtgt", "thue vat", "vat amount",
    "tax amount", "tien vat", "thue suat gtgt tien",
];

const TOTAL_AMOUNT_KEYWORDS: &[&str] = &[
    "tong tien", "tong cong", "tong thanh toan", "thanh tien", "total amount", "grand total",
    "tong gia tri", "so tien thanh toan", "tong tien tt",
];

const DEBIT_AMOUNT_KEYWORDS: &[&str] = &[
    "phat sinh no", "so phat sinh no", "tien no", "debit amount", "ps no", "ps_no",
];

const CREDIT_AMOUNT_KEYWORDS: &[&str] = &[
    "phat sinh co", "so phat sinh co", "tien co", "credit amount", "ps co", "ps_co",
];

const VAT_RATE_KEYWORDS: &[&str] = &[
    "thue suat", "vat rate", "phan tram thue", "% thue", "ts",
];

const DEBIT_ACCOUNT_KEYWORDS: &[&str] = &[
    "tk no", "tai khoan no", "debit account", "debit acc", "tkno",
];

const CREDIT_ACCOUNT_KEYWORDS: &[&str] = &[
    "tk co", "tai khoan co", "credit account", "credit acc", "tkco",
];

const VOUCHER_NO_KEYWORDS: &[&str] = &[
    "so chung tu", "so ct", "so phieu", "voucher no", "so phieu thu", "so phieu chi",
];

const DESCRIPTION_KEYWORDS: &[&str] = &[
    "dien giai", "noi dung", "description", "memo", "ly do", "noi dung thanh toan",
];

const BANK_ACCOUNT_KEYWORDS: &[&str] = &[
    "so tai khoan", "so tk", "bank account", "account number", "stk",
];

/// Analyzes sheet rows to detect header row index, column names, and suggested mappings
pub fn detect_header_and_mapping(
    raw_rows: &[Vec<String>],
) -> (u32, u32, Vec<String>, ColumnMapping, DataSourceKind, f64) {
    if raw_rows.is_empty() {
        return (1, 2, vec![], ColumnMapping::default(), DataSourceKind::Custom, 0.0);
    }

    let mut best_row_idx = 0;
    let mut best_score = 0;

    let scan_limit = raw_rows.len().min(25);
    for (idx, row) in raw_rows.iter().take(scan_limit).enumerate() {
        let non_empty_count = row.iter().filter(|c| !c.trim().is_empty()).count();
        if non_empty_count < 2 {
            continue;
        }

        let mut score = 0;
        for cell in row {
            let t = cell.trim();
            if t.is_empty() {
                continue;
            }
            if matches_any(t, DOC_NO_KEYWORDS)
                || matches_any(t, DATE_KEYWORDS)
                || matches_any(t, TOTAL_AMOUNT_KEYWORDS)
                || matches_any(t, PRETAX_AMOUNT_KEYWORDS)
                || matches_any(t, VAT_AMOUNT_KEYWORDS)
                || matches_any(t, DEBIT_AMOUNT_KEYWORDS)
                || matches_any(t, CREDIT_AMOUNT_KEYWORDS)
                || matches_any(t, TAX_ID_KEYWORDS)
                || matches_any(t, PARTNER_NAME_KEYWORDS)
                || matches_any(t, SERIES_KEYWORDS)
                || matches_any(t, DEBIT_ACCOUNT_KEYWORDS)
                || matches_any(t, CREDIT_ACCOUNT_KEYWORDS)
                || matches_any(t, DESCRIPTION_KEYWORDS)
                || matches_any(t, BANK_ACCOUNT_KEYWORDS)
            {
                score += 10;
            }
        }

        if score > best_score {
            best_score = score;
            best_row_idx = idx;
        }
    }

    let header_row_idx_1based = (best_row_idx + 1) as u32;
    let data_start_row_1based = header_row_idx_1based + 1;

    let raw_header = &raw_rows[best_row_idx];
    let columns: Vec<String> = raw_header
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let trimmed = val.trim();
            if trimmed.is_empty() {
                format!("Cột {}", i + 1)
            } else {
                trimmed.to_string()
            }
        })
        .collect();

    let mut mapping = ColumnMapping::default();
    let mut matched_count = 0;

    for col in &columns {
        // Priority 1: Check debit amount vs credit amount first to prevent misclassifying as total
        if mapping.credit_amount_column.is_none() && matches_any(col, CREDIT_AMOUNT_KEYWORDS) {
            mapping.credit_amount_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.debit_amount_column.is_none() && matches_any(col, DEBIT_AMOUNT_KEYWORDS) {
            mapping.debit_amount_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.pretax_amount_column.is_none() && matches_any(col, PRETAX_AMOUNT_KEYWORDS) {
            mapping.pretax_amount_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.vat_amount_column.is_none() && matches_any(col, VAT_AMOUNT_KEYWORDS) {
            mapping.vat_amount_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.total_amount_column.is_none() && matches_any(col, TOTAL_AMOUNT_KEYWORDS) {
            mapping.total_amount_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.doc_no_column.is_none() && matches_any(col, DOC_NO_KEYWORDS) {
            mapping.doc_no_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.series_column.is_none() && matches_any(col, SERIES_KEYWORDS) {
            mapping.series_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.date_column.is_none() && matches_any(col, DATE_KEYWORDS) {
            mapping.date_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.partner_tax_id_column.is_none() && matches_any(col, TAX_ID_KEYWORDS) {
            mapping.partner_tax_id_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.partner_name_column.is_none() && matches_any(col, PARTNER_NAME_KEYWORDS) {
            mapping.partner_name_column = Some(col.clone());
            matched_count += 1;
        } else if mapping.vat_rate_column.is_none() && matches_any(col, VAT_RATE_KEYWORDS) {
            mapping.vat_rate_column = Some(col.clone());
        } else if mapping.debit_account_column.is_none() && matches_any(col, DEBIT_ACCOUNT_KEYWORDS) {
            mapping.debit_account_column = Some(col.clone());
        } else if mapping.credit_account_column.is_none() && matches_any(col, CREDIT_ACCOUNT_KEYWORDS) {
            mapping.credit_account_column = Some(col.clone());
        } else if mapping.voucher_no_column.is_none() && matches_any(col, VOUCHER_NO_KEYWORDS) {
            mapping.voucher_no_column = Some(col.clone());
        } else if mapping.description_column.is_none() && matches_any(col, DESCRIPTION_KEYWORDS) {
            mapping.description_column = Some(col.clone());
        } else if mapping.bank_account_column.is_none() && matches_any(col, BANK_ACCOUNT_KEYWORDS) {
            mapping.bank_account_column = Some(col.clone());
        }
    }

    // Infer Kind
    let kind = if mapping.series_column.is_some() || mapping.vat_amount_column.is_some() || mapping.pretax_amount_column.is_some() {
        DataSourceKind::EInvoice
    } else if mapping.credit_amount_column.is_some() || mapping.debit_amount_column.is_some() || mapping.debit_account_column.is_some() || mapping.credit_account_column.is_some() {
        DataSourceKind::Ledger511
    } else if mapping.bank_account_column.is_some() {
        DataSourceKind::BankStatement
    } else if mapping.total_amount_column.is_some() && mapping.doc_no_column.is_some() {
        DataSourceKind::EInvoice
    } else {
        DataSourceKind::Custom
    };

    let confidence = (matched_count as f64 / 4.0).min(1.0);

    (
        header_row_idx_1based,
        data_start_row_1based,
        columns,
        mapping,
        kind,
        confidence,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_detection_einvoice() {
        let rows = vec![
            vec!["BẢNG KÊ HÓA ĐƠN ĐIỆN TỬ BÁN RA".to_string(), "".to_string()],
            vec!["Kỳ tính thuế: Tháng 01/2026".to_string(), "".to_string()],
            vec![
                "STT".to_string(),
                "Ký hiệu HĐ".to_string(),
                "Số hóa đơn".to_string(),
                "Ngày lập".to_string(),
                "Mã số thuế".to_string(),
                "Tên khách hàng".to_string(),
                "Tổng tiền chưa thuế".to_string(),
                "Tổng tiền thuế".to_string(),
                "Tổng tiền thanh toán".to_string(),
            ],
            vec![
                "1".to_string(),
                "1C26TAA".to_string(),
                "00000101".to_string(),
                "05/01/2026".to_string(),
                "0109990001".to_string(),
                "Công ty Sao Mai".to_string(),
                "10,000,000".to_string(),
                "1,000,000".to_string(),
                "11,000,000".to_string(),
            ],
        ];

        let (header_row, data_start, cols, mapping, kind, conf) = detect_header_and_mapping(&rows);
        assert_eq!(header_row, 3);
        assert_eq!(data_start, 4);
        assert_eq!(cols.len(), 9);
        assert_eq!(mapping.doc_no_column.as_deref(), Some("Số hóa đơn"));
        assert_eq!(mapping.series_column.as_deref(), Some("Ký hiệu HĐ"));
        assert_eq!(mapping.date_column.as_deref(), Some("Ngày lập"));
        assert_eq!(mapping.partner_tax_id_column.as_deref(), Some("Mã số thuế"));
        assert_eq!(mapping.pretax_amount_column.as_deref(), Some("Tổng tiền chưa thuế"));
        assert_eq!(mapping.vat_amount_column.as_deref(), Some("Tổng tiền thuế"));
        assert_eq!(mapping.total_amount_column.as_deref(), Some("Tổng tiền thanh toán"));
        assert_eq!(kind, DataSourceKind::EInvoice);
        assert!(conf >= 0.9);
    }

    #[test]
    fn test_header_detection_tk511_credit_debit() {
        let rows = vec![
            vec![
                "Ngày ct".to_string(),
                "Số ct".to_string(),
                "Diễn giải".to_string(),
                "TK đối ứng".to_string(),
                "Phát sinh nợ".to_string(),
                "Phát sinh có".to_string(),
            ],
            vec![
                "05/01/2026".to_string(),
                "101".to_string(),
                "Bán hàng".to_string(),
                "131".to_string(),
                "".to_string(),
                "10,000,000".to_string(),
            ],
        ];

        let (_, _, _, mapping, kind, _) = detect_header_and_mapping(&rows);
        assert_eq!(mapping.doc_no_column.as_deref(), Some("Số ct"));
        assert_eq!(mapping.date_column.as_deref(), Some("Ngày ct"));
        assert_eq!(mapping.debit_amount_column.as_deref(), Some("Phát sinh nợ"));
        assert_eq!(mapping.credit_amount_column.as_deref(), Some("Phát sinh có"));
        assert_eq!(mapping.total_amount_column, None); // MUST NOT map Phát sinh nợ/có to total_amount
        assert_eq!(kind, DataSourceKind::Ledger511);
    }
}

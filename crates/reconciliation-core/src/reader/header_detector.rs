use crate::models::{ColumnMapping, DataSourceKind};

/// Removes Vietnamese diacritics / accents for robust keyword matching
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

fn matches_exact_or_contains(header: &str, keywords: &[&str]) -> bool {
    let norm = normalize_token(header);
    keywords.iter().any(|&kw| {
        let norm_kw = normalize_token(kw);
        norm == norm_kw || norm.contains(&norm_kw)
    })
}

fn matches_exact(header: &str, keywords: &[&str]) -> bool {
    let norm = normalize_token(header);
    keywords.iter().any(|&kw| {
        let norm_kw = normalize_token(kw);
        norm == norm_kw
    })
}

/// Document number keywords (strictly excludes "Mã ct" / "Mã chứng từ")
const DOC_NO_KEYWORDS: &[&str] = &[
    "so hoa don", "so hd", "so ct", "so chung tu", "so phieu", "invoice no", "inv no",
    "so ref", "so hdon", "document no", "doc no",
];

/// Document code / voucher type keywords (e.g. "Mã ct", "Loại ct")
const DOC_CODE_KEYWORDS: &[&str] = &[
    "ma ct", "ma chung tu", "loai ct", "loai chung tu", "voucher type", "doc code",
];

/// Invoice Template keywords (e.g. "Ký hiệu mẫu số", "Mẫu số")
const TEMPLATE_CODE_KEYWORDS: &[&str] = &[
    "ky hieu mau so", "mau so ky hieu", "mau so", "mau hd", "template code",
];

/// Invoice Series keywords (e.g. "Ký hiệu hóa đơn", "Ký hiệu HĐ", "Ký hiệu")
const SERIES_KEYWORDS: &[&str] = &[
    "ky hieu hoa don", "ky hieu hd", "ky hieu", "series",
];

/// Date keywords
const DATE_KEYWORDS: &[&str] = &[
    "ngay hoa don", "ngay hd", "ngay ct", "ngay chung tu", "ngay lap", "ngay gd",
    "ngay giao dich", "date", "invoice date", "ngay ghi so", "ngay phat hanh",
];

/// Buyer Tax ID keywords
const BUYER_TAX_ID_KEYWORDS: &[&str] = &[
    "mst nguoi mua", "ma so thue nguoi mua", "mst khach hang", "mst doi tac", "buyer tax id",
];

/// Seller Tax ID keywords
const SELLER_TAX_ID_KEYWORDS: &[&str] = &[
    "mst nguoi ban", "ma so thue nguoi ban", "mst don vi", "seller tax id",
];

/// General Tax ID keywords
const TAX_ID_KEYWORDS: &[&str] = &[
    "ma so thue", "mst", "tax id", "tax code", "mst dv",
];

/// Partner Name keywords
const PARTNER_NAME_KEYWORDS: &[&str] = &[
    "ten khach hang", "ten nguoi mua", "ten don vi", "ten doi tac", "customer name",
    "buyer name", "ten doi tuong", "don vi mua hang", "ten cong ty", "nguoi nop tien", "doi tuong",
];

/// Pretax amount keywords (e.g. "Tổng tiền chưa thuế")
const PRETAX_AMOUNT_KEYWORDS: &[&str] = &[
    "tong tien chua thue", "tien chua thue", "doanh so", "tien hang", "doanh thu", "chua thue",
    "doanh thu ban hang", "pretax", "amount before tax", "thanh tien chua thue", "gia tri chua thue",
];

/// VAT amount keywords (e.g. "Tổng tiền thuế")
const VAT_AMOUNT_KEYWORDS: &[&str] = &[
    "tong tien thue", "tien thue gtgt", "thue gtgt", "tien thue", "thue vat", "vat amount",
    "tax amount", "tien vat", "thue suat gtgt tien",
];

/// Commercial discount keywords (e.g. "Tổng tiền chiết khấu thương mại")
const DISCOUNT_AMOUNT_KEYWORDS: &[&str] = &[
    "tong tien chiet khau", "chiet khau thuong mai", "tien chiet khau", "chiet khau", "giam gia", "tong giam gia", "discount",
];

/// Fee amount keywords (e.g. "Tổng tiền phí", "Tiền phí")
const FEE_AMOUNT_KEYWORDS: &[&str] = &[
    "tong tien phi", "tien phi", "phi", "le phi", "fee amount", "fee",
];

/// Total amount keywords with exact semantic priority
const TOTAL_AMOUNT_HIGH_PRIORITY_KEYWORDS: &[&str] = &[
    "tong tien thanh toan", "tong thanh toan", "thanh tien thanh toan", "so tien thanh toan",
    "tong cong tien thanh toan", "tong gia tri thanh toan", "tong thanh toan sau thue", "tong tien tt",
];

const TOTAL_AMOUNT_GENERIC_KEYWORDS: &[&str] = &[
    "total amount", "grand total", "tong cong", "tong tien",
];

/// Debit amount keywords
const DEBIT_AMOUNT_KEYWORDS: &[&str] = &[
    "phat sinh no", "so phat sinh no", "tien no", "debit amount", "ps no", "ps_no",
];

/// Credit amount keywords
const CREDIT_AMOUNT_KEYWORDS: &[&str] = &[
    "phat sinh co", "so phat sinh co", "tien co", "credit amount", "ps co", "ps_co",
];

const VAT_RATE_KEYWORDS: &[&str] = &[
    "thue suat", "vat rate", "phan tram thue", "% thue", "ts",
];

const DEBIT_ACCOUNT_KEYWORDS: &[&str] = &[
    "tk no", "tai khoan no", "debit account", "debit acc", "tkno", "tk doi ung no",
];

const CREDIT_ACCOUNT_KEYWORDS: &[&str] = &[
    "tk co", "tai khoan co", "credit account", "credit acc", "tkco", "tk doi ung", "tk doi ung co",
];

const VOUCHER_NO_KEYWORDS: &[&str] = &[
    "so phieu", "voucher no", "so phieu thu", "so phieu chi",
];

const DESCRIPTION_KEYWORDS: &[&str] = &[
    "dien giai", "noi dung", "description", "memo", "ly do", "noi dung thanh toan",
];

const BANK_ACCOUNT_KEYWORDS: &[&str] = &[
    "so tai khoan", "so tk", "bank account", "account number", "stk",
];

/// Backward compatibility entry point
pub fn detect_header_and_mapping(
    raw_rows: &[Vec<String>],
) -> (u32, u32, Vec<String>, ColumnMapping, DataSourceKind, f64) {
    detect_header_and_mapping_with_context("", raw_rows)
}

/// Analyzes sheet rows and sheet context (title/metadata) to detect header row index, column mappings, and source kind
pub fn detect_header_and_mapping_with_context(
    sheet_context: &str,
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
            if matches_exact_or_contains(t, DOC_NO_KEYWORDS)
                || matches_exact_or_contains(t, DATE_KEYWORDS)
                || matches_exact_or_contains(t, TOTAL_AMOUNT_HIGH_PRIORITY_KEYWORDS)
                || matches_exact_or_contains(t, TOTAL_AMOUNT_GENERIC_KEYWORDS)
                || matches_exact_or_contains(t, PRETAX_AMOUNT_KEYWORDS)
                || matches_exact_or_contains(t, VAT_AMOUNT_KEYWORDS)
                || matches_exact_or_contains(t, DEBIT_AMOUNT_KEYWORDS)
                || matches_exact_or_contains(t, CREDIT_AMOUNT_KEYWORDS)
                || matches_exact_or_contains(t, TAX_ID_KEYWORDS)
                || matches_exact_or_contains(t, BUYER_TAX_ID_KEYWORDS)
                || matches_exact_or_contains(t, SELLER_TAX_ID_KEYWORDS)
                || matches_exact_or_contains(t, PARTNER_NAME_KEYWORDS)
                || matches_exact_or_contains(t, SERIES_KEYWORDS)
                || matches_exact_or_contains(t, TEMPLATE_CODE_KEYWORDS)
                || matches_exact_or_contains(t, DESCRIPTION_KEYWORDS)
                || matches_exact_or_contains(t, FEE_AMOUNT_KEYWORDS)
                || matches_exact_or_contains(t, DISCOUNT_AMOUNT_KEYWORDS)
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

    // PASS A: Specific columns (Fee, Discount, Pretax, VAT, Debit, Credit)
    for col in &columns {
        let norm = normalize_token(col);

        // 1. Debit & Credit amounts
        if mapping.credit_amount_column.is_none() && matches_exact_or_contains(col, CREDIT_AMOUNT_KEYWORDS) {
            mapping.credit_amount_column = Some(col.clone());
        } else if mapping.debit_amount_column.is_none() && matches_exact_or_contains(col, DEBIT_AMOUNT_KEYWORDS) {
            mapping.debit_amount_column = Some(col.clone());
        }
        // 2. Fee amount (e.g. "Tổng tiền phí", "Tiền phí")
        else if mapping.fee_amount_column.is_none() && matches_exact_or_contains(col, FEE_AMOUNT_KEYWORDS) {
            mapping.fee_amount_column = Some(col.clone());
        }
        // 3. Discount amount (e.g. "Tổng tiền chiết khấu thương mại")
        else if mapping.discount_amount_column.is_none() && matches_exact_or_contains(col, DISCOUNT_AMOUNT_KEYWORDS) {
            mapping.discount_amount_column = Some(col.clone());
        }
        // 4. Pretax amount (e.g. "Tổng tiền chưa thuế")
        else if mapping.pretax_amount_column.is_none() && matches_exact_or_contains(col, PRETAX_AMOUNT_KEYWORDS) {
            mapping.pretax_amount_column = Some(col.clone());
        }
        // 5. VAT amount (e.g. "Tổng tiền thuế")
        else if mapping.vat_amount_column.is_none() && matches_exact_or_contains(col, VAT_AMOUNT_KEYWORDS) {
            mapping.vat_amount_column = Some(col.clone());
        }
        // 6. High priority total amount (e.g. "Tổng tiền thanh toán")
        else if mapping.total_amount_column.is_none()
            && !norm.contains("phi")
            && !norm.contains("chiet khau")
            && !norm.contains("giam gia")
            && !norm.contains("chua thue")
            && (!norm.contains("thue") || norm.contains("sau thue"))
            && matches_exact_or_contains(col, TOTAL_AMOUNT_HIGH_PRIORITY_KEYWORDS)
        {
            mapping.total_amount_column = Some(col.clone());
        }
        // 7. Document code (Mã ct) vs Document number (Số ct)
        else if matches_exact(col, DOC_CODE_KEYWORDS) {
            if mapping.doc_code_column.is_none() {
                mapping.doc_code_column = Some(col.clone());
            }
        }
        // 8. Document number (Số ct, Số hóa đơn) - strictly excludes "ma ct"
        else if mapping.doc_no_column.is_none() && matches_exact_or_contains(col, DOC_NO_KEYWORDS) {
            if !norm.starts_with("ma ct") && !norm.starts_with("ma chung tu") {
                mapping.doc_no_column = Some(col.clone());
            }
        }
        // 9. Template code vs Series
        else if mapping.template_code_column.is_none() && matches_exact_or_contains(col, TEMPLATE_CODE_KEYWORDS) {
            mapping.template_code_column = Some(col.clone());
        } else if mapping.series_column.is_none() && matches_exact_or_contains(col, SERIES_KEYWORDS) && !norm.contains("mau so") {
            mapping.series_column = Some(col.clone());
        }
        // 10. Buyer Tax ID vs Seller Tax ID vs General Tax ID
        else if mapping.buyer_tax_id_column.is_none() && matches_exact_or_contains(col, BUYER_TAX_ID_KEYWORDS) {
            mapping.buyer_tax_id_column = Some(col.clone());
            if mapping.partner_tax_id_column.is_none() {
                mapping.partner_tax_id_column = Some(col.clone());
            }
        } else if mapping.seller_tax_id_column.is_none() && matches_exact_or_contains(col, SELLER_TAX_ID_KEYWORDS) {
            mapping.seller_tax_id_column = Some(col.clone());
        } else if mapping.partner_tax_id_column.is_none() && matches_exact_or_contains(col, TAX_ID_KEYWORDS) {
            mapping.partner_tax_id_column = Some(col.clone());
        }
        // 11. Dates, Accounts, Descriptions
        else if mapping.date_column.is_none() && matches_exact_or_contains(col, DATE_KEYWORDS) {
            mapping.date_column = Some(col.clone());
        } else if mapping.partner_name_column.is_none() && matches_exact_or_contains(col, PARTNER_NAME_KEYWORDS) {
            mapping.partner_name_column = Some(col.clone());
        } else if mapping.vat_rate_column.is_none() && matches_exact_or_contains(col, VAT_RATE_KEYWORDS) {
            mapping.vat_rate_column = Some(col.clone());
        } else if mapping.debit_account_column.is_none() && matches_exact_or_contains(col, DEBIT_ACCOUNT_KEYWORDS) {
            mapping.debit_account_column = Some(col.clone());
        } else if mapping.credit_account_column.is_none() && matches_exact_or_contains(col, CREDIT_ACCOUNT_KEYWORDS) {
            mapping.credit_account_column = Some(col.clone());
        } else if mapping.voucher_no_column.is_none() && matches_exact_or_contains(col, VOUCHER_NO_KEYWORDS) {
            mapping.voucher_no_column = Some(col.clone());
        } else if mapping.description_column.is_none() && matches_exact_or_contains(col, DESCRIPTION_KEYWORDS) {
            mapping.description_column = Some(col.clone());
        } else if mapping.bank_account_column.is_none() && matches_exact_or_contains(col, BANK_ACCOUNT_KEYWORDS) {
            mapping.bank_account_column = Some(col.clone());
        }
    }

    // PASS B: Generic total amount fallback (only if not already mapped, and strictly not fee/tax/discount)
    if mapping.total_amount_column.is_none() {
        for col in &columns {
            let norm = normalize_token(col);
            if !norm.contains("phi")
                && !norm.contains("chiet khau")
                && !norm.contains("giam gia")
                && !norm.contains("chua thue")
                && (!norm.contains("thue") || norm.contains("sau thue"))
                && matches_exact_or_contains(col, TOTAL_AMOUNT_GENERIC_KEYWORDS)
            {
                mapping.total_amount_column = Some(col.clone());
                break;
            }
        }
    }

    // Contextual Source Classification
    // Extract metadata tokens from sheet_context and pre-header rows
    let mut context_str = sheet_context.to_lowercase();
    for row in raw_rows.iter().take(best_row_idx) {
        for cell in row {
            if !cell.trim().is_empty() {
                context_str.push(' ');
                context_str.push_str(&cell.to_lowercase());
            }
        }
    }
    let norm_context = normalize_token(&context_str);

    let (kind, confidence) = if norm_context.contains("511")
        || norm_context.contains("doanh thu")
        || (norm_context.contains("ban hang") && mapping.credit_amount_column.is_some())
    {
        (DataSourceKind::Ledger511, 0.95)
    } else if norm_context.contains("3331")
        || norm_context.contains("thue gtgt phai nop")
        || norm_context.contains("thue gtgt dau ra")
    {
        (DataSourceKind::Ledger3331, 0.95)
    } else if norm_context.contains("131")
        || norm_context.contains("phai thu khach hang")
        || norm_context.contains("cong no phai thu")
    {
        (DataSourceKind::Ledger131, 0.95)
    } else if norm_context.contains("133")
        || norm_context.contains("thue gtgt dau vao")
        || norm_context.contains("thue gtgt duoc khau tru")
    {
        (DataSourceKind::Ledger133, 0.95)
    } else if mapping.series_column.is_some()
        || mapping.template_code_column.is_some()
        || (mapping.pretax_amount_column.is_some() && mapping.vat_amount_column.is_some())
        || norm_context.contains("hoa don")
        || norm_context.contains("einvoice")
    {
        (DataSourceKind::EInvoice, 0.95)
    } else if mapping.bank_account_column.is_some()
        || norm_context.contains("sao ke")
        || norm_context.contains("ngan hang")
        || norm_context.contains("bank")
    {
        (DataSourceKind::BankStatement, 0.90)
    } else if norm_context.contains("so quy") || norm_context.contains("tien mat") {
        (DataSourceKind::CashBook, 0.90)
    } else if norm_context.contains("chi nhanh") {
        (DataSourceKind::BranchLedger, 0.85)
    } else {
        // Generic ledger or table without clear account code / report title
        // Must NOT blindly guess Ledger511! Set Custom / SOURCE_TYPE_REQUIRED
        (DataSourceKind::Custom, 0.40)
    };

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
    fn test_header_detection_real_invoice_headers_with_fee_and_discount() {
        let rows = vec![
            vec![
                "Ký hiệu mẫu số".to_string(),
                "Ký hiệu hóa đơn".to_string(),
                "Số hóa đơn".to_string(),
                "Ngày lập".to_string(),
                "MST người bán".to_string(),
                "MST người mua".to_string(),
                "Tổng tiền chưa thuế".to_string(),
                "Tổng tiền thuế".to_string(),
                "Tổng tiền chiết khấu thương mại".to_string(),
                "Tổng tiền phí".to_string(),
                "Tổng tiền thanh toán".to_string(),
            ],
            vec![
                "1".to_string(),
                "1C26TAA".to_string(),
                "00000101".to_string(),
                "05/01/2026".to_string(),
                "0100000000".to_string(),
                "0109990001".to_string(),
                "10,000,000".to_string(),
                "1,000,000".to_string(),
                "200,000".to_string(),
                "50,000".to_string(),
                "10,850,000".to_string(),
            ],
        ];

        let (_, _, _, mapping, kind, _) = detect_header_and_mapping(&rows);
        assert_eq!(mapping.pretax_amount_column.as_deref(), Some("Tổng tiền chưa thuế"));
        assert_eq!(mapping.vat_amount_column.as_deref(), Some("Tổng tiền thuế"));
        assert_eq!(mapping.discount_amount_column.as_deref(), Some("Tổng tiền chiết khấu thương mại"));
        assert_eq!(mapping.fee_amount_column.as_deref(), Some("Tổng tiền phí"));
        assert_eq!(mapping.total_amount_column.as_deref(), Some("Tổng tiền thanh toán")); // MUST NOT be fee or discount!
        assert_eq!(kind, DataSourceKind::EInvoice);
    }

    #[test]
    fn test_source_classification_with_context() {
        let tk511_rows = vec![
            vec!["SỔ CHI TIẾT TÀI KHOẢN 511".to_string()],
            vec!["Tài khoản: 5111 - Doanh thu bán hàng".to_string()],
            vec![
                "Ngày ct".to_string(),
                "Mã ct".to_string(),
                "Số ct".to_string(),
                "Diễn giải".to_string(),
                "Phát sinh nợ".to_string(),
                "Phát sinh có".to_string(),
            ],
            vec![
                "05/01/2026".to_string(),
                "HĐ".to_string(),
                "101".to_string(),
                "Bán hàng".to_string(),
                "".to_string(),
                "10,000,000".to_string(),
            ],
        ];

        let (_, _, _, _, kind_511, conf_511) =
            detect_header_and_mapping_with_context("TK511", &tk511_rows);
        assert_eq!(kind_511, DataSourceKind::Ledger511);
        assert!(conf_511 >= 0.9);

        let tk3331_rows = vec![
            vec!["SỔ CHI TIẾT TÀI KHOẢN 3331".to_string()],
            vec![
                "Ngày ct".to_string(),
                "Mã ct".to_string(),
                "Số ct".to_string(),
                "Phát sinh nợ".to_string(),
                "Phát sinh có".to_string(),
            ],
        ];

        let (_, _, _, _, kind_3331, _) =
            detect_header_and_mapping_with_context("TK3331", &tk3331_rows);
        assert_eq!(kind_3331, DataSourceKind::Ledger3331);

        let tk131_rows = vec![
            vec!["SỔ CHI TIẾT CÔNG NỢ PHẢI THU (TK 131)".to_string()],
            vec![
                "Ngày ct".to_string(),
                "Mã ct".to_string(),
                "Số ct".to_string(),
                "Phát sinh nợ".to_string(),
                "Phát sinh có".to_string(),
            ],
        ];

        let (_, _, _, _, kind_131, _) =
            detect_header_and_mapping_with_context("TK131", &tk131_rows);
        assert_eq!(kind_131, DataSourceKind::Ledger131);

        // Unknown generic ledger without account code or context MUST NOT default to Ledger511
        let generic_rows = vec![
            vec![
                "Ngày ct".to_string(),
                "Mã ct".to_string(),
                "Số ct".to_string(),
                "Phát sinh nợ".to_string(),
                "Phát sinh có".to_string(),
            ],
            vec![
                "05/01/2026".to_string(),
                "PKT".to_string(),
                "101".to_string(),
                "".to_string(),
                "10,000,000".to_string(),
            ],
        ];

        let (_, _, _, _, generic_kind, conf) =
            detect_header_and_mapping_with_context("Sheet1", &generic_rows);
        assert_eq!(generic_kind, DataSourceKind::Custom);
        assert!(conf < 0.5);
    }
}

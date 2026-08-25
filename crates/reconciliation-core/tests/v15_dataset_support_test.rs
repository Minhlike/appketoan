use reconciliation_core::{
    detect_header_and_mapping, normalize_data_source_rows, normalize_partner_master_rows,
    resolve_partner_by_tax_id, select_sales_analysis_control_layer, AnalyticalRowLevel, DataSource,
    DataSourceKind, PartnerIdentityResolution, SalesAnalysisRecord, SourceRole,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

fn source(
    kind: DataSourceKind,
    header_row: u32,
    data_start_row: u32,
    mapping: reconciliation_core::ColumnMapping,
) -> DataSource {
    DataSource {
        id: "src".into(),
        name: "synthetic".into(),
        file_path: "synthetic.xlsx".into(),
        sheet_name: "Sheet1".into(),
        kind,
        role: SourceRole::Primary,
        header_row,
        data_start_row,
        column_mapping: mapping,
    }
}

#[test]
fn detects_v15_source_types_and_adaptive_legacy_header() {
    let cases = [
        (
            vec![vec![
                "Ngày ct".into(),
                "Mã ct".into(),
                "Số ct".into(),
                "Mã khách".into(),
                "Tên khách hàng".into(),
                "Tiền".into(),
                "Thuế".into(),
                "Phải thu".into(),
            ]],
            DataSourceKind::SalesRegister,
        ),
        (
            vec![vec![
                "Mã khách/ncc".into(),
                "Tên khách hàng/ncc".into(),
                "Địa chỉ".into(),
                "Mã số thuế".into(),
                "Khách hàng".into(),
                "Nhà cung cấp".into(),
            ]],
            DataSourceKind::PartnerMaster,
        ),
        (
            vec![vec![
                "Stt".into(),
                "Mặt hàng".into(),
                "Tên mặt hàng".into(),
                "Doanh thu".into(),
                "Thuế".into(),
                "Phải thu".into(),
                "Tiền vốn".into(),
                "Lãi".into(),
            ]],
            DataSourceKind::SalesAnalysisReport,
        ),
        (
            vec![
                vec!["Sổ cái tài khoản 112".into()],
                vec![
                    "Ngày ct".into(),
                    "Số ct".into(),
                    "Phát sinh nợ".into(),
                    "Phát sinh có".into(),
                    "Tk đối ứng".into(),
                ],
            ],
            DataSourceKind::Ledger112,
        ),
    ];
    for (rows, expected) in cases {
        let (_, _, _, _, actual, confidence) = detect_header_and_mapping(&rows);
        assert_eq!(actual, expected);
        assert!(confidence >= 0.8);
    }

    let mut bank_rows = vec![vec!["Tiêu đề".into()]; 31];
    bank_rows.push(vec![
        "Ngày hạch toán/Accounting date".into(),
        "Nợ/ Debit".into(),
        "Có / Credit".into(),
        "Số dư TK/ Account Balance".into(),
        "Số giao dịch/ Transaction number".into(),
    ]);
    let (header, _, _, mapping, kind, _) = detect_header_and_mapping(&bank_rows);
    assert_eq!(header, 32);
    assert_eq!(kind, DataSourceKind::BankStatement);
    assert!(mapping.debit_amount_column.is_some() && mapping.credit_amount_column.is_some());
}

#[test]
fn bank_correspondent_aliases_map_to_typed_counterparty_fields() {
    let rows = vec![vec![
        "Accounting date".into(),
        "Correspondent account".into(),
        "Correspondent name".into(),
        "Transaction number".into(),
        "Credit".into(),
    ]];
    let (_, _, _, mapping, _, _) = detect_header_and_mapping(&rows);
    assert_eq!(
        mapping.counterparty_account_column.as_deref(),
        Some("Correspondent account")
    );
    assert_eq!(
        mapping.counterparty_name_column.as_deref(),
        Some("Correspondent name")
    );
}

#[test]
fn vietnamese_balance_header_is_mapped_for_balance_equation_checks() {
    let rows = vec![vec![
        "Ngày hạch toán".into(),
        "Số dư".into(),
        "Phát sinh nợ".into(),
        "Phát sinh có".into(),
    ]];
    let (_, _, _, mapping, _, _) = detect_header_and_mapping(&rows);
    assert_eq!(mapping.balance_column.as_deref(), Some("Số dư"));
}

#[test]
fn preserves_invoice_lifecycle_provenance_and_sales_register_amounts() {
    let rows = vec![
        vec![
            "Số hóa đơn".into(),
            "Ngày lập".into(),
            "Tổng tiền chưa thuế".into(),
            "Tổng tiền thuế".into(),
            "Tổng tiền thanh toán".into(),
            "Trạng thái hóa đơn".into(),
            "Kết quả kiểm tra hóa đơn".into(),
        ],
        vec![
            "233".into(),
            "01/07/2026".into(),
            "100".into(),
            "10".into(),
            "110".into(),
            "Hóa đơn điều chỉnh".into(),
            "Đạt".into(),
        ],
    ];
    let (header, start, columns, mapping, kind, _) = detect_header_and_mapping(&rows);
    let records =
        normalize_data_source_rows(&source(kind, header, start, mapping), &columns, &rows);
    assert_eq!(records.len(), 1);
    assert!(records[0].raw_fields.contains_key("Trạng thái hóa đơn"));
    assert!(records[0]
        .raw_fields
        .contains_key("Kết quả kiểm tra hóa đơn"));
}

#[test]
fn partner_master_is_not_filtered_and_duplicate_tax_id_is_ambiguous() {
    let rows = vec![
        vec![
            "Mã khách/ncc".into(),
            "Tên khách hàng/ncc".into(),
            "Mã số thuế".into(),
            "Địa chỉ".into(),
            "Khách hàng".into(),
            "Nhà cung cấp".into(),
            "Trạng thái".into(),
        ],
        vec![
            "0502".into(),
            "Partner A".into(),
            "0100000000".into(),
            "Address".into(),
            "x".into(),
            "".into(),
            "Đang dùng".into(),
        ],
        vec![
            "0503".into(),
            "Partner B".into(),
            "0100000000".into(),
            "Address".into(),
            "".into(),
            "x".into(),
            "Đang dùng".into(),
        ],
    ];
    let (header, start, columns, mapping, kind, _) = detect_header_and_mapping(&rows);
    let records =
        normalize_partner_master_rows(&source(kind, header, start, mapping), &columns, &rows);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].is_customer, Some(true));
    assert_eq!(records[1].is_supplier, Some(true));
    assert_eq!(records[0].status.as_deref(), Some("Đang dùng"));
    assert!(
        matches!(resolve_partner_by_tax_id(&records, "0100000000"), PartnerIdentityResolution::AmbiguousMasterIdentity(ids) if ids.len() == 2)
    );
}

fn analysis(row_level: AnalyticalRowLevel, revenue: Decimal) -> SalesAnalysisRecord {
    SalesAnalysisRecord {
        id: "x".into(),
        source_id: "src".into(),
        source_row: 1,
        row_level,
        group_key: None,
        product_code: None,
        product_name: None,
        quantity: None,
        unit_price: None,
        revenue: Some(revenue),
        vat: Some(dec!(10)),
        discount: None,
        receivable: Some(revenue + dec!(10)),
        unit_cost: None,
        cost: Some(dec!(50)),
        profit: Some(revenue - dec!(50)),
        raw_fields: HashMap::new(),
    }
}

#[test]
fn sales_analysis_uses_one_verified_layer_not_group_plus_detail() {
    let records = vec![
        analysis(AnalyticalRowLevel::Group, dec!(100)),
        analysis(AnalyticalRowLevel::Detail, dec!(100)),
    ];
    let totals = select_sales_analysis_control_layer(&records).expect("equal layers must be safe");
    assert_eq!(totals.row_level, AnalyticalRowLevel::Detail);
    assert_eq!(totals.revenue, dec!(100));
}

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::path::Path;

use reconciliation_core::{
    detect_header_and_mapping, detect_header_and_mapping_with_context, execute_reconciliation,
    inspect_excel_file, normalize_data_source_rows, read_sheet_rows,
    CanonicalRecord, ColumnMapping, DataSource, DataSourceKind, MatchStatus, ReconciliationSession,
    ValueOrigin,
};

#[test]
fn test_01_real_header_auto_detection_and_collision_avoidance_with_fees() {
    let invoice_headers = vec![
        "Ký hiệu mẫu số".to_string(),
        "Ký hiệu hóa đơn".to_string(),
        "Số hóa đơn".to_string(),
        "Ngày lập".to_string(),
        "MST người bán".to_string(),
        "MST người mua".to_string(),
        "Tên người mua".to_string(),
        "Tổng tiền chưa thuế".to_string(),
        "Tổng tiền thuế".to_string(),
        "Tổng tiền chiết khấu thương mại".to_string(),
        "Tổng tiền phí".to_string(),
        "Tổng tiền thanh toán".to_string(),
    ];

    let invoice_rows = vec![
        invoice_headers.clone(),
        vec![
            "1".to_string(),
            "1C26TAA".to_string(),
            "00000101".to_string(),
            "05/01/2026".to_string(),
            "0100000000".to_string(),
            "0109990001".to_string(),
            "CÔNG TY TNHH SAO MAI".to_string(),
            "10,000,000".to_string(),
            "1,000,000".to_string(),
            "200,000".to_string(),
            "50,000".to_string(),
            "10,850,000".to_string(),
        ],
    ];

    let (_, _, _, inv_mapping, inv_kind, _) = detect_header_and_mapping(&invoice_rows);
    assert_eq!(inv_mapping.template_code_column.as_deref(), Some("Ký hiệu mẫu số"));
    assert_eq!(inv_mapping.series_column.as_deref(), Some("Ký hiệu hóa đơn"));
    assert_eq!(inv_mapping.doc_no_column.as_deref(), Some("Số hóa đơn"));
    assert_eq!(inv_mapping.date_column.as_deref(), Some("Ngày lập"));
    assert_eq!(inv_mapping.seller_tax_id_column.as_deref(), Some("MST người bán"));
    assert_eq!(inv_mapping.buyer_tax_id_column.as_deref(), Some("MST người mua"));
    assert_eq!(inv_mapping.partner_tax_id_column.as_deref(), Some("MST người mua"));
    assert_eq!(inv_mapping.partner_name_column.as_deref(), Some("Tên người mua"));
    assert_eq!(inv_mapping.pretax_amount_column.as_deref(), Some("Tổng tiền chưa thuế"));
    assert_eq!(inv_mapping.vat_amount_column.as_deref(), Some("Tổng tiền thuế"));
    assert_eq!(inv_mapping.discount_amount_column.as_deref(), Some("Tổng tiền chiết khấu thương mại"));
    assert_eq!(inv_mapping.fee_amount_column.as_deref(), Some("Tổng tiền phí"));
    assert_eq!(inv_mapping.total_amount_column.as_deref(), Some("Tổng tiền thanh toán")); // MUST NOT be fee or discount!
    assert_eq!(inv_kind, DataSourceKind::EInvoice);

    let tk511_headers = vec![
        "Ngày ct".to_string(),
        "Mã ct".to_string(),
        "Số ct".to_string(),
        "Diễn giải".to_string(),
        "Phát sinh nợ".to_string(),
        "Phát sinh có".to_string(),
    ];

    let tk511_rows = vec![
        tk511_headers.clone(),
        vec![
            "05/01/2026".to_string(),
            "HĐ".to_string(),
            "101".to_string(),
            "Bán hàng".to_string(),
            "".to_string(),
            "10,000,000".to_string(),
        ],
    ];

    let (_, _, _, tk_mapping, _, _) = detect_header_and_mapping(&tk511_rows);
    assert_eq!(tk_mapping.date_column.as_deref(), Some("Ngày ct"));
    assert_eq!(tk_mapping.doc_code_column.as_deref(), Some("Mã ct"));
    assert_eq!(tk_mapping.doc_no_column.as_deref(), Some("Số ct"));
    assert_eq!(tk_mapping.credit_amount_column.as_deref(), Some("Phát sinh có"));
    assert_eq!(tk_mapping.debit_amount_column.as_deref(), Some("Phát sinh nợ"));
    assert_eq!(tk_mapping.total_amount_column, None);
}

#[test]
fn test_02_source_classification_context_safety() {
    let tk511_rows = vec![
        vec!["SỔ CHI TIẾT TÀI KHOẢN 511".to_string()],
        vec!["Tài khoản 5111 - Doanh thu bán hàng".to_string()],
        vec![
            "Ngày ct".to_string(),
            "Mã ct".to_string(),
            "Số ct".to_string(),
            "Phát sinh nợ".to_string(),
            "Phát sinh có".to_string(),
        ],
    ];
    let (_, _, _, _, kind_511, conf_511) =
        detect_header_and_mapping_with_context("TK511", &tk511_rows);
    assert_eq!(kind_511, DataSourceKind::Ledger511);
    assert!(conf_511 >= 0.9);

    let tk3331_rows = vec![
        vec!["SỔ CHI TIẾT TÀI KHOẢN 3331 - THUẾ GTGT PHẢI NỘP".to_string()],
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

    // Generic workbook without account code or context MUST NOT default to Ledger511
    let unknown_rows = vec![
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
    let (_, _, _, _, unknown_kind, conf) =
        detect_header_and_mapping_with_context("Sheet1", &unknown_rows);
    assert_eq!(unknown_kind, DataSourceKind::Custom);
    assert!(conf < 0.5);
}

#[test]
fn test_03_derived_value_audit_origin() {
    let ds_with_total = DataSource {
        id: "src_1".to_string(),
        name: "Source with Total".to_string(),
        file_path: "f1.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: Some("Số HĐ".to_string()),
            total_amount_column: Some("Tổng thanh toán".to_string()),
            ..Default::default()
        },
    };

    let rows1 = vec![
        vec!["Số HĐ".to_string(), "Tổng thanh toán".to_string()],
        vec!["001".to_string(), "100,000,000".to_string()],
    ];
    let recs1 = normalize_data_source_rows(&ds_with_total, &rows1[0], &rows1);
    assert_eq!(recs1.len(), 1);
    assert_eq!(recs1[0].total_amount, dec!(100000000));
    assert_eq!(recs1[0].total_amount_origin, ValueOrigin::Source);

    let ds_derived = DataSource {
        id: "src_2".to_string(),
        name: "Source without Total".to_string(),
        file_path: "f2.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: Some("Số HĐ".to_string()),
            pretax_amount_column: Some("Tiền chưa thuế".to_string()),
            vat_amount_column: Some("Tiền thuế".to_string()),
            total_amount_column: None,
            ..Default::default()
        },
    };

    let rows2 = vec![
        vec!["Số HĐ".to_string(), "Tiền chưa thuế".to_string(), "Tiền thuế".to_string()],
        vec!["001".to_string(), "100,000,000".to_string(), "10,000,000".to_string()],
    ];
    let recs2 = normalize_data_source_rows(&ds_derived, &rows2[0], &rows2);
    assert_eq!(recs2.len(), 1);
    assert_eq!(recs2[0].total_amount, dec!(110000000));
    assert_eq!(recs2[0].total_amount_origin, ValueOrigin::Derived);
}

#[test]
fn test_04_multi_source_positive_exact_matches() {
    let ds_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_511 = DataSource {
        id: "src_511".to_string(),
        name: "Sổ 511".to_string(),
        file_path: "511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_3331 = DataSource {
        id: "src_3331".to_string(),
        name: "Sổ 3331".to_string(),
        file_path: "3331.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger3331,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_131 = DataSource {
        id: "src_131".to_string(),
        name: "Sổ 131".to_string(),
        file_path: "131.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger131,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let rec_inv = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "src_inv".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: Some(dec!(100000000)),
        vat_amount: Some(dec!(10000000)),
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(110000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: None,
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_511 = CanonicalRecord {
        id: "511_1".to_string(),
        source_id: "src_511".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(100000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(100000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_3331 = CanonicalRecord {
        id: "3331_1".to_string(),
        source_id: "src_3331".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(10000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(10000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_131 = CanonicalRecord {
        id: "131_1".to_string(),
        source_id: "src_131".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(110000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: Some(dec!(110000000)),
        credit_amount: None,
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let session = ReconciliationSession {
        session_id: "sess_4sources".to_string(),
        scenario_name: "4-Source Positive".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_511, ds_3331, ds_131],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), vec![rec_inv]);
    map.insert("src_511".to_string(), vec![rec_511]);
    map.insert("src_3331".to_string(), vec![rec_3331]);
    map.insert("src_131".to_string(), vec![rec_131]);

    let res = execute_reconciliation(&session, &map);
    assert_eq!(res.groups.len(), 1);
    let grp = &res.groups[0];

    assert_eq!(grp.status, MatchStatus::MatchedExact);
    assert_eq!(grp.source_breakdowns["src_511"].status, MatchStatus::MatchedExact);
    assert_eq!(grp.source_breakdowns["src_3331"].status, MatchStatus::MatchedExact);
    assert_eq!(grp.source_breakdowns["src_131"].status, MatchStatus::MatchedExact);

    assert_eq!(grp.revenue_variance, Decimal::ZERO);
    assert_eq!(grp.vat_variance, Decimal::ZERO);
    assert_eq!(grp.receivable_variance, Decimal::ZERO);
    assert_eq!(grp.amount_variance, Decimal::ZERO); // MUST NOT be -120M!
}

#[test]
fn test_05_multi_source_negative_missing_secondary() {
    let ds_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_511 = DataSource {
        id: "src_511".to_string(),
        name: "Sổ 511".to_string(),
        file_path: "511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_3331 = DataSource {
        id: "src_3331".to_string(),
        name: "Sổ 3331".to_string(),
        file_path: "3331.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger3331,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_131 = DataSource {
        id: "src_131".to_string(),
        name: "Sổ 131".to_string(),
        file_path: "131.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger131,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let rec_inv = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "src_inv".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: Some(dec!(100000000)),
        vat_amount: Some(dec!(10000000)),
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(110000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: None,
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_511 = CanonicalRecord {
        id: "511_1".to_string(),
        source_id: "src_511".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(100000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(100000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_3331 = CanonicalRecord {
        id: "3331_1".to_string(),
        source_id: "src_3331".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(10000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(10000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    // TK131 is empty / missing record #101!
    let session = ReconciliationSession {
        session_id: "sess_4sources_missing_131".to_string(),
        scenario_name: "4-Source Missing Secondary".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_511, ds_3331, ds_131],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), vec![rec_inv]);
    map.insert("src_511".to_string(), vec![rec_511]);
    map.insert("src_3331".to_string(), vec![rec_3331]);
    map.insert("src_131".to_string(), vec![]); // EMPTY!

    let res = execute_reconciliation(&session, &map);
    assert_eq!(res.groups.len(), 1);
    let grp = &res.groups[0];

    // Assert: TK511=Exact, TK3331=Exact, TK131=Missing, Overall != MatchedExact
    assert_eq!(grp.source_breakdowns["src_511"].status, MatchStatus::MatchedExact);
    assert_eq!(grp.source_breakdowns["src_3331"].status, MatchStatus::MatchedExact);
    assert_eq!(grp.source_breakdowns["src_131"].status, MatchStatus::UnmatchedMissingInTarget);
    assert_ne!(grp.status, MatchStatus::MatchedExact);
    assert_eq!(grp.status, MatchStatus::UnmatchedMissingInTarget);
}

#[test]
fn test_06_multi_source_negative_vat_mismatch() {
    let ds_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_511 = DataSource {
        id: "src_511".to_string(),
        name: "Sổ 511".to_string(),
        file_path: "511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_3331 = DataSource {
        id: "src_3331".to_string(),
        name: "Sổ 3331".to_string(),
        file_path: "3331.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger3331,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let rec_inv = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "src_inv".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: Some(dec!(100000000)),
        vat_amount: Some(dec!(10000000)),
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(110000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: None,
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_511 = CanonicalRecord {
        id: "511_1".to_string(),
        source_id: "src_511".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(100000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(100000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    // VAT is 9.5M (mismatch of 500,000 VND)
    let rec_3331 = CanonicalRecord {
        id: "3331_1".to_string(),
        source_id: "src_3331".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(9500000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(9500000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let session = ReconciliationSession {
        session_id: "sess_vat_mismatch".to_string(),
        scenario_name: "VAT Mismatch".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_511, ds_3331],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), vec![rec_inv]);
    map.insert("src_511".to_string(), vec![rec_511]);
    map.insert("src_3331".to_string(), vec![rec_3331]);

    let res = execute_reconciliation(&session, &map);
    assert_eq!(res.groups.len(), 1);
    let grp = &res.groups[0];

    assert_eq!(grp.source_breakdowns["src_511"].status, MatchStatus::MatchedExact);
    assert_eq!(grp.source_breakdowns["src_3331"].status, MatchStatus::MismatchAmount);
    assert_eq!(grp.status, MatchStatus::MismatchAmount);
    assert_eq!(grp.vat_variance, dec!(500000));
}

#[test]
fn test_07_optional_sources_not_uploaded_still_match_exact() {
    // Scenario requires Invoice + TK511, optional TK3331 and TK131
    // User only uploads Invoice + TK511
    let ds_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds_511 = DataSource {
        id: "src_511".to_string(),
        name: "Sổ 511".to_string(),
        file_path: "511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };

    let rec_inv = CanonicalRecord {
        id: "inv_1".to_string(),
        source_id: "src_inv".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: Some(dec!(100000000)),
        vat_amount: Some(dec!(10000000)),
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(110000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: None,
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let rec_511 = CanonicalRecord {
        id: "511_1".to_string(),
        source_id: "src_511".to_string(),
        source_row: 2,
        date: Some("2026-01-05".to_string()),
        doc_no: Some("101".to_string()),
        doc_code: None,
        series: None,
        template_code: None,
        partner_tax_id: None,
        buyer_tax_id: None,
        seller_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        discount_amount: None,
        fee_amount: None,
        total_amount: dec!(100000000),
        total_amount_origin: ValueOrigin::Source,
        debit_amount: None,
        credit_amount: Some(dec!(100000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let session = ReconciliationSession {
        session_id: "sess_optional_sources".to_string(),
        scenario_name: "Optional sources not uploaded".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        required_source_ids: Some(vec!["src_inv".to_string(), "src_511".to_string()]),
        optional_source_ids: Some(vec!["src_3331".to_string(), "src_131".to_string()]),
        data_sources: vec![ds_inv, ds_511],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), vec![rec_inv]);
    map.insert("src_511".to_string(), vec![rec_511]);

    let res = execute_reconciliation(&session, &map);
    assert_eq!(res.groups.len(), 1);
    assert_eq!(res.groups[0].status, MatchStatus::MatchedExact);
}

#[test]
fn test_08_full_production_auto_pipeline_acceptance() {
    let inv_headers = vec![
        "Ký hiệu mẫu số".to_string(),
        "Ký hiệu hóa đơn".to_string(),
        "Số hóa đơn".to_string(),
        "Ngày lập".to_string(),
        "MST người bán".to_string(),
        "MST người mua".to_string(),
        "Tên người mua".to_string(),
        "Tổng tiền chưa thuế".to_string(),
        "Tổng tiền thuế".to_string(),
        "Tổng tiền chiết khấu thương mại".to_string(),
        "Tổng tiền phí".to_string(),
        "Tổng tiền thanh toán".to_string(),
    ];

    let mut raw_inv_rows = vec![inv_headers.clone()];

    let mut current_sum = Decimal::ZERO;
    for i in 1..=45 {
        let pretax: Decimal = if i == 45 {
            dec!(7223121057) - current_sum
        } else {
            dec!(150000000) + Decimal::from(i * 1000000)
        };
        current_sum += pretax;
        let vat = pretax * dec!(0.10);
        let total = pretax + vat;

        raw_inv_rows.push(vec![
            "1".to_string(),
            "1C26TAA".to_string(),
            format!("{:08}", i),
            "05/01/2026".to_string(),
            "0100000000".to_string(),
            format!("010999{:04}", i),
            format!("CÔNG TY TNHH ĐỐI TÁC {}", i),
            pretax.to_string(),
            vat.to_string(),
            "0".to_string(),
            "0".to_string(),
            total.to_string(),
        ]);
    }
    assert_eq!(current_sum, dec!(7223121057));

    // Row 46: Missing Invoice #233
    raw_inv_rows.push(vec![
        "1".to_string(),
        "1C26TAA".to_string(),
        "00000233".to_string(),
        "06/07/2026".to_string(),
        "0100000000".to_string(),
        "0109990233".to_string(),
        "Công ty Cổ phần Xây Dựng 233".to_string(),
        "105,000,000".to_string(),
        "10,500,000".to_string(),
        "0".to_string(),
        "0".to_string(),
        "115,500,000".to_string(),
    ]);

    // 17 empty/zero amount rows (64 raw rows total: 1 header + 46 data + 17 empty = 64)
    for k in 1..=17 {
        raw_inv_rows.push(vec![
            "1".to_string(),
            "1C26TAA".to_string(),
            format!("DUMMY_{}", k),
            "01/01/2026".to_string(),
            "0100000000".to_string(),
            "".to_string(),
            "Dòng ghi chú không số tiền".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]);
    }
    assert_eq!(raw_inv_rows.len(), 64);

    let tk511_headers = vec![
        "Ngày ct".to_string(),
        "Mã ct".to_string(),
        "Số ct".to_string(),
        "Diễn giải".to_string(),
        "Phát sinh nợ".to_string(),
        "Phát sinh có".to_string(),
    ];

    let mut raw_tk511_rows = vec![tk511_headers.clone()];

    // Subtotal 1
    raw_tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "SỐ DƯ ĐẦU KỲ".to_string(),
        "".to_string(),
        "0".to_string(),
    ]);

    let mut tk511_sum = Decimal::ZERO;
    for i in 1..=45 {
        let pretax: Decimal = if i == 45 {
            dec!(7223121057) - tk511_sum
        } else {
            dec!(150000000) + Decimal::from(i * 1000000)
        };
        tk511_sum += pretax;

        raw_tk511_rows.push(vec![
            "05/01/2026".to_string(),
            "HĐ".to_string(),
            format!("{}", i),
            format!("Bán hàng theo HĐ {}", i),
            "".to_string(),
            pretax.to_string(),
        ]);
    }
    assert_eq!(tk511_sum, dec!(7223121057));

    // Footers
    raw_tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "PHÁT SINH TRONG KỲ".to_string(),
        "".to_string(),
        "7,223,121,057".to_string(),
    ]);
    raw_tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "SỐ DƯ CUỐI KỲ".to_string(),
        "".to_string(),
        "0".to_string(),
    ]);
    assert_eq!(raw_tk511_rows.len(), 49);

    // 3. EXECUTE FULL AUTO PIPELINE (NO MANUAL MAPPINGS)
    let (h_inv, d_inv, cols_inv, map_inv, kind_inv, _) =
        detect_header_and_mapping_with_context("HĐ Bán Ra", &raw_inv_rows);
    let (h_tk, d_tk, cols_tk, map_tk, _, _) =
        detect_header_and_mapping_with_context("TK511 - Doanh thu", &raw_tk511_rows);

    let ds_inv = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn điện tử".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: kind_inv,
        header_row: h_inv,
        data_start_row: d_inv,
        column_mapping: map_inv,
    };

    let ds_tk511 = DataSource {
        id: "src_tk511".to_string(),
        name: "Sổ cái TK 511".to_string(),
        file_path: "tk511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: h_tk,
        data_start_row: d_tk,
        column_mapping: map_tk,
    };

    let norm_inv = normalize_data_source_rows(&ds_inv, &cols_inv, &raw_inv_rows);
    let norm_tk511 = normalize_data_source_rows(&ds_tk511, &cols_tk, &raw_tk511_rows);

    assert_eq!(norm_inv.len(), 46, "Invoice valid records must be exactly 46");
    assert_eq!(norm_tk511.len(), 45, "TK511 valid records must be exactly 45");

    let total_inv_pretax: Decimal = norm_inv.iter().map(|r| r.pretax_amount.unwrap_or(r.total_amount)).sum();
    let total_tk511_credit: Decimal = norm_tk511.iter().map(|r| r.credit_amount.unwrap_or(r.total_amount)).sum();
    assert_eq!(total_inv_pretax, dec!(7328121057));
    assert_eq!(total_tk511_credit, dec!(7223121057));

    let session = ReconciliationSession {
        session_id: "sess_auto_pipeline".to_string(),
        scenario_name: "Auto Pipeline Test".to_string(),
        primary_source_id: Some("src_inv".to_string()),
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![ds_inv, ds_tk511],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_inv".to_string(), norm_inv);
    map.insert("src_tk511".to_string(), norm_tk511);

    let result = execute_reconciliation(&session, &map);

    assert_eq!(result.summary.total_source_records, 46);
    assert_eq!(result.summary.total_target_records, 45);
    assert_eq!(result.summary.exact_matches_count, 45);
    assert_eq!(result.summary.mismatches_count, 0);
    assert_eq!(result.summary.missing_in_target_count, 1);
    assert_eq!(result.summary.missing_in_source_count, 0);
    assert_eq!(result.summary.net_financial_variance, dec!(105000000));

    let missing = result.groups.iter().find(|g| g.status == MatchStatus::UnmatchedMissingInTarget).unwrap();
    assert_eq!(missing.total_source_amount, dec!(105000000));
    assert!(missing.discrepancies.iter().any(|d| d.message.contains("233")));
}

#[test]
fn test_09_real_workbooks_verification_if_present() {
    let inv_path = Path::new("D:/appketoan/T7.2026 Thuế.xlsx");
    let tk_path = Path::new("D:/appketoan/T7.2026.xlsx");

    if inv_path.exists() && tk_path.exists() {
        let meta_inv = inspect_excel_file(inv_path).expect("Read real invoice excel");
        let sheet_inv = &meta_inv.sheets[0];
        let rows_inv = read_sheet_rows(inv_path, &sheet_inv.name).expect("Read invoice rows");

        let meta_tk = inspect_excel_file(tk_path).expect("Read real tk511 excel");
        let sheet_tk = &meta_tk.sheets[0];
        let rows_tk = read_sheet_rows(tk_path, &sheet_tk.name).expect("Read tk rows");

        let (h_inv, d_inv, cols_inv, map_inv, kind_inv, _) =
            detect_header_and_mapping_with_context(&sheet_inv.name, &rows_inv);
        let (h_tk, d_tk, cols_tk, map_tk, _, _) =
            detect_header_and_mapping_with_context(&sheet_tk.name, &rows_tk);

        let ds_inv = DataSource {
            id: "src_real_inv".to_string(),
            name: "Hóa đơn thật".to_string(),
            file_path: inv_path.to_string_lossy().to_string(),
            sheet_name: sheet_inv.name.clone(),
            kind: kind_inv,
            header_row: h_inv,
            data_start_row: d_inv,
            column_mapping: map_inv,
        };

        let ds_tk = DataSource {
            id: "src_real_tk".to_string(),
            name: "Sổ cái thật".to_string(),
            file_path: tk_path.to_string_lossy().to_string(),
            sheet_name: sheet_tk.name.clone(),
            kind: DataSourceKind::Ledger511,
            header_row: h_tk,
            data_start_row: d_tk,
            column_mapping: map_tk,
        };

        let norm_inv = normalize_data_source_rows(&ds_inv, &cols_inv, &rows_inv);
        let norm_tk = normalize_data_source_rows(&ds_tk, &cols_tk, &rows_tk);

        println!("[REAL WORKBOOKS VERIFICATION]");
        println!(" - Invoice valid records: {}", norm_inv.len());
        println!(" - TK511 valid records:   {}", norm_tk.len());

        let session = ReconciliationSession {
            session_id: "sess_real_files".to_string(),
            scenario_name: "Real File Reconciliation".to_string(),
            primary_source_id: Some("src_real_inv".to_string()),
            required_source_ids: None,
            optional_source_ids: None,
            data_sources: vec![ds_inv, ds_tk],
            matching_tolerance_vnd: dec!(10),
            date_tolerance_days: 3,
            enable_aggregate_match: true,
        };

        let mut map = HashMap::new();
        map.insert("src_real_inv".to_string(), norm_inv);
        map.insert("src_real_tk".to_string(), norm_tk);

        let res = execute_reconciliation(&session, &map);
        println!(" - Exact Matches:  {}", res.summary.exact_matches_count);
        println!(" - Mismatches:     {}", res.summary.mismatches_count);
        println!(" - Missing Target: {}", res.summary.missing_in_target_count);
        println!(" - Variance:       {}", res.summary.net_financial_variance);

        assert_eq!(res.summary.total_source_records, 46);
        assert_eq!(res.summary.total_target_records, 45);
        assert_eq!(res.summary.exact_matches_count, 45);
        assert_eq!(res.summary.mismatches_count, 0);
        assert_eq!(res.summary.missing_in_target_count, 1);
        assert_eq!(res.summary.missing_in_source_count, 0);
        assert_eq!(res.summary.net_financial_variance, dec!(105000000));
    }
}

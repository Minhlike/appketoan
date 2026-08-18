use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

use reconciliation_core::{
    detect_header_and_mapping, execute_reconciliation, is_garbage_or_subtotal_row,
    normalize_data_source_rows, CanonicalRecord, ColumnMapping, DataSource, DataSourceKind,
    MatchStatus, ReconciliationSession,
};

#[test]
fn test_01_and_02_tk511_credit_debit_semantic_mapping() {
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
    assert_eq!(mapping.credit_amount_column.as_deref(), Some("Phát sinh có"));
    assert_eq!(mapping.debit_amount_column.as_deref(), Some("Phát sinh nợ"));
    assert_eq!(mapping.total_amount_column, None);
    assert_eq!(kind, DataSourceKind::Ledger511);
}

#[test]
fn test_03_summary_rows_filtered() {
    let summary_rows = vec![
        vec!["SỐ DƯ ĐẦU KỲ".to_string(), "".to_string(), "500,000,000".to_string()],
        vec!["Phát sinh trong kỳ".to_string(), "".to_string(), "7,223,121,057".to_string()],
        vec!["Số dư cuối kỳ".to_string(), "".to_string(), "0".to_string()],
        vec!["Tổng cộng".to_string(), "".to_string(), "7,223,121,057".to_string()],
        vec!["Người lập biểu".to_string(), "Kế toán trưởng".to_string()],
    ];

    for row in summary_rows {
        assert!(
            is_garbage_or_subtotal_row(&row),
            "Row should be identified as garbage/subtotal: {:?}",
            row
        );
    }
}

#[test]
fn test_04_empty_amount_invoice_rows_filtered() {
    let source = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn điện tử".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: Some("Số hóa đơn".to_string()),
            date_column: Some("Ngày lập".to_string()),
            pretax_amount_column: Some("Tổng tiền chưa thuế".to_string()),
            vat_amount_column: Some("Tổng tiền thuế".to_string()),
            total_amount_column: Some("Tổng tiền thanh toán".to_string()),
            ..Default::default()
        },
    };

    let headers = vec![
        "Số hóa đơn".to_string(),
        "Ngày lập".to_string(),
        "Tổng tiền chưa thuế".to_string(),
        "Tổng tiền thuế".to_string(),
        "Tổng tiền thanh toán".to_string(),
    ];

    let rows = vec![
        headers.clone(),
        // Valid row
        vec![
            "00000101".to_string(),
            "05/01/2026".to_string(),
            "10,000,000".to_string(),
            "1,000,000".to_string(),
            "11,000,000".to_string(),
        ],
        // Row with doc number but empty amount -> Must be filtered out
        vec![
            "00000102".to_string(),
            "05/01/2026".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ],
        // Row with doc number and null/0 amount -> Must be filtered out
        vec![
            "00000103".to_string(),
            "05/01/2026".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
        ],
    ];

    let normalized = normalize_data_source_rows(&source, &headers, &rows);
    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].doc_no.as_deref(), Some("101"));
}

#[test]
fn test_05_06_07_08_mandatory_acceptance_regression() {
    // 1. Build synthetic E-Invoices: 46 valid invoices + 18 empty/header dummy rows = 64 total rows
    // Total pretax = 7,328,121,057. Contains Invoice 233 with pretax 105,000,000.
    let mut inv_rows = Vec::new();
    let inv_headers = vec![
        "Số hóa đơn".to_string(),
        "Ngày lập".to_string(),
        "Tổng tiền chưa thuế".to_string(),
        "Tổng tiền thuế".to_string(),
        "Tổng tiền thanh toán".to_string(),
    ];
    inv_rows.push(inv_headers.clone());

    // 45 matching invoices: Base amounts sum to 7,223,121,057
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

        inv_rows.push(vec![
            format!("{:08}", i),
            "05/01/2026".to_string(),
            pretax.to_string(),
            vat.to_string(),
            total.to_string(),
        ]);
    }
    assert_eq!(current_sum, dec!(7223121057));

    // Invoice #46: Invoice 233 (Missing in TK511)
    inv_rows.push(vec![
        "00000233".to_string(),
        "06/07/2026".to_string(),
        "105,000,000".to_string(),
        "10,500,000".to_string(),
        "115,500,000".to_string(),
    ]);

    // Dummy empty amount rows that must be ignored
    for k in 1..=18 {
        inv_rows.push(vec![
            format!("DUMMY_{}", k),
            "01/01/2026".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ]);
    }

    // 2. Build synthetic TK511: 45 matching records + 4 summary rows = 49 total rows
    let mut tk511_rows = Vec::new();
    let tk511_headers = vec![
        "Ngày ct".to_string(),
        "Số ct".to_string(),
        "Diễn giải".to_string(),
        "TK đối ứng".to_string(),
        "Phát sinh nợ".to_string(),
        "Phát sinh có".to_string(),
    ];
    tk511_rows.push(tk511_headers.clone());

    // Summary row 1: SỐ DƯ ĐẦU KỲ
    tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "SỐ DƯ ĐẦU KỲ".to_string(),
        "".to_string(),
        "".to_string(),
        "0".to_string(),
    ]);

    // 45 matching records in TK511
    let mut tk511_sum = Decimal::ZERO;
    for i in 1..=45 {
        let pretax: Decimal = if i == 45 {
            dec!(7223121057) - tk511_sum
        } else {
            dec!(150000000) + Decimal::from(i * 1000000)
        };
        tk511_sum += pretax;

        tk511_rows.push(vec![
            "05/01/2026".to_string(),
            format!("{}", i), // "1", "2", ... (normalized matching against "00000001")
            format!("Doanh thu HĐ {}", i),
            "131".to_string(),
            "".to_string(),
            pretax.to_string(),
        ]);
    }
    assert_eq!(tk511_sum, dec!(7223121057));

    // Summary rows: Phát sinh trong kỳ, Số dư cuối kỳ, Tổng cộng
    tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "PHÁT SINH TRONG KỲ".to_string(),
        "".to_string(),
        "".to_string(),
        "7,223,121,057".to_string(),
    ]);
    tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "SỐ DƯ CUỐI KỲ".to_string(),
        "".to_string(),
        "".to_string(),
        "0".to_string(),
    ]);
    tk511_rows.push(vec![
        "".to_string(),
        "".to_string(),
        "TỔNG CỘNG".to_string(),
        "".to_string(),
        "".to_string(),
        "7,223,121,057".to_string(),
    ]);

    // Normalize Data Sources
    let ds_inv = DataSource {
        id: "src_einvoice".to_string(),
        name: "Hóa đơn điện tử bán ra".to_string(),
        file_path: "einvoice.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: Some("Số hóa đơn".to_string()),
            date_column: Some("Ngày lập".to_string()),
            pretax_amount_column: Some("Tổng tiền chưa thuế".to_string()),
            vat_amount_column: Some("Tổng tiền thuế".to_string()),
            total_amount_column: Some("Tổng tiền thanh toán".to_string()),
            ..Default::default()
        },
    };

    let ds_tk511 = DataSource {
        id: "src_tk511".to_string(),
        name: "Sổ cái TK 511".to_string(),
        file_path: "tk511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            date_column: Some("Ngày ct".to_string()),
            doc_no_column: Some("Số ct".to_string()),
            debit_amount_column: Some("Phát sinh nợ".to_string()),
            credit_amount_column: Some("Phát sinh có".to_string()),
            description_column: Some("Diễn giải".to_string()),
            ..Default::default()
        },
    };

    let norm_inv = normalize_data_source_rows(&ds_inv, &inv_headers, &inv_rows);
    let norm_tk511 = normalize_data_source_rows(&ds_tk511, &tk511_headers, &tk511_rows);

    // Assert Clean Ingestion Filtering
    assert_eq!(norm_inv.len(), 46, "Invoice valid records must equal 46 (not 64)");
    assert_eq!(norm_tk511.len(), 45, "TK511 valid records must equal 45 (not 49)");

    let total_inv_pretax: Decimal = norm_inv.iter().map(|r| r.pretax_amount.unwrap_or(r.total_amount)).sum();
    let total_tk511_credit: Decimal = norm_tk511.iter().map(|r| r.credit_amount.unwrap_or(r.total_amount)).sum();
    assert_eq!(total_inv_pretax, dec!(7328121057), "Total pretax must equal 7,328,121,057");
    assert_eq!(total_tk511_credit, dec!(7223121057), "Total credit must equal 7,223,121,057");

    // Execute Reconciliation
    let session = ReconciliationSession {
        session_id: "sess_regression".to_string(),
        scenario_name: "Đối chiếu Doanh thu & Sổ cái TK 511".to_string(),
        data_sources: vec![ds_inv, ds_tk511],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_einvoice".to_string(), norm_inv);
    map.insert("src_tk511".to_string(), norm_tk511);

    let result = execute_reconciliation(&session, &map);

    // Verify Mandatory Acceptance Metrics
    assert_eq!(result.summary.total_source_records, 46);
    assert_eq!(result.summary.total_target_records, 45);
    assert_eq!(result.summary.exact_matches_count, 45);
    assert_eq!(result.summary.mismatches_count, 0, "No amount mismatches allowed");
    assert_eq!(result.summary.missing_in_target_count, 1, "Exactly 1 missing in TK511");
    assert_eq!(result.summary.missing_in_source_count, 0, "0 missing in invoice");
    assert_eq!(result.summary.net_financial_variance, dec!(105000000));

    // Verify Missing Invoice 233 Details
    let missing_group = result
        .groups
        .iter()
        .find(|g| g.status == MatchStatus::UnmatchedMissingInTarget)
        .expect("Missing in target group must exist");

    assert_eq!(missing_group.total_source_amount, dec!(105000000));
    assert!(missing_group.discrepancies.iter().any(|d| d.message.contains("233")));
}

#[test]
fn test_09_no_false_matching_for_same_amount_different_doc() {
    let source1 = DataSource {
        id: "src_1".to_string(),
        name: "Source 1".to_string(),
        file_path: "s1.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: Some("Số HĐ".to_string()),
            total_amount_column: Some("Số tiền".to_string()),
            ..Default::default()
        },
    };

    let source2 = DataSource {
        id: "src_2".to_string(),
        name: "Source 2".to_string(),
        file_path: "s2.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping {
            doc_no_column: Some("Số CT".to_string()),
            credit_amount_column: Some("Phát sinh Có".to_string()),
            ..Default::default()
        },
    };

    let rec_a = CanonicalRecord {
        id: "a1".to_string(),
        source_id: "src_1".to_string(),
        source_row: 2,
        date: Some("2026-01-10".to_string()),
        doc_no: Some("100".to_string()),
        series: None,
        template_code: None,
        partner_tax_id: Some("0101234567".to_string()),
        partner_name: None,
        pretax_amount: Some(dec!(50000000)),
        vat_amount: None,
        total_amount: dec!(50000000),
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

    let rec_b = CanonicalRecord {
        id: "b1".to_string(),
        source_id: "src_2".to_string(),
        source_row: 2,
        date: Some("2026-01-10".to_string()),
        doc_no: Some("999".to_string()), // Different doc number!
        series: None,
        template_code: None,
        partner_tax_id: Some("0101234567".to_string()), // Same tax id & same amount
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        total_amount: dec!(50000000),
        debit_amount: None,
        credit_amount: Some(dec!(50000000)),
        vat_rate: None,
        debit_account: None,
        credit_account: None,
        voucher_no: None,
        description: None,
        bank_account: None,
        raw_fields: HashMap::new(),
    };

    let session = ReconciliationSession {
        session_id: "sess_no_false_match".to_string(),
        scenario_name: "Test No False Match".to_string(),
        data_sources: vec![source1, source2],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut map = HashMap::new();
    map.insert("src_1".to_string(), vec![rec_a]);
    map.insert("src_2".to_string(), vec![rec_b]);

    let res = execute_reconciliation(&session, &map);

    // MUST NOT match them together because doc_no is different!
    assert_eq!(res.summary.exact_matches_count, 0);
    assert_eq!(res.summary.tolerance_matches_count, 0);
    assert_eq!(res.summary.missing_in_target_count, 1);
    assert_eq!(res.summary.missing_in_source_count, 1);
}

#[test]
fn test_10_multi_source_identity_preserved() {
    let ds1 = DataSource {
        id: "src_inv".to_string(),
        name: "Hóa đơn".to_string(),
        file_path: "inv.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::EInvoice,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds2 = DataSource {
        id: "src_511".to_string(),
        name: "Sổ 511".to_string(),
        file_path: "511.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind: DataSourceKind::Ledger511,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    };
    let ds3 = DataSource {
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
        series: None,
        template_code: None,
        partner_tax_id: None,
        partner_name: None,
        pretax_amount: Some(dec!(100000000)),
        vat_amount: Some(dec!(10000000)),
        total_amount: dec!(110000000),
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
        series: None,
        template_code: None,
        partner_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        total_amount: dec!(100000000),
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
        series: None,
        template_code: None,
        partner_tax_id: None,
        partner_name: None,
        pretax_amount: None,
        vat_amount: None,
        total_amount: dec!(10000000),
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

    let session = ReconciliationSession {
        session_id: "sess_3sources".to_string(),
        scenario_name: "3-Source Reconciliation".to_string(),
        data_sources: vec![ds1, ds2, ds3],
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
    assert!(grp.source_breakdowns.contains_key("src_511"));
    assert!(grp.source_breakdowns.contains_key("src_3331"));
    assert_eq!(grp.source_breakdowns["src_511"].compared_amount, dec!(100000000));
    assert_eq!(grp.source_breakdowns["src_3331"].compared_amount, dec!(10000000));
}

use reconciliation_core::*;
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[test]
fn test_end_to_end_reconciliation_engine_flow() {
    let e_invoices_json =
        include_str!("../../../fixtures/synthetic/einvoices_comprehensive_synthetic.json");
    let ledger_511_json =
        include_str!("../../../fixtures/synthetic/ledger_511_comprehensive_synthetic.json");

    let e_invoices: Vec<CanonicalRecord> =
        serde_json::from_str(e_invoices_json).expect("Failed to parse e-invoices");
    let ledger_511: Vec<CanonicalRecord> =
        serde_json::from_str(ledger_511_json).expect("Failed to parse ledger 511");

    let session = ReconciliationSession {
        session_id: "sess_e2e_test_01".to_string(),
        scenario_name: "Đối chiếu Doanh thu & Thuế đầu ra (3 nguồn)".to_string(),
        primary_source_id: Some("src_e_invoice".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: vec![
            DataSource {
                id: "src_e_invoice".to_string(),
                name: "Bảng kê HĐĐT Bán ra".to_string(),
                file_path: "mock_einvoices.xlsx".to_string(),
                sheet_name: "Sheet1".to_string(),
                kind: DataSourceKind::EInvoice,
                role: SourceRole::Primary,
                header_row: 1,
                data_start_row: 2,
                column_mapping: ColumnMapping::default(),
            },
            DataSource {
                id: "src_ledger_511".to_string(),
                name: "Sổ chi tiết TK 511".to_string(),
                file_path: "mock_ledger511.xlsx".to_string(),
                sheet_name: "Sheet1".to_string(),
                kind: DataSourceKind::Ledger511,
                role: SourceRole::RequiredSecondary,
                header_row: 1,
                data_start_row: 2,
                column_mapping: ColumnMapping::default(),
            },
        ],
        comparison_rules: vec![],
        matching_tolerance_vnd: dec!(10),
        date_tolerance_days: 3,
        enable_aggregate_match: true,
    };

    let mut source_map = HashMap::new();
    source_map.insert("src_e_invoice".to_string(), e_invoices);
    source_map.insert("src_ledger_511".to_string(), ledger_511);

    let result =
        execute_reconciliation(&session, &source_map).expect("Reconciliation should succeed");

    // Assert summary metrics
    assert_eq!(result.summary.total_source_records, 7);
    assert_eq!(result.summary.total_target_records, 6);
    assert!(result.summary.exact_matches_count >= 1);

    // Test Excel Export
    let temp_export_path = std::env::temp_dir().join("test_reconciliation_report.xlsx");
    let export_res = export_reconciliation_to_excel(&result, &temp_export_path);
    assert!(
        export_res.is_ok(),
        "Excel export failed: {:?}",
        export_res.err()
    );

    let summary = export_res.unwrap();
    assert!(summary.file_size_bytes > 1000);
    assert_eq!(summary.total_groups_exported, result.groups.len());

    // Verify exported Excel can be opened by Calamine reader
    let inspected = inspect_excel_file(&temp_export_path);
    assert!(
        inspected.is_ok(),
        "Failed to read exported Excel: {:?}",
        inspected.err()
    );
    let meta = inspected.unwrap();
    assert_eq!(meta.sheets.len(), 3);
    assert_eq!(meta.sheets[0].name, "Tong quan");
    assert_eq!(meta.sheets[1].name, "Sai lech & Can chu y");
    assert_eq!(meta.sheets[2].name, "Chi tiet tat ca");

    let _ = std::fs::remove_file(temp_export_path);
}

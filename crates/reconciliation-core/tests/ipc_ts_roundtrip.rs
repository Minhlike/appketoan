use std::collections::HashMap;
use std::path::Path;

use reconciliation_core::{
    execute_reconciliation, CanonicalRecord, DataSourceKind, MatchStatus, ReconciliationSession,
};
use rust_decimal_macros::dec;

#[test]
fn test_ipc_ts_roundtrip_file_exists_and_deserializes() {
    let candidate_paths = [
        "audit-runtime/generated-ts-ipc.json",
        "../../audit-runtime/generated-ts-ipc.json",
        "fixtures/artifacts/ts_generated_session.json",
        "../../fixtures/artifacts/ts_generated_session.json",
    ];

    let found_path = candidate_paths.iter().map(Path::new).find(|p| p.exists());

    assert!(
        found_path.is_some(),
        "FATAL: TS-generated IPC file not found in any of {:?}. Run `npm test` first to generate it.",
        candidate_paths
    );

    let file_path = found_path.unwrap();
    let content = std::fs::read_to_string(file_path).expect("Failed to read TS generated IPC JSON");
    let session: ReconciliationSession = serde_json::from_str(&content)
        .expect("Failed to deserialize TS generated session into Rust struct");

    assert_eq!(session.session_id, "sess_ts_to_rust_roundtrip");
    assert_eq!(session.matching_tolerance_vnd, dec!(50000));
    assert_eq!(session.date_tolerance_days, 5);
    assert_eq!(session.data_sources.len(), 9);

    // Verify all 9 kinds deserialized correctly
    assert_eq!(session.data_sources[0].kind, DataSourceKind::EInvoice);
    assert_eq!(session.data_sources[1].kind, DataSourceKind::Ledger511);
    assert_eq!(session.data_sources[2].kind, DataSourceKind::Ledger3331);
    assert_eq!(session.data_sources[3].kind, DataSourceKind::Ledger131);
    assert_eq!(session.data_sources[4].kind, DataSourceKind::Ledger133);
    assert_eq!(session.data_sources[5].kind, DataSourceKind::BankStatement);
    assert_eq!(session.data_sources[6].kind, DataSourceKind::CashBook);
    assert_eq!(session.data_sources[7].kind, DataSourceKind::BranchLedger);
    assert_eq!(session.data_sources[8].kind, DataSourceKind::Custom);

    // Verify comparison rules
    assert_eq!(session.comparison_rules.len(), 8);
    assert_eq!(session.comparison_rules[0].primary_field, "pretaxAmount");
    assert_eq!(session.comparison_rules[0].secondary_field, "creditAmount");
    assert_eq!(session.comparison_rules[0].tolerance_vnd, dec!(0));

    // Test execution with this session
    let mut records_map = HashMap::new();
    for ds in &session.data_sources {
        records_map.insert(ds.id.clone(), vec![]);
    }
    records_map.insert(
        session.data_sources[0].id.clone(),
        vec![CanonicalRecord {
            id: "rec_ts_1".to_string(),
            source_id: session.data_sources[0].id.clone(),
            source_row: 2,
            doc_no: Some("INV-001".to_string()),
            pretax_amount: Some(dec!(1000000)),
            vat_amount: Some(dec!(100000)),
            total_amount: dec!(1100000),
            ..Default::default()
        }],
    );
    records_map.insert(
        session.data_sources[1].id.clone(),
        vec![CanonicalRecord {
            id: "rec_ts_2".to_string(),
            source_id: session.data_sources[1].id.clone(),
            source_row: 2,
            doc_no: Some("INV-001".to_string()),
            credit_amount: Some(dec!(1000000)),
            total_amount: dec!(1000000),
            ..Default::default()
        }],
    );

    let res = execute_reconciliation(&session, &records_map)
        .expect("Reconciliation execution should succeed");
    assert_eq!(res.groups.len(), 1);
    assert_eq!(res.groups[0].status, MatchStatus::MatchedWithMissingSource);
    assert_eq!(
        res.groups[0].target_source_record_ids,
        vec!["rec_ts_2".to_string()]
    );
    assert_eq!(res.groups[0].total_source_amount, dec!(1000000));
    assert_eq!(res.summary.missing_in_target_count, 1);
}

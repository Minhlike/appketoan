use reconciliation_core::models::*;

#[test]
fn test_golden_dataset_schema_and_integrity() {
    let e_invoices_json = include_str!("../../../fixtures/synthetic/einvoices_comprehensive_synthetic.json");
    let ledger_511_json = include_str!("../../../fixtures/synthetic/ledger_511_comprehensive_synthetic.json");
    let golden_result_json = include_str!("../../../fixtures/expected/golden_comprehensive_reconciliation_result.json");

    let e_invoices: Vec<CanonicalRecord> =
        serde_json::from_str(e_invoices_json).expect("Failed to parse synthetic e-invoices");
    assert_eq!(e_invoices.len(), 7);

    let ledger_511: Vec<CanonicalRecord> =
        serde_json::from_str(ledger_511_json).expect("Failed to parse synthetic ledger 511");
    assert_eq!(ledger_511.len(), 6);

    let golden_result: ReconciliationResult =
        serde_json::from_str(golden_result_json).expect("Failed to parse golden result");
    assert_eq!(golden_result.groups.len(), 7);
    assert_eq!(golden_result.summary.exact_matches_count, 1);
    assert_eq!(golden_result.summary.mismatches_count, 1);
    assert_eq!(golden_result.summary.aggregate_matches_count, 1);
    assert_eq!(golden_result.summary.tolerance_matches_count, 1);
    assert_eq!(golden_result.summary.missing_in_target_count, 1);
    assert_eq!(golden_result.summary.missing_in_source_count, 1);
    assert_eq!(golden_result.summary.duplicates_count, 2);
}

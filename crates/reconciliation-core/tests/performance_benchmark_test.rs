use reconciliation_core::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::time::Instant;

fn generate_synthetic_dataset(count: usize, source_id: &str) -> Vec<CanonicalRecord> {
    let mut records = Vec::with_capacity(count);
    for i in 1..=count {
        let pretax = Decimal::from(i * 1000);
        let vat = Decimal::from(i * 100);
        let total = Decimal::from(i * 1100);
        records.push(CanonicalRecord {
            id: format!("{}_row_{}", source_id, i),
            source_id: source_id.to_string(),
            source_row: (i + 1) as u32,
            date: Some(format!("2026-01-{:02}", (i % 28) + 1)),
            doc_no: Some(format!("{:07}", i)),
            doc_code: None,
            series: Some("1C26TAA".to_string()),
            template_code: Some("1".to_string()),
            partner_tax_id: Some(format!("010{:07}", i % 500)),
            buyer_tax_id: Some(format!("010{:07}", i % 500)),
            seller_tax_id: None,
            partner_name: Some(format!("Công ty Thử Nghiệm {}", i % 500)),
            pretax_amount: Some(pretax),
            vat_amount: Some(vat),
            discount_amount: None,
            fee_amount: None,
            total_amount: total,
            total_amount_origin: ValueOrigin::Source,
            debit_amount: None,
            credit_amount: Some(pretax),
            vat_rate: Some("10%".to_string()),
            debit_account: Some("131".to_string()),
            credit_account: Some("5111".to_string()),
            voucher_no: Some(format!("PKT-{:06}", i)),
            description: Some(format!("Giao dịch số {}", i)),
            bank_account: None,
            raw_fields: HashMap::new(),
        });
    }
    records
}

#[test]
fn test_performance_scaling_1k_to_100k() {
    let scales = [1_000, 10_000, 50_000, 100_000];

    for &count in &scales {
        let gen_start = Instant::now();
        let src_a = generate_synthetic_dataset(count, "src_a");
        let src_b = generate_synthetic_dataset(count, "src_b");
        let _gen_dur = gen_start.elapsed();

        let session = ReconciliationSession {
            session_id: format!("perf_test_{}", count),
            scenario_name: "Benchmark Performance".to_string(),
            primary_source_id: Some("src_a".to_string()),
            expected_primary_kind: None,
            required_source_ids: None,
            optional_source_ids: None,
            data_sources: vec![
                DataSource {
                    id: "src_a".to_string(),
                    name: "Nguồn A".to_string(),
                    file_path: "a.xlsx".to_string(),
                    sheet_name: "Sheet1".to_string(),
                    kind: DataSourceKind::EInvoice,
                    role: SourceRole::Primary,
                    header_row: 1,
                    data_start_row: 2,
                    column_mapping: ColumnMapping::default(),
                },
                DataSource {
                    id: "src_b".to_string(),
                    name: "Nguồn B".to_string(),
                    file_path: "b.xlsx".to_string(),
                    sheet_name: "Sheet1".to_string(),
                    kind: DataSourceKind::Ledger511,
                    role: SourceRole::RequiredSecondary,
                    header_row: 1,
                    data_start_row: 2,
                    column_mapping: ColumnMapping::default(),
                },
            ],
            comparison_rules: vec![ComparisonRule {
                id: "rule_perf_rev".to_string(),
                name: "Doanh thu".to_string(),
                semantic: ComparisonSemantic::Revenue,
                primary_source_kind: DataSourceKind::EInvoice,
                primary_field: "pretaxAmount".to_string(),
                secondary_source_kind: DataSourceKind::Ledger511,
                secondary_field: "creditAmount".to_string(),
                is_required: true,
                tolerance_vnd: dec!(1),
                date_tolerance_days: 0,
            }],
            matching_tolerance_vnd: dec!(1),
            date_tolerance_days: 0,
            enable_aggregate_match: false,
        };

        let mut source_map = HashMap::new();
        source_map.insert("src_a".to_string(), src_a);
        source_map.insert("src_b".to_string(), src_b);

        let match_start = Instant::now();
        let result = execute_reconciliation(&session, &source_map).expect("Should succeed");
        let match_dur = match_start.elapsed();

        println!(
            "[PERF BENCHMARK] Count: {:6} pairs | Time: {:8.2?} | Exact Matches: {}",
            count, match_dur, result.summary.exact_matches_count
        );

        assert_eq!(result.summary.exact_matches_count, count);

        if count == 100_000 {
            // Informational timing check for unoptimized debug test suite (Release is < 150ms)
            assert!(
                match_dur.as_secs_f64() < 10.0,
                "100k matching exceeded 10.0s threshold: {:?}",
                match_dur
            );
        }
    }
}

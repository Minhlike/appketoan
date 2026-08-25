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
            partner_code: None,
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
            transaction_number: None,
            accounting_date: None,
            transaction_date: None,
            counterparty_account: None,
            counterparty_name: None,
            balance: None,
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
    }
}

fn perf_source(id: &str, kind: DataSourceKind, role: SourceRole) -> DataSource {
    DataSource {
        id: id.to_string(),
        name: id.to_string(),
        file_path: "synthetic.xlsx".to_string(),
        sheet_name: "Sheet1".to_string(),
        kind,
        role,
        header_row: 1,
        data_start_row: 2,
        column_mapping: ColumnMapping::default(),
    }
}

fn perf_rule(
    id: &str,
    semantic: ComparisonSemantic,
    secondary_kind: DataSourceKind,
    primary_field: &str,
    secondary_field: &str,
) -> ComparisonRule {
    ComparisonRule {
        id: id.to_string(),
        name: id.to_string(),
        semantic,
        primary_source_kind: DataSourceKind::EInvoice,
        primary_field: primary_field.to_string(),
        secondary_source_kind: secondary_kind,
        secondary_field: secondary_field.to_string(),
        is_required: true,
        tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
    }
}

fn run_100k_multi_control(
    name: &str,
    sources: Vec<DataSource>,
    rules: Vec<ComparisonRule>,
    records: HashMap<String, Vec<CanonicalRecord>>,
) {
    let session = ReconciliationSession {
        session_id: name.to_string(),
        scenario_name: name.to_string(),
        primary_source_id: Some("invoice".to_string()),
        expected_primary_kind: None,
        required_source_ids: None,
        optional_source_ids: None,
        data_sources: sources,
        comparison_rules: rules,
        matching_tolerance_vnd: Decimal::ZERO,
        date_tolerance_days: 0,
        enable_aggregate_match: false,
    };
    let started = Instant::now();
    let result = execute_reconciliation(&session, &records).expect("benchmark execution");
    println!(
        "[PERF BENCHMARK] {name} | elapsed={:?} | exact={}",
        started.elapsed(),
        result.summary.exact_matches_count
    );
    assert_eq!(result.summary.exact_matches_count, 100_000);
}

#[test]
fn benchmark_100k_invoice_sales_register_three_semantics() {
    run_100k_multi_control(
        "100k invoice-sales-register 3 controls",
        vec![
            perf_source("invoice", DataSourceKind::EInvoice, SourceRole::Primary),
            perf_source(
                "register",
                DataSourceKind::SalesRegister,
                SourceRole::RequiredSecondary,
            ),
        ],
        vec![
            perf_rule(
                "revenue",
                ComparisonSemantic::Revenue,
                DataSourceKind::SalesRegister,
                "pretaxAmount",
                "pretaxAmount",
            ),
            perf_rule(
                "vat",
                ComparisonSemantic::Vat,
                DataSourceKind::SalesRegister,
                "vatAmount",
                "vatAmount",
            ),
            perf_rule(
                "receivable",
                ComparisonSemantic::Receivable,
                DataSourceKind::SalesRegister,
                "totalAmount",
                "totalAmount",
            ),
        ],
        HashMap::from([
            (
                "invoice".to_string(),
                generate_synthetic_dataset(100_000, "invoice"),
            ),
            (
                "register".to_string(),
                generate_synthetic_dataset(100_000, "register"),
            ),
        ]),
    );
}

#[test]
fn benchmark_100k_tri_source_four_controls() {
    run_100k_multi_control(
        "100k invoice-sales-register-ledger 4 controls",
        vec![
            perf_source("invoice", DataSourceKind::EInvoice, SourceRole::Primary),
            perf_source(
                "register",
                DataSourceKind::SalesRegister,
                SourceRole::RequiredSecondary,
            ),
            perf_source(
                "ledger",
                DataSourceKind::Ledger511,
                SourceRole::RequiredSecondary,
            ),
        ],
        vec![
            perf_rule(
                "register-revenue",
                ComparisonSemantic::Revenue,
                DataSourceKind::SalesRegister,
                "pretaxAmount",
                "pretaxAmount",
            ),
            perf_rule(
                "register-vat",
                ComparisonSemantic::Vat,
                DataSourceKind::SalesRegister,
                "vatAmount",
                "vatAmount",
            ),
            perf_rule(
                "register-receivable",
                ComparisonSemantic::Receivable,
                DataSourceKind::SalesRegister,
                "totalAmount",
                "totalAmount",
            ),
            perf_rule(
                "ledger-revenue",
                ComparisonSemantic::Revenue,
                DataSourceKind::Ledger511,
                "pretaxAmount",
                "creditAmount",
            ),
        ],
        HashMap::from([
            (
                "invoice".to_string(),
                generate_synthetic_dataset(100_000, "invoice"),
            ),
            (
                "register".to_string(),
                generate_synthetic_dataset(100_000, "register"),
            ),
            (
                "ledger".to_string(),
                generate_synthetic_dataset(100_000, "ledger"),
            ),
        ]),
    );
}

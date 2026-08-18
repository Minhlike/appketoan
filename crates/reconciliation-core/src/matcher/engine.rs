use chrono::{NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

use crate::analyzer::discrepancy_analyzer::{analyze_pair_discrepancies, format_vnd};
use crate::models::{
    CanonicalRecord, ComparisonRule, ComparisonSemantic, DataSourceKind, FieldDiscrepancy,
    MatchGroup, MatchStatus, ReconciliationResult, ReconciliationSession, ReconciliationSummary,
    SemanticFieldComparison, SourceMatchBreakdown, SourceRole,
};

/// Resolves pairwise accounting comparison amounts and comparison semantic dynamically
pub fn resolve_pair_comparison_amounts(
    primary: &CanonicalRecord,
    primary_kind: &DataSourceKind,
    secondary: &CanonicalRecord,
    secondary_kind: &DataSourceKind,
    rules: &[ComparisonRule],
    session_tolerance_vnd: Decimal,
    session_date_tolerance_days: u32,
) -> (
    Decimal,
    Decimal,
    ComparisonSemantic,
    &'static str,
    Decimal,
    u32,
) {
    // 1. Check dynamic rules first
    if let Some(rule) = rules.iter().find(|r| {
        &r.primary_source_kind == primary_kind && &r.secondary_source_kind == secondary_kind
    }) {
        let pri_amt = extract_field_amount(primary, &rule.primary_field);
        let sec_amt = extract_field_amount(secondary, &rule.secondary_field);
        let eff_amt_tol = if !rule.tolerance_vnd.is_zero() {
            rule.tolerance_vnd
        } else {
            session_tolerance_vnd
        };
        let eff_date_tol = if rule.date_tolerance_days > 0 {
            rule.date_tolerance_days
        } else {
            session_date_tolerance_days
        };

        return (
            pri_amt,
            sec_amt,
            rule.semantic,
            match rule.semantic {
                ComparisonSemantic::Revenue => "Doanh thu (Pretax ↔ TK511 Phát sinh Có)",
                ComparisonSemantic::Vat => "Thuế GTGT (VAT ↔ TK3331/133 Phát sinh)",
                ComparisonSemantic::Receivable => "Công nợ phải thu (Total ↔ TK131 Phát sinh Nợ)",
                ComparisonSemantic::BankPayment => "Dòng tiền sao kê (Total ↔ Bank Phát sinh Có)",
                ComparisonSemantic::Other => "Đối chiếu số tiền",
            },
            eff_amt_tol,
            eff_date_tol,
        );
    }

    // 2. Built-in defaults by source kind
    let eff_amt_tol = session_tolerance_vnd;
    let eff_date_tol = session_date_tolerance_days;

    match (primary_kind, secondary_kind) {
        (DataSourceKind::EInvoice, DataSourceKind::Ledger511) => (
            primary.pretax_amount.unwrap_or(primary.total_amount),
            secondary
                .credit_amount
                .unwrap_or_else(|| secondary.pretax_amount.unwrap_or(secondary.total_amount)),
            ComparisonSemantic::Revenue,
            "Doanh thu (Pretax ↔ TK511 Phát sinh Có)",
            eff_amt_tol,
            eff_date_tol,
        ),
        (DataSourceKind::EInvoice, DataSourceKind::Ledger3331) => (
            primary.vat_amount.unwrap_or(primary.total_amount),
            secondary
                .credit_amount
                .unwrap_or_else(|| secondary.vat_amount.unwrap_or(secondary.total_amount)),
            ComparisonSemantic::Vat,
            "Thuế GTGT đầu ra (VAT ↔ TK3331 Phát sinh Có)",
            eff_amt_tol,
            eff_date_tol,
        ),
        (DataSourceKind::EInvoice, DataSourceKind::Ledger131) => (
            primary.total_amount,
            secondary.debit_amount.unwrap_or(secondary.total_amount),
            ComparisonSemantic::Receivable,
            "Công nợ phải thu (Total ↔ TK131 Phát sinh Nợ)",
            eff_amt_tol,
            eff_date_tol,
        ),
        (DataSourceKind::EInvoice, DataSourceKind::Ledger133) => (
            primary.vat_amount.unwrap_or(primary.total_amount),
            secondary.debit_amount.unwrap_or(secondary.total_amount),
            ComparisonSemantic::Vat,
            "Thuế GTGT đầu vào (VAT ↔ TK133 Phát sinh Nợ)",
            eff_amt_tol,
            eff_date_tol,
        ),
        (DataSourceKind::EInvoice, DataSourceKind::BankStatement) => (
            primary.total_amount,
            secondary.credit_amount.unwrap_or(secondary.total_amount),
            ComparisonSemantic::BankPayment,
            "Dòng tiền sao kê (Total ↔ Bank Phát sinh Có)",
            eff_amt_tol,
            eff_date_tol,
        ),
        (DataSourceKind::Ledger131, DataSourceKind::BankStatement) => (
            primary.credit_amount.unwrap_or(primary.total_amount),
            secondary.credit_amount.unwrap_or(secondary.total_amount),
            ComparisonSemantic::BankPayment,
            "Dòng tiền thu hồi công nợ (TK131 Có ↔ Bank Có)",
            eff_amt_tol,
            eff_date_tol,
        ),
        _ => {
            let pri = if let Some(pre) = primary.pretax_amount {
                pre
            } else {
                primary.total_amount
            };
            let sec = if let Some(cred) = secondary.credit_amount {
                cred
            } else if let Some(deb) = secondary.debit_amount {
                deb
            } else if let Some(pre) = secondary.pretax_amount {
                pre
            } else {
                secondary.total_amount
            };
            (
                pri,
                sec,
                ComparisonSemantic::Other,
                "Đối chiếu số tiền",
                eff_amt_tol,
                eff_date_tol,
            )
        }
    }
}

fn extract_field_amount(record: &CanonicalRecord, field_name: &str) -> Decimal {
    match field_name {
        "pretaxAmount" | "pretax_amount" => record.pretax_amount.unwrap_or(record.total_amount),
        "vatAmount" | "vat_amount" => record.vat_amount.unwrap_or(Decimal::ZERO),
        "totalAmount" | "total_amount" => record.total_amount,
        "debitAmount" | "debit_amount" => record.debit_amount.unwrap_or(record.total_amount),
        "creditAmount" | "credit_amount" => record.credit_amount.unwrap_or(record.total_amount),
        _ => record.total_amount,
    }
}

fn is_date_within_tolerance(d1: Option<&str>, d2: Option<&str>, tolerance_days: u32) -> bool {
    match (d1, d2) {
        (Some(s1), Some(s2)) => {
            if let (Ok(date1), Ok(date2)) = (
                NaiveDate::parse_from_str(s1, "%Y-%m-%d"),
                NaiveDate::parse_from_str(s2, "%Y-%m-%d"),
            ) {
                let diff = (date1 - date2).num_days().abs();
                diff <= (tolerance_days as i64)
            } else {
                true // If parse fails, don't hard reject at prefilter, let analyzer report discrepancy
            }
        }
        _ => true,
    }
}

/// Checks if primary and candidate counterparties (MST) are compatible
pub fn is_counterparty_compatible(
    primary_tax_id: Option<&str>,
    candidate_tax_id: Option<&str>,
) -> bool {
    match (primary_tax_id, candidate_tax_id) {
        (Some(p), Some(c)) => {
            let norm_p = CanonicalRecord::normalize_tax_id(p);
            let norm_c = CanonicalRecord::normalize_tax_id(c);
            if !norm_p.is_empty() && !norm_c.is_empty() {
                norm_p == norm_c
            } else {
                true
            }
        }
        _ => true,
    }
}

/// Core Deterministic Multi-Source Reconciliation Engine with Full Pre-Execution Validation
pub fn execute_reconciliation(
    session: &ReconciliationSession,
    source_records_map: &HashMap<String, Vec<CanonicalRecord>>,
) -> Result<ReconciliationResult, String> {
    if session.data_sources.len() < 2 {
        return Err("Cần ít nhất 2 nguồn dữ liệu Excel để thực hiện đối chiếu.".to_string());
    }

    let default_tolerance_vnd = session.matching_tolerance_vnd;
    let default_date_tolerance_days = session.date_tolerance_days;
    let allow_aggregate = session.enable_aggregate_match;
    let empty_vec = Vec::new();

    // 1. Identify & Validate Primary Source
    let primary_source = if let Some(ref pri_id) = session.primary_source_id {
        session.data_sources.iter().find(|s| &s.id == pri_id)
    } else {
        session
            .data_sources
            .iter()
            .find(|s| s.role == SourceRole::Primary)
            .or_else(|| {
                session
                    .data_sources
                    .iter()
                    .find(|s| s.kind == DataSourceKind::EInvoice)
            })
            .or_else(|| session.data_sources.first())
    };

    let primary_source = primary_source
        .ok_or_else(|| "Không tìm thấy nguồn dữ liệu chính (PRIMARY).".to_string())?;

    // Validate expected primary kind if scenario specified it
    if let Some(ref exp_kind) = session.expected_primary_kind {
        if &primary_source.kind != exp_kind {
            return Err(format!(
                "Nguồn chính ({}) không đúng loại dữ liệu kịch bản yêu cầu (Kỳ vọng: {}, Thực tế: {}).",
                primary_source.name,
                exp_kind.display_name(),
                primary_source.kind.display_name()
            ));
        }
    }

    let primary_source_id = primary_source.id.as_str();
    let primary_source_name = primary_source.name.as_str();
    let primary_kind = &primary_source.kind;

    let primary_records = source_records_map
        .get(primary_source_id)
        .unwrap_or(&empty_vec);

    // 2. Validate Required Secondary Sources Existence
    if let Some(ref req_ids) = session.required_source_ids {
        for req_id in req_ids {
            if req_id == primary_source_id {
                continue;
            }
            let is_present = session.data_sources.iter().any(|s| &s.id == req_id)
                && source_records_map.contains_key(req_id);
            if !is_present {
                return Err(format!(
                    "Thiếu nguồn dữ liệu bắt buộc (ID: '{}') theo yêu cầu của kịch bản.",
                    req_id
                ));
            }
        }
    }

    // Secondary sources list
    let secondary_sources: Vec<_> = session
        .data_sources
        .iter()
        .filter(|s| s.id != primary_source_id)
        .collect();

    let mut consumed_primary_ids = HashSet::new();
    let mut consumed_secondary_ids = HashSet::new();
    let mut groups: Vec<MatchGroup> = Vec::new();

    // -------------------------------------------------------------
    // PASS 0: Duplicate Detection within primary source
    // -------------------------------------------------------------
    let mut primary_doc_counts: HashMap<(String, String), Vec<&CanonicalRecord>> = HashMap::new();
    let mut primary_doc_no_frequencies: HashMap<String, usize> = HashMap::new();

    for rec in primary_records {
        if let Some(doc) = &rec.doc_no {
            let clean_doc = CanonicalRecord::normalize_doc_no(doc);
            if !clean_doc.is_empty() {
                let series = rec.series.as_deref().unwrap_or("").to_string();
                primary_doc_counts
                    .entry((series, clean_doc.clone()))
                    .or_default()
                    .push(rec);
                *primary_doc_no_frequencies.entry(clean_doc).or_default() += 1;
            }
        }
    }

    for ((series, doc_no), recs) in primary_doc_counts {
        if recs.len() > 1 {
            let ids: Vec<String> = recs.iter().map(|r| r.id.clone()).collect();
            for id in &ids {
                consumed_primary_ids.insert(id.clone());
            }
            let total_src: Decimal = recs
                .iter()
                .map(|r| r.pretax_amount.unwrap_or(r.total_amount))
                .sum();

            let first_rec = recs[0];
            groups.push(MatchGroup {
                id: format!("grp_dup_prim_{}_{}", series, doc_no),
                status: MatchStatus::DuplicateSuspect,
                doc_no: Some(doc_no.clone()),
                series: if series.is_empty() {
                    None
                } else {
                    Some(series.clone())
                },
                date: first_rec.date.clone(),
                partner_name: first_rec.partner_name.clone(),
                primary_source_record_ids: ids,
                target_source_record_ids: vec![],
                source_breakdowns: HashMap::new(),
                discrepancies: vec![FieldDiscrepancy {
                    field_name: "docNo".to_string(),
                    source_value: Some(format!("{} (Ký hiệu {})", doc_no, series)),
                    target_value: None,
                    amount_diff: None,
                    message: format!(
                        "Phát hiện {} bản ghi trùng số chứng từ #{} (Ký hiệu {}) trong nguồn chính",
                        recs.len(),
                        doc_no,
                        series
                    ),
                }],
                semantic_comparisons: vec![],
                revenue_variance: total_src,
                vat_variance: Decimal::ZERO,
                receivable_variance: Decimal::ZERO,
                other_variance: Decimal::ZERO,
                total_source_amount: total_src,
                total_target_amount: Decimal::ZERO,
                amount_variance: total_src,
            });
        }
    }

    // -------------------------------------------------------------
    // Build Index for Each Secondary Source (Series-Aware)
    // -------------------------------------------------------------
    #[allow(dead_code)]
    struct SourceIndex<'a> {
        source_id: String,
        source_name: String,
        source_kind: DataSourceKind,
        source_role: SourceRole,
        is_required: bool,
        by_series_and_doc: HashMap<(String, String), Vec<&'a CanonicalRecord>>,
        by_doc_no_only: HashMap<String, Vec<&'a CanonicalRecord>>,
        by_tax_amount: HashMap<(String, i64), Vec<&'a CanonicalRecord>>,
        all_records: &'a [CanonicalRecord],
    }

    let mut secondary_indexes: Vec<SourceIndex> = Vec::new();
    let mut total_secondary_records_count = 0;

    for sec_source in &secondary_sources {
        let sec_records = source_records_map
            .get(&sec_source.id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        total_secondary_records_count += sec_records.len();

        let is_required = if let Some(ref req_ids) = session.required_source_ids {
            req_ids.contains(&sec_source.id)
        } else {
            sec_source.role != SourceRole::OptionalSecondary
        };

        let mut by_series_and_doc: HashMap<(String, String), Vec<&CanonicalRecord>> =
            HashMap::new();
        let mut by_doc_no_only: HashMap<String, Vec<&CanonicalRecord>> = HashMap::new();
        let mut by_tax_amount: HashMap<(String, i64), Vec<&CanonicalRecord>> = HashMap::new();

        for rec in sec_records {
            if let Some(doc) = &rec.doc_no {
                let clean_doc = CanonicalRecord::normalize_doc_no(doc);
                if !clean_doc.is_empty() {
                    let series = rec.series.as_deref().unwrap_or("").to_string();
                    by_series_and_doc
                        .entry((series, clean_doc.clone()))
                        .or_default()
                        .push(rec);
                    by_doc_no_only.entry(clean_doc).or_default().push(rec);
                }
            }
            if let Some(tax_id) = &rec.partner_tax_id {
                let clean_tax = CanonicalRecord::normalize_tax_id(tax_id);
                if !clean_tax.is_empty() {
                    let comp_amt = rec
                        .credit_amount
                        .or(rec.debit_amount)
                        .or(rec.pretax_amount)
                        .unwrap_or(rec.total_amount);
                    let rounded = comp_amt.round().to_i64().unwrap_or(0);
                    by_tax_amount
                        .entry((clean_tax, rounded))
                        .or_default()
                        .push(rec);
                }
            }
        }

        secondary_indexes.push(SourceIndex {
            source_id: sec_source.id.clone(),
            source_name: sec_source.name.clone(),
            source_kind: sec_source.kind.clone(),
            source_role: sec_source.role,
            is_required,
            by_series_and_doc,
            by_doc_no_only,
            by_tax_amount,
            all_records: sec_records,
        });
    }

    // -------------------------------------------------------------
    // PASS 1: Series-Aware & Counterparty-Bounded Document Key Matching
    // -------------------------------------------------------------
    for primary in primary_records {
        if consumed_primary_ids.contains(&primary.id) {
            continue;
        }

        let doc_key = match &primary.doc_no {
            Some(d) => {
                let clean = CanonicalRecord::normalize_doc_no(d);
                if clean.is_empty() {
                    continue;
                }
                clean
            }
            _ => continue,
        };
        let primary_series = primary.series.as_deref().unwrap_or("").to_string();

        let mut group_target_ids: Vec<String> = Vec::new();
        let mut group_discrepancies: Vec<FieldDiscrepancy> = Vec::new();
        let mut group_source_breakdowns: HashMap<String, SourceMatchBreakdown> = HashMap::new();
        let mut group_semantic_comparisons: Vec<SemanticFieldComparison> = Vec::new();

        let mut total_target_amount = Decimal::ZERO;
        let mut matched_in_any_secondary = false;
        let mut all_secondaries_exact = true;
        let mut has_missing_required_secondary = false;
        let mut has_missing_optional_secondary = false;
        let mut has_amount_mismatch = false;
        let mut has_metadata_mismatch = false;
        let mut has_ambiguous = false;
        let mut has_tolerance = false;
        let mut has_aggregate = false;

        let mut grp_revenue_var = Decimal::ZERO;
        let mut grp_vat_var = Decimal::ZERO;
        let mut grp_receivable_var = Decimal::ZERO;
        let mut grp_other_var = Decimal::ZERO;

        let primary_display_amount = primary.pretax_amount.unwrap_or(primary.total_amount);

        for sec_idx in &secondary_indexes {
            let (pri_comp, _, semantic, semantic_name, rule_tolerance_vnd, rule_date_tol_days) =
                resolve_pair_comparison_amounts(
                    primary,
                    primary_kind,
                    primary,
                    &sec_idx.source_kind,
                    &session.comparison_rules,
                    default_tolerance_vnd,
                    default_date_tolerance_days,
                );

            // Find candidates with series awareness
            let raw_candidates: Vec<&CanonicalRecord> = if !primary_series.is_empty() {
                if let Some(list) = sec_idx
                    .by_series_and_doc
                    .get(&(primary_series.clone(), doc_key.clone()))
                {
                    list.clone()
                } else if let Some(list_no_series) = sec_idx
                    .by_series_and_doc
                    .get(&("".to_string(), doc_key.clone()))
                {
                    let freq = primary_doc_no_frequencies
                        .get(&doc_key)
                        .copied()
                        .unwrap_or(1);
                    if freq == 1 {
                        list_no_series.clone()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else if let Some(list) = sec_idx.by_doc_no_only.get(&doc_key) {
                list.clone()
            } else {
                vec![]
            };

            // Pre-filter: unconsumed, date boundary & strict counterparty validation
            let available: Vec<&&CanonicalRecord> = raw_candidates
                .iter()
                .filter(|c| {
                    !consumed_secondary_ids.contains(&c.id)
                        && is_date_within_tolerance(
                            primary.date.as_deref(),
                            c.date.as_deref(),
                            rule_date_tol_days + 15,
                        )
                        && is_counterparty_compatible(
                            primary.partner_tax_id.as_deref(),
                            c.partner_tax_id.as_deref(),
                        )
                })
                .collect();

            if available.is_empty() {
                all_secondaries_exact = false;
                if !sec_idx.is_required {
                    has_missing_optional_secondary = true;
                } else {
                    has_missing_required_secondary = true;
                }

                match semantic {
                    ComparisonSemantic::Revenue => grp_revenue_var += pri_comp,
                    ComparisonSemantic::Vat => grp_vat_var += pri_comp,
                    ComparisonSemantic::Receivable => grp_receivable_var += pri_comp,
                    _ => grp_other_var += pri_comp,
                }

                let disc = FieldDiscrepancy {
                    field_name: "docNo".to_string(),
                    source_value: Some(format!("{} (Ký hiệu {})", doc_key, primary_series)),
                    target_value: None,
                    amount_diff: Some(pri_comp),
                    message: format!(
                        "Chứng từ #{} không tìm thấy trong nguồn {}",
                        doc_key, sec_idx.source_name
                    ),
                };

                group_source_breakdowns.insert(
                    sec_idx.source_id.clone(),
                    SourceMatchBreakdown {
                        source_id: sec_idx.source_id.clone(),
                        source_name: sec_idx.source_name.clone(),
                        record_ids: vec![],
                        compared_amount: Decimal::ZERO,
                        status: MatchStatus::UnmatchedMissingInTarget,
                        discrepancies: vec![disc.clone()],
                    },
                );

                group_semantic_comparisons.push(SemanticFieldComparison {
                    semantic,
                    semantic_name: semantic_name.to_string(),
                    primary_source_id: primary_source_id.to_string(),
                    primary_source_name: primary_source_name.to_string(),
                    secondary_source_id: sec_idx.source_id.clone(),
                    secondary_source_name: sec_idx.source_name.clone(),
                    secondary_source_kind: sec_idx.source_kind.clone(),
                    semantic_field: semantic_name.to_string(),
                    expected_amount: pri_comp,
                    actual_amount: Decimal::ZERO,
                    variance: pri_comp,
                    status: MatchStatus::UnmatchedMissingInTarget,
                    primary_record_ids: vec![primary.id.clone()],
                    secondary_record_ids: vec![],
                    discrepancies: vec![disc],
                });
                continue;
            }

            matched_in_any_secondary = true;

            // Candidate evaluation
            let cand_amounts: Vec<Decimal> = available
                .iter()
                .map(|c| {
                    let (_, tgt_comp, _, _, _, _) = resolve_pair_comparison_amounts(
                        primary,
                        primary_kind,
                        c,
                        &sec_idx.source_kind,
                        &session.comparison_rules,
                        default_tolerance_vnd,
                        default_date_tolerance_days,
                    );
                    tgt_comp
                })
                .collect();

            let mut matching_subsets: Vec<Vec<usize>> = Vec::new();
            let subset_n = available.len().min(16);
            let total_combos = 1usize << subset_n;

            for mask in 1..total_combos {
                let mut sum = Decimal::ZERO;
                let mut subset_indices = Vec::new();
                for (i, amt) in cand_amounts.iter().enumerate().take(subset_n) {
                    if (mask & (1 << i)) != 0 {
                        sum += *amt;
                        subset_indices.push(i);
                    }
                }
                if (sum - pri_comp).abs() <= rule_tolerance_vnd {
                    // Check if subset elements are within date tolerance & counterparty compatible
                    let all_valid = subset_indices.iter().all(|&idx| {
                        is_date_within_tolerance(
                            primary.date.as_deref(),
                            available[idx].date.as_deref(),
                            rule_date_tol_days,
                        ) && is_counterparty_compatible(
                            primary.partner_tax_id.as_deref(),
                            available[idx].partner_tax_id.as_deref(),
                        )
                    });
                    if all_valid {
                        matching_subsets.push(subset_indices);
                    }
                }
            }

            if available.len() == 1 {
                let target = *available[0];
                consumed_secondary_ids.insert(target.id.clone());

                let (pri_comp, tgt_comp, _, _, _, _) = resolve_pair_comparison_amounts(
                    primary,
                    primary_kind,
                    target,
                    &sec_idx.source_kind,
                    &session.comparison_rules,
                    default_tolerance_vnd,
                    default_date_tolerance_days,
                );
                total_target_amount += tgt_comp;

                let pair_variance = pri_comp - tgt_comp;
                match semantic {
                    ComparisonSemantic::Revenue => grp_revenue_var += pair_variance,
                    ComparisonSemantic::Vat => grp_vat_var += pair_variance,
                    ComparisonSemantic::Receivable => grp_receivable_var += pair_variance,
                    _ => grp_other_var += pair_variance,
                }

                let discrepancies = analyze_pair_discrepancies(
                    primary,
                    target,
                    pri_comp,
                    tgt_comp,
                    rule_tolerance_vnd,
                    rule_date_tol_days,
                );

                let diff = (pri_comp - tgt_comp).abs();
                let sec_status = if discrepancies.is_empty() {
                    MatchStatus::MatchedExact
                } else if diff <= rule_tolerance_vnd
                    && discrepancies.iter().all(|d| d.field_name == "amount")
                {
                    has_tolerance = true;
                    all_secondaries_exact = false;
                    MatchStatus::MatchedWithTolerance
                } else if diff > rule_tolerance_vnd {
                    has_amount_mismatch = true;
                    all_secondaries_exact = false;
                    MatchStatus::MismatchAmount
                } else {
                    has_metadata_mismatch = true;
                    all_secondaries_exact = false;
                    MatchStatus::MismatchMetadata
                };

                group_target_ids.push(target.id.clone());
                group_discrepancies.extend(discrepancies.clone());

                group_semantic_comparisons.push(SemanticFieldComparison {
                    semantic,
                    semantic_name: semantic_name.to_string(),
                    primary_source_id: primary_source_id.to_string(),
                    primary_source_name: primary_source_name.to_string(),
                    secondary_source_id: sec_idx.source_id.clone(),
                    secondary_source_name: sec_idx.source_name.clone(),
                    secondary_source_kind: sec_idx.source_kind.clone(),
                    semantic_field: semantic_name.to_string(),
                    expected_amount: pri_comp,
                    actual_amount: tgt_comp,
                    variance: pair_variance,
                    status: sec_status.clone(),
                    primary_record_ids: vec![primary.id.clone()],
                    secondary_record_ids: vec![target.id.clone()],
                    discrepancies: discrepancies.clone(),
                });

                group_source_breakdowns.insert(
                    sec_idx.source_id.clone(),
                    SourceMatchBreakdown {
                        source_id: sec_idx.source_id.clone(),
                        source_name: sec_idx.source_name.clone(),
                        record_ids: vec![target.id.clone()],
                        compared_amount: tgt_comp,
                        status: sec_status,
                        discrepancies,
                    },
                );
            } else if matching_subsets.len() == 1 {
                let chosen_indices = &matching_subsets[0];
                if chosen_indices.len() == 1 {
                    let target = *available[chosen_indices[0]];
                    consumed_secondary_ids.insert(target.id.clone());

                    let (pri_comp, tgt_comp, _, _, _, _) = resolve_pair_comparison_amounts(
                        primary,
                        primary_kind,
                        target,
                        &sec_idx.source_kind,
                        &session.comparison_rules,
                        default_tolerance_vnd,
                        default_date_tolerance_days,
                    );
                    total_target_amount += tgt_comp;

                    let pair_variance = pri_comp - tgt_comp;
                    match semantic {
                        ComparisonSemantic::Revenue => grp_revenue_var += pair_variance,
                        ComparisonSemantic::Vat => grp_vat_var += pair_variance,
                        ComparisonSemantic::Receivable => grp_receivable_var += pair_variance,
                        _ => grp_other_var += pair_variance,
                    }

                    let discrepancies = analyze_pair_discrepancies(
                        primary,
                        target,
                        pri_comp,
                        tgt_comp,
                        rule_tolerance_vnd,
                        rule_date_tol_days,
                    );

                    let diff = (pri_comp - tgt_comp).abs();
                    let sec_status = if discrepancies.is_empty() {
                        MatchStatus::MatchedExact
                    } else if diff <= rule_tolerance_vnd
                        && discrepancies.iter().all(|d| d.field_name == "amount")
                    {
                        has_tolerance = true;
                        all_secondaries_exact = false;
                        MatchStatus::MatchedWithTolerance
                    } else if diff > rule_tolerance_vnd {
                        has_amount_mismatch = true;
                        all_secondaries_exact = false;
                        MatchStatus::MismatchAmount
                    } else {
                        has_metadata_mismatch = true;
                        all_secondaries_exact = false;
                        MatchStatus::MismatchMetadata
                    };

                    group_target_ids.push(target.id.clone());
                    group_discrepancies.extend(discrepancies.clone());

                    group_semantic_comparisons.push(SemanticFieldComparison {
                        semantic,
                        semantic_name: semantic_name.to_string(),
                        primary_source_id: primary_source_id.to_string(),
                        primary_source_name: primary_source_name.to_string(),
                        secondary_source_id: sec_idx.source_id.clone(),
                        secondary_source_name: sec_idx.source_name.clone(),
                        secondary_source_kind: sec_idx.source_kind.clone(),
                        semantic_field: semantic_name.to_string(),
                        expected_amount: pri_comp,
                        actual_amount: tgt_comp,
                        variance: pair_variance,
                        status: sec_status.clone(),
                        primary_record_ids: vec![primary.id.clone()],
                        secondary_record_ids: vec![target.id.clone()],
                        discrepancies: discrepancies.clone(),
                    });

                    group_source_breakdowns.insert(
                        sec_idx.source_id.clone(),
                        SourceMatchBreakdown {
                            source_id: sec_idx.source_id.clone(),
                            source_name: sec_idx.source_name.clone(),
                            record_ids: vec![target.id.clone()],
                            compared_amount: tgt_comp,
                            status: sec_status,
                            discrepancies,
                        },
                    );
                } else if allow_aggregate {
                    let subset_sum: Decimal = chosen_indices.iter().map(|&i| cand_amounts[i]).sum();
                    total_target_amount += subset_sum;

                    let pair_variance = pri_comp - subset_sum;
                    match semantic {
                        ComparisonSemantic::Revenue => grp_revenue_var += pair_variance,
                        ComparisonSemantic::Vat => grp_vat_var += pair_variance,
                        ComparisonSemantic::Receivable => grp_receivable_var += pair_variance,
                        _ => grp_other_var += pair_variance,
                    }

                    let target_ids: Vec<String> = chosen_indices
                        .iter()
                        .map(|&i| available[i].id.clone())
                        .collect();
                    for tid in &target_ids {
                        consumed_secondary_ids.insert(tid.clone());
                    }
                    group_target_ids.extend(target_ids.clone());

                    has_aggregate = true;
                    all_secondaries_exact = false;

                    let disc = FieldDiscrepancy {
                        field_name: "amount".to_string(),
                        source_value: Some(format_vnd(pri_comp)),
                        target_value: Some(format_vnd(subset_sum)),
                        amount_diff: Some((pri_comp - subset_sum).abs()),
                        message: format!(
                            "Khớp gộp tổng: Nguồn chính ({}) đ = Tổng {} dòng ({}) đ trong {}",
                            format_vnd(pri_comp),
                            target_ids.len(),
                            format_vnd(subset_sum),
                            sec_idx.source_name
                        ),
                    };

                    group_semantic_comparisons.push(SemanticFieldComparison {
                        semantic,
                        semantic_name: semantic_name.to_string(),
                        primary_source_id: primary_source_id.to_string(),
                        primary_source_name: primary_source_name.to_string(),
                        secondary_source_id: sec_idx.source_id.clone(),
                        secondary_source_name: sec_idx.source_name.clone(),
                        secondary_source_kind: sec_idx.source_kind.clone(),
                        semantic_field: semantic_name.to_string(),
                        expected_amount: pri_comp,
                        actual_amount: subset_sum,
                        variance: pair_variance,
                        status: MatchStatus::MatchedAggregate,
                        primary_record_ids: vec![primary.id.clone()],
                        secondary_record_ids: target_ids.clone(),
                        discrepancies: vec![disc.clone()],
                    });

                    group_discrepancies.push(disc.clone());
                    group_source_breakdowns.insert(
                        sec_idx.source_id.clone(),
                        SourceMatchBreakdown {
                            source_id: sec_idx.source_id.clone(),
                            source_name: sec_idx.source_name.clone(),
                            record_ids: target_ids,
                            compared_amount: subset_sum,
                            status: MatchStatus::MatchedAggregate,
                            discrepancies: vec![disc],
                        },
                    );
                } else {
                    has_amount_mismatch = true;
                    all_secondaries_exact = false;
                }
            } else if matching_subsets.len() > 1 {
                // Ambiguous: Multiple valid candidate combinations
                has_ambiguous = true;
                all_secondaries_exact = false;

                let target_ids: Vec<String> = available.iter().map(|c| c.id.clone()).collect();
                group_target_ids.extend(target_ids.clone());

                let all_sum: Decimal = cand_amounts.iter().sum();
                let disc = FieldDiscrepancy {
                    field_name: "amount".to_string(),
                    source_value: Some(format_vnd(pri_comp)),
                    target_value: Some(format_vnd(all_sum)),
                    amount_diff: None,
                    message: format!(
                        "Phát hiện {} bản ghi trùng số #{} trong {} với {} tổ hợp số tiền khả dĩ (cần kiểm tra thủ công)",
                        available.len(),
                        doc_key,
                        sec_idx.source_name,
                        matching_subsets.len()
                    ),
                };

                group_semantic_comparisons.push(SemanticFieldComparison {
                    semantic,
                    semantic_name: semantic_name.to_string(),
                    primary_source_id: primary_source_id.to_string(),
                    primary_source_name: primary_source_name.to_string(),
                    secondary_source_id: sec_idx.source_id.clone(),
                    secondary_source_name: sec_idx.source_name.clone(),
                    secondary_source_kind: sec_idx.source_kind.clone(),
                    semantic_field: semantic_name.to_string(),
                    expected_amount: pri_comp,
                    actual_amount: all_sum,
                    variance: pri_comp - all_sum,
                    status: MatchStatus::AmbiguousMatch,
                    primary_record_ids: vec![primary.id.clone()],
                    secondary_record_ids: target_ids.clone(),
                    discrepancies: vec![disc.clone()],
                });

                group_discrepancies.push(disc.clone());
                group_source_breakdowns.insert(
                    sec_idx.source_id.clone(),
                    SourceMatchBreakdown {
                        source_id: sec_idx.source_id.clone(),
                        source_name: sec_idx.source_name.clone(),
                        record_ids: target_ids,
                        compared_amount: all_sum,
                        status: MatchStatus::AmbiguousMatch,
                        discrepancies: vec![disc],
                    },
                );
            } else {
                // matching_subsets.is_empty(): Mismatch amount
                has_amount_mismatch = true;
                all_secondaries_exact = false;

                let target_ids: Vec<String> = available.iter().map(|c| c.id.clone()).collect();
                group_target_ids.extend(target_ids.clone());

                let all_sum: Decimal = cand_amounts.iter().sum();
                let pair_variance = pri_comp - all_sum;
                match semantic {
                    ComparisonSemantic::Revenue => grp_revenue_var += pair_variance,
                    ComparisonSemantic::Vat => grp_vat_var += pair_variance,
                    ComparisonSemantic::Receivable => grp_receivable_var += pair_variance,
                    _ => grp_other_var += pair_variance,
                }

                let disc = FieldDiscrepancy {
                    field_name: "amount".to_string(),
                    source_value: Some(format_vnd(pri_comp)),
                    target_value: Some(format_vnd(all_sum)),
                    amount_diff: Some(pair_variance.abs()),
                    message: format!(
                        "Sai lệch số tiền: Nguồn chính ({}) đ != {} ({}) đ",
                        format_vnd(pri_comp),
                        sec_idx.source_name,
                        format_vnd(all_sum)
                    ),
                };

                group_semantic_comparisons.push(SemanticFieldComparison {
                    semantic,
                    semantic_name: semantic_name.to_string(),
                    primary_source_id: primary_source_id.to_string(),
                    primary_source_name: primary_source_name.to_string(),
                    secondary_source_id: sec_idx.source_id.clone(),
                    secondary_source_name: sec_idx.source_name.clone(),
                    secondary_source_kind: sec_idx.source_kind.clone(),
                    semantic_field: semantic_name.to_string(),
                    expected_amount: pri_comp,
                    actual_amount: all_sum,
                    variance: pair_variance,
                    status: MatchStatus::MismatchAmount,
                    primary_record_ids: vec![primary.id.clone()],
                    secondary_record_ids: target_ids.clone(),
                    discrepancies: vec![disc.clone()],
                });

                group_discrepancies.push(disc.clone());
                group_source_breakdowns.insert(
                    sec_idx.source_id.clone(),
                    SourceMatchBreakdown {
                        source_id: sec_idx.source_id.clone(),
                        source_name: sec_idx.source_name.clone(),
                        record_ids: target_ids,
                        compared_amount: all_sum,
                        status: MatchStatus::MismatchAmount,
                        discrepancies: vec![disc],
                    },
                );
            }
        }

        consumed_primary_ids.insert(primary.id.clone());

        let overall_status = if has_amount_mismatch {
            MatchStatus::MismatchAmount
        } else if has_metadata_mismatch {
            MatchStatus::MismatchMetadata
        } else if has_ambiguous {
            MatchStatus::AmbiguousMatch
        } else if has_missing_required_secondary {
            MatchStatus::UnmatchedMissingInTarget
        } else if has_missing_optional_secondary {
            MatchStatus::MatchedWithMissingSource
        } else if all_secondaries_exact && matched_in_any_secondary {
            MatchStatus::MatchedExact
        } else if has_tolerance {
            MatchStatus::MatchedWithTolerance
        } else if has_aggregate {
            MatchStatus::MatchedAggregate
        } else {
            MatchStatus::MismatchMetadata
        };

        let amount_variance = if secondary_indexes.len() == 1 {
            primary_display_amount - total_target_amount
        } else {
            grp_revenue_var + grp_vat_var + grp_receivable_var + grp_other_var
        };

        groups.push(MatchGroup {
            id: format!("grp_match_{}", primary.id),
            status: overall_status,
            doc_no: primary.doc_no.clone(),
            series: primary.series.clone(),
            date: primary.date.clone(),
            partner_name: primary.partner_name.clone(),
            primary_source_record_ids: vec![primary.id.clone()],
            target_source_record_ids: group_target_ids,
            source_breakdowns: group_source_breakdowns,
            discrepancies: group_discrepancies,
            semantic_comparisons: group_semantic_comparisons,
            revenue_variance: grp_revenue_var,
            vat_variance: grp_vat_var,
            receivable_variance: grp_receivable_var,
            other_variance: grp_other_var,
            total_source_amount: primary_display_amount,
            total_target_amount,
            amount_variance,
        });
    }

    // -------------------------------------------------------------
    // PASS 2: Multi-Source Fallback (Tax ID + Pair Amount + Date)
    // ONLY applied if primary record has NO doc_no!
    // -------------------------------------------------------------
    for primary in primary_records {
        if consumed_primary_ids.contains(&primary.id) {
            continue;
        }

        if primary.doc_no.is_some() {
            continue;
        }

        if let Some(tax_id) = &primary.partner_tax_id {
            let clean_tax = CanonicalRecord::normalize_tax_id(tax_id);
            if !clean_tax.is_empty() && primary.date.is_some() {
                let mut matched_in_any = false;
                let mut group_target_ids = Vec::new();
                let mut group_comparisons = Vec::new();
                let mut group_breakdowns = HashMap::new();
                let mut group_discrepancies = Vec::new();
                let mut fallback_candidates_to_consume = Vec::new();

                let mut total_target_amount = Decimal::ZERO;
                let mut grp_revenue_var = Decimal::ZERO;
                let mut grp_vat_var = Decimal::ZERO;
                let mut grp_receivable_var = Decimal::ZERO;
                let mut grp_other_var = Decimal::ZERO;
                let mut has_mismatch = false;
                let mut has_ambiguous_fallback = false;
                let mut has_missing_required = false;

                for sec_idx in &secondary_indexes {
                    let (
                        pri_comp,
                        _,
                        semantic,
                        semantic_name,
                        rule_tolerance_vnd,
                        rule_date_tol_days,
                    ) = resolve_pair_comparison_amounts(
                        primary,
                        primary_kind,
                        primary,
                        &sec_idx.source_kind,
                        &session.comparison_rules,
                        default_tolerance_vnd,
                        default_date_tolerance_days,
                    );
                    let rounded = pri_comp.round().to_i64().unwrap_or(0);

                    let candidates = sec_idx
                        .by_tax_amount
                        .get(&(clean_tax.clone(), rounded))
                        .map(|c| c.as_slice())
                        .unwrap_or(&[]);

                    let available: Vec<&&CanonicalRecord> = candidates
                        .iter()
                        .filter(|c| {
                            !consumed_secondary_ids.contains(&c.id)
                                && is_date_within_tolerance(
                                    primary.date.as_deref(),
                                    c.date.as_deref(),
                                    rule_date_tol_days,
                                )
                        })
                        .collect();

                    if available.len() == 1 {
                        let target = *available[0];
                        let (_, tgt_comp, _, _, _, _) = resolve_pair_comparison_amounts(
                            primary,
                            primary_kind,
                            target,
                            &sec_idx.source_kind,
                            &session.comparison_rules,
                            default_tolerance_vnd,
                            default_date_tolerance_days,
                        );

                        matched_in_any = true;
                        fallback_candidates_to_consume.push(target.id.clone());
                        group_target_ids.push(target.id.clone());
                        total_target_amount += tgt_comp;

                        let pair_variance = pri_comp - tgt_comp;
                        match semantic {
                            ComparisonSemantic::Revenue => grp_revenue_var += pair_variance,
                            ComparisonSemantic::Vat => grp_vat_var += pair_variance,
                            ComparisonSemantic::Receivable => grp_receivable_var += pair_variance,
                            _ => grp_other_var += pair_variance,
                        }

                        let discrepancies = analyze_pair_discrepancies(
                            primary,
                            target,
                            pri_comp,
                            tgt_comp,
                            rule_tolerance_vnd,
                            rule_date_tol_days,
                        );

                        if !discrepancies.is_empty() {
                            has_mismatch = true;
                        }

                        group_discrepancies.extend(discrepancies.clone());
                        group_comparisons.push(SemanticFieldComparison {
                            semantic,
                            semantic_name: semantic_name.to_string(),
                            primary_source_id: primary_source_id.to_string(),
                            primary_source_name: primary_source_name.to_string(),
                            secondary_source_id: sec_idx.source_id.clone(),
                            secondary_source_name: sec_idx.source_name.clone(),
                            secondary_source_kind: sec_idx.source_kind.clone(),
                            semantic_field: semantic_name.to_string(),
                            expected_amount: pri_comp,
                            actual_amount: tgt_comp,
                            variance: pair_variance,
                            status: MatchStatus::MatchedWithTolerance,
                            primary_record_ids: vec![primary.id.clone()],
                            secondary_record_ids: vec![target.id.clone()],
                            discrepancies: discrepancies.clone(),
                        });

                        group_breakdowns.insert(
                            sec_idx.source_id.clone(),
                            SourceMatchBreakdown {
                                source_id: sec_idx.source_id.clone(),
                                source_name: sec_idx.source_name.clone(),
                                record_ids: vec![target.id.clone()],
                                compared_amount: tgt_comp,
                                status: MatchStatus::MatchedWithTolerance,
                                discrepancies,
                            },
                        );
                    } else if available.len() > 1 {
                        has_ambiguous_fallback = true;
                        let target_ids: Vec<String> =
                            available.iter().map(|c| c.id.clone()).collect();
                        group_target_ids.extend(target_ids.clone());

                        let disc = FieldDiscrepancy {
                            field_name: "partnerTaxId".to_string(),
                            source_value: Some(clean_tax.clone()),
                            target_value: None,
                            amount_diff: None,
                            message: format!(
                                "Phát hiện {} bản ghi trùng MST ({}) và số tiền trong {} (cần kiểm tra thủ công)",
                                available.len(),
                                clean_tax,
                                sec_idx.source_name
                            ),
                        };
                        group_discrepancies.push(disc.clone());
                    } else if sec_idx.is_required {
                        has_missing_required = true;
                    }
                }

                if matched_in_any || has_ambiguous_fallback {
                    consumed_primary_ids.insert(primary.id.clone());
                    let primary_display_amount =
                        primary.pretax_amount.unwrap_or(primary.total_amount);

                    let status = if has_ambiguous_fallback {
                        MatchStatus::AmbiguousMatch
                    } else if has_missing_required {
                        MatchStatus::UnmatchedMissingInTarget
                    } else if has_mismatch {
                        MatchStatus::MismatchMetadata
                    } else {
                        for cid in fallback_candidates_to_consume {
                            consumed_secondary_ids.insert(cid);
                        }
                        MatchStatus::MatchedWithTolerance
                    };

                    groups.push(MatchGroup {
                        id: format!("grp_fallback_{}", primary.id),
                        status,
                        doc_no: primary.doc_no.clone(),
                        series: primary.series.clone(),
                        date: primary.date.clone(),
                        partner_name: primary.partner_name.clone(),
                        primary_source_record_ids: vec![primary.id.clone()],
                        target_source_record_ids: group_target_ids,
                        source_breakdowns: group_breakdowns,
                        discrepancies: group_discrepancies,
                        semantic_comparisons: group_comparisons,
                        revenue_variance: grp_revenue_var,
                        vat_variance: grp_vat_var,
                        receivable_variance: grp_receivable_var,
                        other_variance: grp_other_var,
                        total_source_amount: primary_display_amount,
                        total_target_amount,
                        amount_variance: grp_revenue_var
                            + grp_vat_var
                            + grp_receivable_var
                            + grp_other_var,
                    });
                }
            }
        }
    }

    // -------------------------------------------------------------
    // PASS 2B: Unmatched Primary Sweep (Record Conservation Invariant)
    // Every primary record MUST be represented in groups!
    // -------------------------------------------------------------
    for primary in primary_records {
        if consumed_primary_ids.contains(&primary.id) {
            continue;
        }
        consumed_primary_ids.insert(primary.id.clone());

        let primary_display_amount = primary.pretax_amount.unwrap_or(primary.total_amount);
        let mut grp_revenue_var = Decimal::ZERO;
        let mut grp_vat_var = Decimal::ZERO;
        let mut grp_receivable_var = Decimal::ZERO;
        let mut grp_other_var = Decimal::ZERO;
        let mut group_comparisons = Vec::new();
        let mut group_breakdowns = HashMap::new();

        let is_insufficient_evidence = primary.doc_no.is_none() && primary.partner_tax_id.is_none();
        let status = if is_insufficient_evidence {
            MatchStatus::NeedsReview
        } else {
            MatchStatus::UnmatchedMissingInTarget
        };

        let disc_msg = if is_insufficient_evidence {
            "Bản ghi nguồn chính thiếu cả số chứng từ và MST - không đủ dữ kiện để đối chiếu tự động"
                .to_string()
        } else {
            format!(
                "Chứng từ {} không tìm thấy bản ghi tương ứng trong các nguồn đối chiếu",
                primary.doc_no.as_deref().unwrap_or("N/A")
            )
        };

        let group_disc = FieldDiscrepancy {
            field_name: "docNo".to_string(),
            source_value: primary
                .doc_no
                .clone()
                .or_else(|| primary.partner_tax_id.clone()),
            target_value: None,
            amount_diff: Some(primary_display_amount),
            message: disc_msg,
        };

        for sec_idx in &secondary_indexes {
            let (pri_comp, _, semantic, semantic_name, _, _) = resolve_pair_comparison_amounts(
                primary,
                primary_kind,
                primary,
                &sec_idx.source_kind,
                &session.comparison_rules,
                default_tolerance_vnd,
                default_date_tolerance_days,
            );

            match semantic {
                ComparisonSemantic::Revenue => grp_revenue_var += pri_comp,
                ComparisonSemantic::Vat => grp_vat_var += pri_comp,
                ComparisonSemantic::Receivable => grp_receivable_var += pri_comp,
                _ => grp_other_var += pri_comp,
            }

            group_comparisons.push(SemanticFieldComparison {
                semantic,
                semantic_name: semantic_name.to_string(),
                primary_source_id: primary_source_id.to_string(),
                primary_source_name: primary_source_name.to_string(),
                secondary_source_id: sec_idx.source_id.clone(),
                secondary_source_name: sec_idx.source_name.clone(),
                secondary_source_kind: sec_idx.source_kind.clone(),
                semantic_field: semantic_name.to_string(),
                expected_amount: pri_comp,
                actual_amount: Decimal::ZERO,
                variance: pri_comp,
                status: status.clone(),
                primary_record_ids: vec![primary.id.clone()],
                secondary_record_ids: vec![],
                discrepancies: vec![group_disc.clone()],
            });

            group_breakdowns.insert(
                sec_idx.source_id.clone(),
                SourceMatchBreakdown {
                    source_id: sec_idx.source_id.clone(),
                    source_name: sec_idx.source_name.clone(),
                    record_ids: vec![],
                    compared_amount: Decimal::ZERO,
                    status: status.clone(),
                    discrepancies: vec![group_disc.clone()],
                },
            );
        }

        let amount_variance = if secondary_indexes.len() == 1 {
            primary_display_amount
        } else {
            grp_revenue_var + grp_vat_var + grp_receivable_var + grp_other_var
        };

        groups.push(MatchGroup {
            id: format!("grp_unmatched_prim_{}", primary.id),
            status,
            doc_no: primary.doc_no.clone(),
            series: primary.series.clone(),
            date: primary.date.clone(),
            partner_name: primary.partner_name.clone(),
            primary_source_record_ids: vec![primary.id.clone()],
            target_source_record_ids: vec![],
            source_breakdowns: group_breakdowns,
            discrepancies: vec![group_disc],
            semantic_comparisons: group_comparisons,
            revenue_variance: grp_revenue_var,
            vat_variance: grp_vat_var,
            receivable_variance: grp_receivable_var,
            other_variance: grp_other_var,
            total_source_amount: primary_display_amount,
            total_target_amount: Decimal::ZERO,
            amount_variance,
        });
    }

    // -------------------------------------------------------------
    // PASS 3: Residual Sweep for Secondary-Missing Records
    // -------------------------------------------------------------
    for sec_idx in &secondary_indexes {
        for sec in sec_idx.all_records {
            if consumed_secondary_ids.contains(&sec.id) {
                continue;
            }
            consumed_secondary_ids.insert(sec.id.clone());

            let doc_display = sec
                .doc_no
                .as_deref()
                .or(sec.voucher_no.as_deref())
                .unwrap_or("N/A");
            let sec_comp_amount = sec
                .credit_amount
                .or(sec.debit_amount)
                .or(sec.pretax_amount)
                .unwrap_or(sec.total_amount);

            let mut sec_rev_var = Decimal::ZERO;
            let mut sec_vat_var = Decimal::ZERO;
            let mut sec_rec_var = Decimal::ZERO;
            let mut sec_oth_var = Decimal::ZERO;

            match sec_idx.source_kind {
                DataSourceKind::Ledger511 => sec_rev_var = -sec_comp_amount,
                DataSourceKind::Ledger3331 | DataSourceKind::Ledger133 => {
                    sec_vat_var = -sec_comp_amount
                }
                DataSourceKind::Ledger131 => sec_rec_var = -sec_comp_amount,
                _ => sec_oth_var = -sec_comp_amount,
            }

            groups.push(MatchGroup {
                id: format!("grp_missing_source_{}", sec.id),
                status: MatchStatus::UnmatchedMissingInSource,
                doc_no: sec.doc_no.clone().or_else(|| sec.voucher_no.clone()),
                series: sec.series.clone(),
                date: sec.date.clone(),
                partner_name: sec.partner_name.clone(),
                primary_source_record_ids: vec![],
                target_source_record_ids: vec![sec.id.clone()],
                source_breakdowns: HashMap::new(),
                discrepancies: vec![FieldDiscrepancy {
                    field_name: "docNo".to_string(),
                    source_value: None,
                    target_value: Some(doc_display.to_string()),
                    amount_diff: Some(sec_comp_amount),
                    message: format!(
                        "Chứng từ #{} ({}) đ tồn tại trong {} nhưng không có trong nguồn chính",
                        doc_display,
                        format_vnd(sec_comp_amount),
                        sec_idx.source_name
                    ),
                }],
                semantic_comparisons: vec![],
                revenue_variance: sec_rev_var,
                vat_variance: sec_vat_var,
                receivable_variance: sec_rec_var,
                other_variance: sec_oth_var,
                total_source_amount: Decimal::ZERO,
                total_target_amount: sec_comp_amount,
                amount_variance: -sec_comp_amount,
            });
        }
    }

    // -------------------------------------------------------------
    // Deterministic Canonical Sorting for Stable Output
    // Sort by: date (asc) -> doc_no (natural asc) -> series (asc) -> id (asc)
    // -------------------------------------------------------------
    groups.sort_by(|a, b| {
        let date_a = a.date.as_deref().unwrap_or("");
        let date_b = b.date.as_deref().unwrap_or("");
        let cmp_date = date_a.cmp(date_b);
        if cmp_date != std::cmp::Ordering::Equal {
            return cmp_date;
        }

        let doc_a = a.doc_no.as_deref().unwrap_or("");
        let doc_b = b.doc_no.as_deref().unwrap_or("");
        let cmp_doc = doc_a.cmp(doc_b);
        if cmp_doc != std::cmp::Ordering::Equal {
            return cmp_doc;
        }

        let ser_a = a.series.as_deref().unwrap_or("");
        let ser_b = b.series.as_deref().unwrap_or("");
        let cmp_ser = ser_a.cmp(ser_b);
        if cmp_ser != std::cmp::Ordering::Equal {
            return cmp_ser;
        }

        a.id.cmp(&b.id)
    });

    // -------------------------------------------------------------
    // Build Summary Metrics
    // -------------------------------------------------------------
    let mut exact_count = 0;
    let mut tolerance_count = 0;
    let mut aggregate_count = 0;
    let mut mismatch_count = 0;
    let mut missing_target_count = 0;
    let mut missing_source_count = 0;
    let mut duplicate_count = 0;
    let mut ambiguous_count = 0;
    let mut needs_review_count = 0;

    let mut sum_revenue_var = Decimal::ZERO;
    let mut sum_vat_var = Decimal::ZERO;
    let mut sum_receivable_var = Decimal::ZERO;
    let mut sum_total_discrepant = Decimal::ZERO;
    let mut net_variance = Decimal::ZERO;

    for g in &groups {
        match g.status {
            MatchStatus::MatchedExact => exact_count += 1,
            MatchStatus::MatchedWithTolerance => tolerance_count += 1,
            MatchStatus::MatchedAggregate => aggregate_count += 1,
            MatchStatus::MismatchAmount | MatchStatus::MismatchMetadata => mismatch_count += 1,
            MatchStatus::UnmatchedMissingInTarget | MatchStatus::MatchedWithMissingSource => {
                missing_target_count += 1
            }
            MatchStatus::UnmatchedMissingInSource => missing_source_count += 1,
            MatchStatus::DuplicateSuspect => duplicate_count += g.primary_source_record_ids.len(),
            MatchStatus::AmbiguousMatch => ambiguous_count += 1,
            MatchStatus::NeedsReview => needs_review_count += 1,
        }

        sum_revenue_var += g.revenue_variance;
        sum_vat_var += g.vat_variance;
        sum_receivable_var += g.receivable_variance;

        sum_total_discrepant += g.revenue_variance.abs()
            + g.vat_variance.abs()
            + g.receivable_variance.abs()
            + g.other_variance.abs();

        net_variance += g.amount_variance;
    }

    let summary = ReconciliationSummary {
        total_source_records: primary_records.len(),
        total_target_records: total_secondary_records_count,
        exact_matches_count: exact_count,
        tolerance_matches_count: tolerance_count,
        aggregate_matches_count: aggregate_count,
        mismatches_count: mismatch_count,
        missing_in_target_count: missing_target_count,
        missing_in_source_count: missing_source_count,
        duplicates_count: duplicate_count,
        ambiguous_count,
        needs_review_count,
        revenue_variance: sum_revenue_var,
        vat_variance: sum_vat_var,
        receivable_variance: sum_receivable_var,
        total_discrepant_amount: sum_total_discrepant,
        net_financial_variance: net_variance,
    };

    Ok(ReconciliationResult {
        session_id: session.session_id.clone(),
        executed_at: Utc::now().to_rfc3339(),
        profile_id: session.scenario_name.clone(),
        summary,
        groups,
    })
}

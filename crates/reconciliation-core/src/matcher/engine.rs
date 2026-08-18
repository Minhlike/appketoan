use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

use crate::analyzer::discrepancy_analyzer::{analyze_pair_discrepancies, format_vnd};
use crate::models::{
    CanonicalRecord, DataSourceKind, FieldDiscrepancy, MatchGroup, MatchStatus,
    ReconciliationResult, ReconciliationSession, ReconciliationSummary, SourceMatchBreakdown,
};

/// Resolves pairwise accounting comparison amounts based on exact source relationship
pub fn resolve_pair_comparison_amounts(
    primary: &CanonicalRecord,
    primary_kind: &DataSourceKind,
    secondary: &CanonicalRecord,
    secondary_kind: &DataSourceKind,
) -> (Decimal, Decimal) {
    match (primary_kind, secondary_kind) {
        (DataSourceKind::EInvoice, DataSourceKind::Ledger511) => (
            primary.pretax_amount.unwrap_or(primary.total_amount),
            secondary
                .credit_amount
                .unwrap_or_else(|| secondary.pretax_amount.unwrap_or(secondary.total_amount)),
        ),
        (DataSourceKind::EInvoice, DataSourceKind::Ledger3331) => (
            primary.vat_amount.unwrap_or(primary.total_amount),
            secondary
                .credit_amount
                .unwrap_or_else(|| secondary.vat_amount.unwrap_or(secondary.total_amount)),
        ),
        (DataSourceKind::EInvoice, DataSourceKind::Ledger131) => (
            primary.total_amount,
            secondary
                .debit_amount
                .unwrap_or(secondary.total_amount),
        ),
        (DataSourceKind::EInvoice, DataSourceKind::Ledger133) => (
            primary.vat_amount.unwrap_or(primary.total_amount),
            secondary
                .debit_amount
                .unwrap_or(secondary.total_amount),
        ),
        (DataSourceKind::EInvoice, DataSourceKind::BankStatement) => (
            primary.total_amount,
            secondary.credit_amount.unwrap_or(secondary.total_amount),
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
            (pri, sec)
        }
    }
}

/// Core Multi-Pass Deterministic Multi-Source Reconciliation Engine
pub fn execute_reconciliation(
    session: &ReconciliationSession,
    source_records_map: &HashMap<String, Vec<CanonicalRecord>>,
) -> ReconciliationResult {
    let tolerance_vnd = session.matching_tolerance_vnd;
    let date_tolerance_days = session.date_tolerance_days;
    let allow_aggregate = session.enable_aggregate_match;

    let empty_vec = Vec::new();

    // Deterministically select primary source
    let primary_source = if let Some(ref pri_id) = session.primary_source_id {
        session.data_sources.iter().find(|s| &s.id == pri_id)
    } else {
        session
            .data_sources
            .iter()
            .find(|s| s.kind == DataSourceKind::EInvoice)
            .or_else(|| session.data_sources.first())
    };

    let primary_source_id = primary_source
        .map(|s| s.id.as_str())
        .unwrap_or("src_primary");
    let primary_kind = primary_source
        .map(|s| &s.kind)
        .unwrap_or(&DataSourceKind::EInvoice);

    let primary_records = source_records_map
        .get(primary_source_id)
        .unwrap_or(&empty_vec);

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
    for rec in primary_records {
        if let Some(doc) = &rec.doc_no {
            let clean_doc = CanonicalRecord::normalize_doc_no(doc);
            if !clean_doc.is_empty() {
                let series = rec.series.as_deref().unwrap_or("").to_string();
                primary_doc_counts
                    .entry((series, clean_doc))
                    .or_default()
                    .push(rec);
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

            groups.push(MatchGroup {
                id: format!("grp_dup_prim_{}", doc_no),
                status: MatchStatus::DuplicateSuspect,
                primary_source_record_ids: ids,
                target_source_record_ids: vec![],
                source_breakdowns: HashMap::new(),
                discrepancies: vec![FieldDiscrepancy {
                    field_name: "docNo".to_string(),
                    source_value: Some(format!("{} (Ký hiệu {})", doc_no, series)),
                    target_value: None,
                    amount_diff: None,
                    message: format!(
                        "Phát hiện {} bản ghi trùng số chứng từ #{} trong nguồn chính",
                        recs.len(),
                        doc_no
                    ),
                }],
                total_source_amount: total_src,
                total_target_amount: Decimal::ZERO,
                amount_variance: total_src,
            });
        }
    }

    // -------------------------------------------------------------
    // Build Index for Each Secondary Source
    // -------------------------------------------------------------
    #[allow(dead_code)]
    struct SourceIndex<'a> {
        source_id: String,
        source_name: String,
        source_kind: DataSourceKind,
        by_doc_no: HashMap<String, Vec<&'a CanonicalRecord>>,
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

        let mut by_doc_no: HashMap<String, Vec<&CanonicalRecord>> = HashMap::new();
        let mut by_tax_amount: HashMap<(String, i64), Vec<&CanonicalRecord>> = HashMap::new();

        for rec in sec_records {
            if let Some(doc) = &rec.doc_no {
                let clean_doc = CanonicalRecord::normalize_doc_no(doc);
                if !clean_doc.is_empty() {
                    by_doc_no.entry(clean_doc).or_default().push(rec);
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
                    let rounded = comp_amt.round().to_string().parse::<i64>().unwrap_or(0);
                    by_tax_amount.entry((clean_tax, rounded)).or_default().push(rec);
                }
            }
        }

        secondary_indexes.push(SourceIndex {
            source_id: sec_source.id.clone(),
            source_name: sec_source.name.clone(),
            source_kind: sec_source.kind.clone(),
            by_doc_no,
            by_tax_amount,
            all_records: sec_records,
        });
    }

    // -------------------------------------------------------------
    // PASS 1: Exact Key Matching (Doc No / Series)
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

        let mut group_target_ids: Vec<String> = Vec::new();
        let mut group_discrepancies: Vec<FieldDiscrepancy> = Vec::new();
        let mut group_source_breakdowns: HashMap<String, SourceMatchBreakdown> = HashMap::new();
        let mut total_target_amount = Decimal::ZERO;
        let mut matched_in_any_secondary = false;
        let mut all_secondaries_exact = true;
        let mut has_mismatch = false;
        let mut has_tolerance = false;
        let mut has_aggregate = false;

        let primary_display_amount = primary.pretax_amount.unwrap_or(primary.total_amount);

        for sec_idx in &secondary_indexes {
            let candidates = sec_idx.by_doc_no.get(&doc_key);
            if let Some(candidates) = candidates {
                let available: Vec<&&CanonicalRecord> = candidates
                    .iter()
                    .filter(|c| !consumed_secondary_ids.contains(&c.id))
                    .collect();

                if available.is_empty() {
                    all_secondaries_exact = false;
                    continue;
                }

                matched_in_any_secondary = true;

                if available.len() == 1 {
                    let target = *available[0];
                    let (pri_comp, tgt_comp) = resolve_pair_comparison_amounts(
                        primary,
                        primary_kind,
                        target,
                        &sec_idx.source_kind,
                    );
                    total_target_amount += tgt_comp;

                    let discrepancies = analyze_pair_discrepancies(
                        primary,
                        target,
                        pri_comp,
                        tgt_comp,
                        tolerance_vnd,
                        date_tolerance_days,
                    );

                    let diff = (pri_comp - tgt_comp).abs();
                    let sec_status = if discrepancies.is_empty() {
                        MatchStatus::MatchedExact
                    } else if diff <= tolerance_vnd
                        && discrepancies.iter().all(|d| d.field_name == "amount")
                    {
                        has_tolerance = true;
                        all_secondaries_exact = false;
                        MatchStatus::MatchedWithTolerance
                    } else if diff > tolerance_vnd {
                        has_mismatch = true;
                        all_secondaries_exact = false;
                        MatchStatus::MismatchAmount
                    } else {
                        has_mismatch = true;
                        all_secondaries_exact = false;
                        MatchStatus::MismatchMetadata
                    };

                    consumed_secondary_ids.insert(target.id.clone());
                    group_target_ids.push(target.id.clone());
                    group_discrepancies.extend(discrepancies.clone());

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
                    // Aggregate 1-to-N
                    let mut sum_target = Decimal::ZERO;
                    let (pri_comp, _) = resolve_pair_comparison_amounts(
                        primary,
                        primary_kind,
                        available[0],
                        &sec_idx.source_kind,
                    );

                    for c in &available {
                        let (_, tgt_comp) = resolve_pair_comparison_amounts(
                            primary,
                            primary_kind,
                            c,
                            &sec_idx.source_kind,
                        );
                        sum_target += tgt_comp;
                    }
                    total_target_amount += sum_target;

                    let agg_diff = (pri_comp - sum_target).abs();
                    let target_ids: Vec<String> = available.iter().map(|c| c.id.clone()).collect();
                    for id in &target_ids {
                        consumed_secondary_ids.insert(id.clone());
                    }
                    group_target_ids.extend(target_ids.clone());

                    if agg_diff <= tolerance_vnd {
                        has_aggregate = true;
                        all_secondaries_exact = false;
                        let discrepancies = if !agg_diff.is_zero() {
                            vec![FieldDiscrepancy {
                                field_name: "amount".to_string(),
                                source_value: Some(format_vnd(pri_comp)),
                                target_value: Some(format_vnd(sum_target)),
                                amount_diff: Some(agg_diff),
                                message: format!(
                                    "Khớp gộp tổng: Nguồn chính ({}) đ = Tổng {} dòng ({}) đ",
                                    format_vnd(pri_comp),
                                    target_ids.len(),
                                    format_vnd(sum_target)
                                ),
                            }]
                        } else {
                            vec![]
                        };

                        group_discrepancies.extend(discrepancies.clone());
                        group_source_breakdowns.insert(
                            sec_idx.source_id.clone(),
                            SourceMatchBreakdown {
                                source_id: sec_idx.source_id.clone(),
                                source_name: sec_idx.source_name.clone(),
                                record_ids: target_ids,
                                compared_amount: sum_target,
                                status: MatchStatus::MatchedAggregate,
                                discrepancies,
                            },
                        );
                    } else {
                        has_mismatch = true;
                        all_secondaries_exact = false;
                        let discrepancies = vec![FieldDiscrepancy {
                            field_name: "amount".to_string(),
                            source_value: Some(format_vnd(pri_comp)),
                            target_value: Some(format_vnd(sum_target)),
                            amount_diff: Some(agg_diff),
                            message: format!(
                                "Lệch tiền gộp: Nguồn chính là {} đ, Tổng {} dòng là {} đ (lệch {} đ)",
                                format_vnd(pri_comp),
                                target_ids.len(),
                                format_vnd(sum_target),
                                format_vnd(agg_diff)
                            ),
                        }];

                        group_discrepancies.extend(discrepancies.clone());
                        group_source_breakdowns.insert(
                            sec_idx.source_id.clone(),
                            SourceMatchBreakdown {
                                source_id: sec_idx.source_id.clone(),
                                source_name: sec_idx.source_name.clone(),
                                record_ids: target_ids,
                                compared_amount: sum_target,
                                status: MatchStatus::MismatchAmount,
                                discrepancies,
                            },
                        );
                    }
                }
            } else {
                all_secondaries_exact = false;
            }
        }

        if matched_in_any_secondary {
            consumed_primary_ids.insert(primary.id.clone());

            let overall_status = if has_mismatch {
                MatchStatus::MismatchAmount
            } else if all_secondaries_exact && !group_source_breakdowns.is_empty() {
                MatchStatus::MatchedExact
            } else if has_tolerance {
                MatchStatus::MatchedWithTolerance
            } else if has_aggregate {
                MatchStatus::MatchedAggregate
            } else {
                MatchStatus::MatchedExact
            };

            let amount_variance = primary_display_amount - total_target_amount;

            groups.push(MatchGroup {
                id: format!("grp_match_{}", primary.id),
                status: overall_status,
                primary_source_record_ids: vec![primary.id.clone()],
                target_source_record_ids: group_target_ids,
                source_breakdowns: group_source_breakdowns,
                discrepancies: group_discrepancies,
                total_source_amount: primary_display_amount,
                total_target_amount,
                amount_variance,
            });
        }
    }

    // -------------------------------------------------------------
    // PASS 2: Secondary Matching (Tax ID + Amount)
    // ONLY applied if primary record has NO doc_no, to prevent false matches!
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
            if !clean_tax.is_empty() {
                let primary_comp_amount = primary.pretax_amount.unwrap_or(primary.total_amount);
                let rounded = primary_comp_amount.round().to_string().parse::<i64>().unwrap_or(0);

                for sec_idx in &secondary_indexes {
                    if let Some(candidates) = sec_idx.by_tax_amount.get(&(clean_tax.clone(), rounded)) {
                        let available: Vec<&&CanonicalRecord> = candidates
                            .iter()
                            .filter(|c| !consumed_secondary_ids.contains(&c.id))
                            .collect();

                        if available.len() == 1 {
                            let target = *available[0];
                            let (pri_comp, tgt_comp) = resolve_pair_comparison_amounts(
                                primary,
                                primary_kind,
                                target,
                                &sec_idx.source_kind,
                            );

                            consumed_primary_ids.insert(primary.id.clone());
                            consumed_secondary_ids.insert(target.id.clone());

                            let discrepancies = analyze_pair_discrepancies(
                                primary,
                                target,
                                pri_comp,
                                tgt_comp,
                                tolerance_vnd,
                                date_tolerance_days,
                            );

                            groups.push(MatchGroup {
                                id: format!("grp_sec_match_{}_{}", primary.id, target.id),
                                status: MatchStatus::MatchedWithTolerance,
                                primary_source_record_ids: vec![primary.id.clone()],
                                target_source_record_ids: vec![target.id.clone()],
                                source_breakdowns: HashMap::new(),
                                discrepancies,
                                total_source_amount: pri_comp,
                                total_target_amount: tgt_comp,
                                amount_variance: pri_comp - tgt_comp,
                            });
                            break;
                        }
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------
    // PASS 3: Residual Sweep for Missing Records
    // -------------------------------------------------------------
    for primary in primary_records {
        if consumed_primary_ids.contains(&primary.id) {
            continue;
        }
        consumed_primary_ids.insert(primary.id.clone());

        let doc_display = primary.doc_no.as_deref().unwrap_or("N/A");
        let primary_comp_amount = primary.pretax_amount.unwrap_or(primary.total_amount);

        groups.push(MatchGroup {
            id: format!("grp_missing_target_{}", primary.id),
            status: MatchStatus::UnmatchedMissingInTarget,
            primary_source_record_ids: vec![primary.id.clone()],
            target_source_record_ids: vec![],
            source_breakdowns: HashMap::new(),
            discrepancies: vec![FieldDiscrepancy {
                field_name: "docNo".to_string(),
                source_value: Some(doc_display.to_string()),
                target_value: None,
                amount_diff: Some(primary_comp_amount),
                message: format!(
                    "Chứng từ #{} ({}) đ tồn tại trong nguồn chính nhưng không tìm thấy trong nguồn đối chiếu",
                    doc_display,
                    format_vnd(primary_comp_amount)
                ),
            }],
            total_source_amount: primary_comp_amount,
            total_target_amount: Decimal::ZERO,
            amount_variance: primary_comp_amount,
        });
    }

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

            groups.push(MatchGroup {
                id: format!("grp_missing_source_{}", sec.id),
                status: MatchStatus::UnmatchedMissingInSource,
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
                total_source_amount: Decimal::ZERO,
                total_target_amount: sec_comp_amount,
                amount_variance: -sec_comp_amount,
            });
        }
    }

    // -------------------------------------------------------------
    // Build Summary
    // -------------------------------------------------------------
    let mut exact_count = 0;
    let mut tolerance_count = 0;
    let mut aggregate_count = 0;
    let mut mismatch_count = 0;
    let mut missing_target_count = 0;
    let mut missing_source_count = 0;
    let mut duplicate_count = 0;
    let mut ambiguous_count = 0;
    let mut net_variance = Decimal::ZERO;

    for g in &groups {
        match g.status {
            MatchStatus::MatchedExact => exact_count += 1,
            MatchStatus::MatchedWithTolerance => tolerance_count += 1,
            MatchStatus::MatchedAggregate => aggregate_count += 1,
            MatchStatus::MismatchAmount | MatchStatus::MismatchMetadata => mismatch_count += 1,
            MatchStatus::UnmatchedMissingInTarget => missing_target_count += 1,
            MatchStatus::UnmatchedMissingInSource => missing_source_count += 1,
            MatchStatus::DuplicateSuspect => duplicate_count += g.primary_source_record_ids.len(),
            MatchStatus::AmbiguousMatch => ambiguous_count += 1,
        }
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
        net_financial_variance: net_variance,
    };

    ReconciliationResult {
        session_id: session.session_id.clone(),
        executed_at: Utc::now().to_rfc3339(),
        profile_id: session.scenario_name.clone(),
        summary,
        groups,
    }
}

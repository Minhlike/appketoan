use std::collections::{HashMap, HashSet};
use chrono::Utc;

use crate::analyzer::discrepancy_analyzer::{analyze_pair_discrepancies, format_vnd};
use crate::models::{
    CanonicalRecord, FieldDiscrepancy, MatchGroup, MatchStatus, ReconciliationResult,
    ReconciliationSession, ReconciliationSummary,
};

/// Core Multi-Pass Deterministic Reconciliation Engine
pub fn execute_reconciliation(
    session: &ReconciliationSession,
    source_records_map: &HashMap<String, Vec<CanonicalRecord>>,
) -> ReconciliationResult {
    let tolerance_vnd = session.matching_tolerance_vnd;
    let date_tolerance_days = session.date_tolerance_days;
    let allow_aggregate = session.enable_aggregate_match;

    let empty_vec = Vec::new();
    let primary_source_id = session
        .data_sources
        .first()
        .map(|s| s.id.as_str())
        .unwrap_or("src_primary");

    let primary_records = source_records_map
        .get(primary_source_id)
        .unwrap_or(&empty_vec);

    let mut secondary_records = Vec::new();
    for source in session.data_sources.iter().skip(1) {
        if let Some(records) = source_records_map.get(&source.id) {
            secondary_records.extend(records.iter());
        }
    }

    let mut consumed_primary_ids = HashSet::new();
    let mut consumed_secondary_ids = HashSet::new();
    let mut groups: Vec<MatchGroup> = Vec::new();

    // -------------------------------------------------------------
    // PASS 0: Duplicate Detection within each source
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
            let total_src = recs.iter().map(|r| r.total_amount).sum();
            groups.push(MatchGroup {
                id: format!("grp_dup_prim_{}", doc_no),
                status: MatchStatus::DuplicateSuspect,
                primary_source_record_ids: ids,
                target_source_record_ids: vec![],
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
                total_target_amount: 0.0,
                amount_variance: total_src,
            });
        }
    }

    // -------------------------------------------------------------
    // Index Target records by doc_no and (tax_id, rounded_amount)
    // -------------------------------------------------------------
    let mut target_by_doc_no: HashMap<String, Vec<&CanonicalRecord>> = HashMap::new();
    let mut target_by_voucher_no: HashMap<String, Vec<&CanonicalRecord>> = HashMap::new();
    let mut target_by_tax_amount: HashMap<(String, i64), Vec<&CanonicalRecord>> = HashMap::new();

    for rec in &secondary_records {
        if consumed_secondary_ids.contains(&rec.id) {
            continue;
        }
        if let Some(doc) = &rec.doc_no {
            let clean_doc = CanonicalRecord::normalize_doc_no(doc);
            if !clean_doc.is_empty() {
                target_by_doc_no.entry(clean_doc).or_default().push(rec);
            }
        }
        if let Some(v_no) = &rec.voucher_no {
            let clean_v = CanonicalRecord::normalize_doc_no(v_no);
            if !clean_v.is_empty() {
                target_by_voucher_no.entry(clean_v).or_default().push(rec);
            }
        }
        if let Some(tax_id) = &rec.partner_tax_id {
            let clean_tax = CanonicalRecord::normalize_tax_id(tax_id);
            if !clean_tax.is_empty() {
                let rounded = rec.total_amount.round() as i64;
                target_by_tax_amount
                    .entry((clean_tax, rounded))
                    .or_default()
                    .push(rec);
            }
        }
    }

    // -------------------------------------------------------------
    // PASS 1: Exact Key Matching (Doc No / Series) & Discrepancies
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

        if let Some(candidates) = target_by_doc_no.get(&doc_key) {
            let available_candidates: Vec<&&CanonicalRecord> = candidates
                .iter()
                .filter(|c| !consumed_secondary_ids.contains(&c.id))
                .collect();

            if available_candidates.is_empty() {
                continue;
            }

            // Case A: 1-to-1 match candidate
            if available_candidates.len() == 1 {
                let target = *available_candidates[0];
                let discrepancies = analyze_pair_discrepancies(
                    primary,
                    target,
                    tolerance_vnd,
                    date_tolerance_days,
                );

                let total_diff = (primary.total_amount - target.total_amount).abs();
                let status = if discrepancies.is_empty() {
                    MatchStatus::MatchedExact
                } else if total_diff <= tolerance_vnd
                    && discrepancies.iter().all(|d| d.field_name == "totalAmount")
                {
                    MatchStatus::MatchedWithTolerance
                } else if total_diff > tolerance_vnd {
                    MatchStatus::MismatchAmount
                } else {
                    MatchStatus::MismatchMetadata
                };

                consumed_primary_ids.insert(primary.id.clone());
                consumed_secondary_ids.insert(target.id.clone());

                groups.push(MatchGroup {
                    id: format!("grp_match_{}_{}", primary.id, target.id),
                    status,
                    primary_source_record_ids: vec![primary.id.clone()],
                    target_source_record_ids: vec![target.id.clone()],
                    discrepancies,
                    total_source_amount: primary.total_amount,
                    total_target_amount: target.total_amount,
                    amount_variance: primary.total_amount - target.total_amount,
                });
            } else if allow_aggregate {
                // Case B: 1-to-N aggregate match (sum of multiple lines equals primary)
                let sum_target: f64 = available_candidates.iter().map(|c| c.total_amount).sum();
                let agg_diff = (primary.total_amount - sum_target).abs();

                if agg_diff <= tolerance_vnd {
                    let target_ids: Vec<String> =
                        available_candidates.iter().map(|c| c.id.clone()).collect();
                    for id in &target_ids {
                        consumed_secondary_ids.insert(id.clone());
                    }
                    consumed_primary_ids.insert(primary.id.clone());

                    let discrepancies = if agg_diff > 0.0 {
                        vec![FieldDiscrepancy {
                            field_name: "totalAmount".to_string(),
                            source_value: Some(format_vnd(primary.total_amount)),
                            target_value: Some(format_vnd(sum_target)),
                            amount_diff: Some(agg_diff),
                            message: format!(
                                "Khớp gộp tổng: Nguồn A ({}) đ = Tổng {} dòng Nguồn B ({}) đ",
                                format_vnd(primary.total_amount),
                                target_ids.len(),
                                format_vnd(sum_target)
                            ),
                        }]
                    } else {
                        vec![]
                    };

                    groups.push(MatchGroup {
                        id: format!("grp_agg_{}", primary.id),
                        status: MatchStatus::MatchedAggregate,
                        primary_source_record_ids: vec![primary.id.clone()],
                        target_source_record_ids: target_ids,
                        discrepancies,
                        total_source_amount: primary.total_amount,
                        total_target_amount: sum_target,
                        amount_variance: primary.total_amount - sum_target,
                    });
                } else {
                    // Mismatch in aggregate
                    let target_ids: Vec<String> =
                        available_candidates.iter().map(|c| c.id.clone()).collect();
                    for id in &target_ids {
                        consumed_secondary_ids.insert(id.clone());
                    }
                    consumed_primary_ids.insert(primary.id.clone());

                    groups.push(MatchGroup {
                        id: format!("grp_agg_mismatch_{}", primary.id),
                        status: MatchStatus::MismatchAmount,
                        primary_source_record_ids: vec![primary.id.clone()],
                        target_source_record_ids: target_ids,
                        discrepancies: vec![FieldDiscrepancy {
                            field_name: "totalAmount".to_string(),
                            source_value: Some(format_vnd(primary.total_amount)),
                            target_value: Some(format_vnd(sum_target)),
                            amount_diff: Some(agg_diff),
                            message: format!(
                                "Lệch tiền gộp: Nguồn A là {} đ, Tổng {} dòng Nguồn B là {} đ (lệch {} đ)",
                                format_vnd(primary.total_amount),
                                available_candidates.len(),
                                format_vnd(sum_target),
                                format_vnd(agg_diff)
                            ),
                        }],
                        total_source_amount: primary.total_amount,
                        total_target_amount: sum_target,
                        amount_variance: primary.total_amount - sum_target,
                    });
                }
            }
        }
    }

    // -------------------------------------------------------------
    // PASS 2: Secondary Matching (Tax ID + Amount)
    // -------------------------------------------------------------
    for primary in primary_records {
        if consumed_primary_ids.contains(&primary.id) {
            continue;
        }

        if let Some(tax_id) = &primary.partner_tax_id {
            let clean_tax = CanonicalRecord::normalize_tax_id(tax_id);
            if !clean_tax.is_empty() {
                let rounded = primary.total_amount.round() as i64;
                if let Some(candidates) = target_by_tax_amount.get(&(clean_tax.clone(), rounded)) {
                    let available: Vec<&&CanonicalRecord> = candidates
                        .iter()
                        .filter(|c| !consumed_secondary_ids.contains(&c.id))
                        .collect();

                    if available.len() == 1 {
                        let target = *available[0];
                        consumed_primary_ids.insert(primary.id.clone());
                        consumed_secondary_ids.insert(target.id.clone());

                        let discrepancies = analyze_pair_discrepancies(
                            primary,
                            target,
                            tolerance_vnd,
                            date_tolerance_days,
                        );

                        groups.push(MatchGroup {
                            id: format!("grp_sec_match_{}_{}", primary.id, target.id),
                            status: MatchStatus::MatchedWithTolerance,
                            primary_source_record_ids: vec![primary.id.clone()],
                            target_source_record_ids: vec![target.id.clone()],
                            discrepancies,
                            total_source_amount: primary.total_amount,
                            total_target_amount: target.total_amount,
                            amount_variance: primary.total_amount - target.total_amount,
                        });
                    } else if available.len() > 1 {
                        // Ambiguous match
                        let target_ids: Vec<String> =
                            available.iter().map(|c| c.id.clone()).collect();
                        for id in &target_ids {
                            consumed_secondary_ids.insert(id.clone());
                        }
                        consumed_primary_ids.insert(primary.id.clone());

                        groups.push(MatchGroup {
                            id: format!("grp_ambiguous_{}", primary.id),
                            status: MatchStatus::AmbiguousMatch,
                            primary_source_record_ids: vec![primary.id.clone()],
                            target_source_record_ids: target_ids,
                            discrepancies: vec![FieldDiscrepancy {
                                field_name: "partnerTaxId".to_string(),
                                source_value: Some(clean_tax.clone()),
                                target_value: None,
                                amount_diff: None,
                                message: format!(
                                    "Khớp nhiều bản ghi cùng MST {} và cùng số tiền {} đ. Cần kiểm tra thủ công.",
                                    clean_tax,
                                    format_vnd(primary.total_amount)
                                ),
                            }],
                            total_source_amount: primary.total_amount,
                            total_target_amount: primary.total_amount,
                            amount_variance: 0.0,
                        });
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------
    // PASS 3: Residual Sweep for Missing Records
    // -------------------------------------------------------------
    // Residual Unmatched in Primary -> Missing in Target
    for primary in primary_records {
        if consumed_primary_ids.contains(&primary.id) {
            continue;
        }
        consumed_primary_ids.insert(primary.id.clone());

        let doc_display = primary.doc_no.as_deref().unwrap_or("N/A");
        groups.push(MatchGroup {
            id: format!("grp_missing_target_{}", primary.id),
            status: MatchStatus::UnmatchedMissingInTarget,
            primary_source_record_ids: vec![primary.id.clone()],
            target_source_record_ids: vec![],
            discrepancies: vec![FieldDiscrepancy {
                field_name: "docNo".to_string(),
                source_value: Some(doc_display.to_string()),
                target_value: None,
                amount_diff: Some(primary.total_amount),
                message: format!(
                    "Chứng từ #{} ({}) đ tồn tại trong nguồn chính nhưng không tìm thấy trong nguồn đối chiếu",
                    doc_display,
                    format_vnd(primary.total_amount)
                ),
            }],
            total_source_amount: primary.total_amount,
            total_target_amount: 0.0,
            amount_variance: primary.total_amount,
        });
    }

    // Residual Unmatched in Secondary -> Missing in Primary
    for sec in &secondary_records {
        if consumed_secondary_ids.contains(&sec.id) {
            continue;
        }
        consumed_secondary_ids.insert(sec.id.clone());

        let doc_display = sec
            .doc_no
            .as_deref()
            .or(sec.voucher_no.as_deref())
            .unwrap_or("N/A");

        groups.push(MatchGroup {
            id: format!("grp_missing_source_{}", sec.id),
            status: MatchStatus::UnmatchedMissingInSource,
            primary_source_record_ids: vec![],
            target_source_record_ids: vec![sec.id.clone()],
            discrepancies: vec![FieldDiscrepancy {
                field_name: "docNo".to_string(),
                source_value: None,
                target_value: Some(doc_display.to_string()),
                amount_diff: Some(sec.total_amount),
                message: format!(
                    "Chứng từ #{} ({}) đ tồn tại trong nguồn đối chiếu nhưng không có trong nguồn chính",
                    doc_display,
                    format_vnd(sec.total_amount)
                ),
            }],
            total_source_amount: 0.0,
            total_target_amount: sec.total_amount,
            amount_variance: -sec.total_amount,
        });
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
    let mut net_variance = 0.0;

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
        total_target_records: secondary_records.len(),
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

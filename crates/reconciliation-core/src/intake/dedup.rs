use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use crate::models::{
    CanonicalRecord, DataSource, DataSourceKind, ReconciliationSession, SourceRole,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DatasetRelation {
    Unique,
    ExactDuplicate {
        original_source_id: String,
        raw_sha256: String,
    },
    ContentDuplicate {
        original_source_id: String,
        content_fingerprint: String,
    },
    SubsetDuplicate {
        superset_source_id: String,
        record_count: usize,
        total_in_superset: usize,
    },
    PartialOverlap {
        overlapping_source_id: String,
        overlap_count: usize,
        overlap_ratio: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeSourceAnalysis {
    pub source_id: String,
    pub source_name: String,
    pub file_path: String,
    pub raw_sha256: Option<String>,
    pub canonical_content_fingerprint: String,
    pub total_records: usize,
    pub relation: DatasetRelation,
    pub is_eligible_for_reconciliation: bool,
    pub diagnostic_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeAnalysisResult {
    pub total_physical_sources: usize,
    pub unique_datasets_count: usize,
    pub exact_duplicates_count: usize,
    pub content_duplicates_count: usize,
    pub subset_duplicates_count: usize,
    pub partial_overlaps_count: usize,
    pub logical_sources_count: usize,
    pub source_analyses: Vec<IntakeSourceAnalysis>,
    pub requires_user_confirmation: bool,
}

/// Removes typed reference-control sources before transactional reconciliation
/// and re-resolves a primary from the remaining physical datasets. This keeps
/// a partner master loaded first from becoming the transactional primary.
pub fn transactional_session_from(
    session: &ReconciliationSession,
    transactional_source_ids: &HashSet<&str>,
) -> ReconciliationSession {
    let mut transactional_session = session.clone();
    transactional_session
        .data_sources
        .retain(|source| transactional_source_ids.contains(source.id.as_str()));
    transactional_session.primary_source_id = transactional_session
        .data_sources
        .iter()
        .find(|source| source.role == SourceRole::Primary)
        .or_else(|| {
            transactional_session
                .data_sources
                .iter()
                .find(|source| source.kind == DataSourceKind::EInvoice)
        })
        .or_else(|| transactional_session.data_sources.first())
        .map(|source| source.id.clone());
    if transactional_session
        .expected_primary_kind
        .as_ref()
        .is_some_and(|kind| {
            *kind == DataSourceKind::PartnerMaster || *kind == DataSourceKind::SalesAnalysisReport
        })
    {
        transactional_session.expected_primary_kind = None;
    }
    transactional_session.required_source_ids = session.required_source_ids.as_ref().map(|ids| {
        ids.iter()
            .filter(|id| transactional_source_ids.contains(id.as_str()))
            .cloned()
            .collect()
    });
    transactional_session.optional_source_ids = session.optional_source_ids.as_ref().map(|ids| {
        ids.iter()
            .filter(|id| transactional_source_ids.contains(id.as_str()))
            .cloned()
            .collect()
    });
    transactional_session
}

/// Computes a stable canonical fingerprint for an individual accounting record
pub fn compute_record_fingerprint(r: &CanonicalRecord, kind: &DataSourceKind) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", kind).as_bytes());
    hasher.update(b"|");

    match kind {
        DataSourceKind::EInvoice => {
            hasher.update(
                CanonicalRecord::normalize_doc_no(r.doc_no.as_deref().unwrap_or("")).as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(
                r.series
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_uppercase()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.date.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(
                CanonicalRecord::normalize_tax_id(
                    r.seller_tax_id
                        .as_deref()
                        .or(r.buyer_tax_id.as_deref())
                        .or(r.partner_tax_id.as_deref())
                        .unwrap_or(""),
                )
                .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(
                r.pretax_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(
                r.vat_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.total_amount.to_string().as_bytes());
        }
        DataSourceKind::Ledger511
        | DataSourceKind::Ledger3331
        | DataSourceKind::Ledger131
        | DataSourceKind::Ledger133
        | DataSourceKind::Ledger112
        | DataSourceKind::SalesRegister
        | DataSourceKind::CashBook
        | DataSourceKind::BranchLedger => {
            hasher.update(
                CanonicalRecord::normalize_doc_no(
                    r.doc_no
                        .as_deref()
                        .or(r.voucher_no.as_deref())
                        .unwrap_or(""),
                )
                .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(
                r.series
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_uppercase()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.date.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(
                CanonicalRecord::normalize_tax_id(r.partner_tax_id.as_deref().unwrap_or(""))
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.debit_account.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(r.credit_account.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(
                r.credit_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(
                r.debit_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.total_amount.to_string().as_bytes());
        }
        DataSourceKind::BankStatement => {
            hasher.update(r.bank_account.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(
                CanonicalRecord::normalize_doc_no(
                    r.doc_no
                        .as_deref()
                        .or(r.voucher_no.as_deref())
                        .unwrap_or(""),
                )
                .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.date.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(
                r.credit_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(
                r.debit_amount
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.total_amount.to_string().as_bytes());
        }
        DataSourceKind::PartnerMaster
        | DataSourceKind::SalesAnalysisReport
        | DataSourceKind::Custom => {
            hasher.update(
                CanonicalRecord::normalize_doc_no(r.doc_no.as_deref().unwrap_or("")).as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.date.as_deref().unwrap_or("").trim().as_bytes());
            hasher.update(b"|");
            hasher.update(
                CanonicalRecord::normalize_tax_id(r.partner_tax_id.as_deref().unwrap_or(""))
                    .as_bytes(),
            );
            hasher.update(b"|");
            hasher.update(r.total_amount.to_string().as_bytes());
        }
    }

    format!("{:x}", hasher.finalize())
}

/// Computes a deterministic canonical content fingerprint for an entire dataset
pub fn compute_canonical_content_fingerprint(
    records: &[CanonicalRecord],
    kind: &DataSourceKind,
) -> String {
    let mut fps: Vec<String> = records
        .iter()
        .map(|r| compute_record_fingerprint(r, kind))
        .collect();
    fps.sort();
    let mut hasher = Sha256::new();
    for fp in fps {
        hasher.update(fp.as_bytes());
        hasher.update(b";");
    }
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LogicalSourceCompatibilityKey {
    pub kind: DataSourceKind,
    pub role: SourceRole,
    pub accounting_entity_id: Option<String>,
    pub bank_account: Option<String>,
    pub ledger_account: Option<String>,
}

impl LogicalSourceCompatibilityKey {
    pub fn from_data_source(ds: &DataSource, recs: &[CanonicalRecord]) -> Self {
        let mut bank_account: Option<String> = None;
        let mut accounting_entity_id: Option<String> = None;
        let mut ledger_account: Option<String> = None;

        for r in recs {
            if bank_account.is_none() && r.bank_account.is_some() {
                bank_account = r.bank_account.clone().filter(|s| !s.trim().is_empty());
            }
            if accounting_entity_id.is_none() {
                let tid = r
                    .seller_tax_id
                    .as_deref()
                    .or(r.buyer_tax_id.as_deref())
                    .or(r.partner_tax_id.as_deref());
                if let Some(t) = tid {
                    let clean = CanonicalRecord::normalize_tax_id(t);
                    if !clean.is_empty() {
                        accounting_entity_id = Some(clean);
                    }
                }
            }
            if ledger_account.is_none() {
                let acc = r.debit_account.as_deref().or(r.credit_account.as_deref());
                if let Some(a) = acc {
                    let clean = a.trim();
                    if !clean.is_empty() {
                        ledger_account = Some(clean.to_string());
                    }
                }
            }
            if bank_account.is_some() && accounting_entity_id.is_some() && ledger_account.is_some()
            {
                break;
            }
        }

        Self {
            kind: ds.kind.clone(),
            role: ds.role,
            accounting_entity_id,
            bank_account,
            ledger_account,
        }
    }
}

#[derive(Debug, Clone)]
struct AcceptedSourceMeta {
    source_id: String,
    source_name: String,
    file_path: String,
    sheet_name: String,
    kind: DataSourceKind,
    compatibility_key: LogicalSourceCompatibilityKey,
    raw_hash: Option<String>,
    content_fingerprint: String,
    record_fingerprints: HashSet<String>,
}

/// Analyzes incoming physical files/datasets for exact duplicate, content duplicate, subset, and partial overlap
pub fn analyze_intake_data_sources(
    session: &ReconciliationSession,
    source_records_map: &HashMap<String, Vec<CanonicalRecord>>,
    raw_file_hashes: &HashMap<String, String>,
) -> IntakeAnalysisResult {
    let mut analyses: Vec<IntakeSourceAnalysis> = Vec::new();
    let mut accepted_sources: Vec<AcceptedSourceMeta> = Vec::new();

    let mut exact_dup_count = 0;
    let mut content_dup_count = 0;
    let mut subset_dup_count = 0;
    let mut partial_overlap_count = 0;
    let mut requires_user_confirmation = false;

    for ds in &session.data_sources {
        let recs = source_records_map.get(&ds.id).cloned().unwrap_or_default();
        let raw_hash = raw_file_hashes.get(&ds.id).cloned();
        let content_fp = compute_canonical_content_fingerprint(&recs, &ds.kind);
        let compat_key = LogicalSourceCompatibilityKey::from_data_source(ds, &recs);
        let record_fps: HashSet<String> = recs
            .iter()
            .map(|r| compute_record_fingerprint(r, &ds.kind))
            .collect();

        let mut matched_relation = DatasetRelation::Unique;
        let mut is_eligible = true;
        let mut diag_msg = format!("Nguồn '{}' hợp lệ để đối chiếu.", ds.name);

        for prev in &accepted_sources {
            if ds.kind != prev.kind {
                continue;
            }

            let same_sheet = ds
                .sheet_name
                .trim()
                .eq_ignore_ascii_case(prev.sheet_name.trim());
            let same_file_path = ds
                .file_path
                .trim()
                .eq_ignore_ascii_case(prev.file_path.trim());
            let same_raw_hash = match (&raw_hash, &prev.raw_hash) {
                (Some(curr_raw), Some(prev_raw)) => curr_raw == prev_raw,
                _ => false,
            };

            // 1. Exact raw byte & sheet duplicate
            if (same_raw_hash || same_file_path)
                && same_sheet
                && content_fp == prev.content_fingerprint
                && compat_key == prev.compatibility_key
            {
                matched_relation = DatasetRelation::ExactDuplicate {
                    original_source_id: prev.source_id.clone(),
                    raw_sha256: raw_hash.clone().unwrap_or_default(),
                };
                is_eligible = false;
                exact_dup_count += 1;
                diag_msg = format!(
                    "Bỏ qua '{}': Tệp trùng lặp chính xác (Exact Duplicate) với nguồn '{}' (cùng tệp và cùng sheet '{}')",
                    ds.name, prev.source_name, ds.sheet_name
                );
                break;
            }

            // 2. Canonical content duplicate
            if content_fp == prev.content_fingerprint && compat_key == prev.compatibility_key {
                matched_relation = DatasetRelation::ContentDuplicate {
                    original_source_id: prev.source_id.clone(),
                    content_fingerprint: content_fp.clone(),
                };
                is_eligible = false;
                content_dup_count += 1;
                diag_msg = format!(
                    "Bỏ qua '{}': Trùng lặp nội dung chuẩn hóa (Content Duplicate) với nguồn '{}'",
                    ds.name, prev.source_name
                );
                break;
            }

            // 3. Symmetric subset detection
            let curr_is_subset =
                !record_fps.is_empty() && record_fps.is_subset(&prev.record_fingerprints);
            let prev_is_subset = !prev.record_fingerprints.is_empty()
                && prev.record_fingerprints.is_subset(&record_fps);

            if (curr_is_subset || prev_is_subset) && compat_key == prev.compatibility_key {
                matched_relation = DatasetRelation::SubsetDuplicate {
                    superset_source_id: if curr_is_subset {
                        prev.source_id.clone()
                    } else {
                        ds.id.clone()
                    },
                    record_count: if curr_is_subset {
                        record_fps.len()
                    } else {
                        prev.record_fingerprints.len()
                    },
                    total_in_superset: if curr_is_subset {
                        prev.record_fingerprints.len()
                    } else {
                        record_fps.len()
                    },
                };
                is_eligible = false;
                requires_user_confirmation = true;
                subset_dup_count += 1;
                diag_msg = format!(
                    "Phát hiện quan hệ tập con (Subset) giữa '{}' và '{}' (yêu cầu xác nhận)",
                    ds.name, prev.source_name
                );
                break;
            }

            // 4. Symmetric partial overlap detection
            let overlap_count = record_fps.intersection(&prev.record_fingerprints).count();
            if overlap_count > 0 && compat_key == prev.compatibility_key {
                let ratio = (overlap_count as f64 / record_fps.len() as f64) * 100.0;
                matched_relation = DatasetRelation::PartialOverlap {
                    overlapping_source_id: prev.source_id.clone(),
                    overlap_count,
                    overlap_ratio: format!("{:.1}%", ratio),
                };
                is_eligible = false;
                requires_user_confirmation = true;
                partial_overlap_count += 1;
                diag_msg = format!(
                    "Phát hiện trùng lặp một phần ({} bản ghi trùng) giữa '{}' và '{}'",
                    overlap_count, ds.name, prev.source_name
                );
                break;
            }
        }

        if is_eligible {
            accepted_sources.push(AcceptedSourceMeta {
                source_id: ds.id.clone(),
                source_name: ds.name.clone(),
                file_path: ds.file_path.clone(),
                sheet_name: ds.sheet_name.clone(),
                kind: ds.kind.clone(),
                compatibility_key: compat_key,
                raw_hash: raw_hash.clone(),
                content_fingerprint: content_fp.clone(),
                record_fingerprints: record_fps,
            });
        }

        analyses.push(IntakeSourceAnalysis {
            source_id: ds.id.clone(),
            source_name: ds.name.clone(),
            file_path: ds.file_path.clone(),
            raw_sha256: raw_hash,
            canonical_content_fingerprint: content_fp,
            total_records: recs.len(),
            relation: matched_relation,
            is_eligible_for_reconciliation: is_eligible,
            diagnostic_message: diag_msg,
        });
    }

    let unique_count = accepted_sources.len();

    // Compute logical source count based on distinct accounting compatibility keys
    let mut compat_groups: HashMap<LogicalSourceCompatibilityKey, usize> = HashMap::new();
    for acc in &accepted_sources {
        *compat_groups
            .entry(acc.compatibility_key.clone())
            .or_insert(0) += 1;
    }
    let logical_count = compat_groups.len();

    IntakeAnalysisResult {
        total_physical_sources: session.data_sources.len(),
        unique_datasets_count: unique_count,
        exact_duplicates_count: exact_dup_count,
        content_duplicates_count: content_dup_count,
        subset_duplicates_count: subset_dup_count,
        partial_overlaps_count: partial_overlap_count,
        logical_sources_count: logical_count,
        source_analyses: analyses,
        requires_user_confirmation,
    }
}

pub type FilteredIntakeResult = Result<
    (
        ReconciliationSession,
        HashMap<String, Vec<CanonicalRecord>>,
        IntakeAnalysisResult,
    ),
    String,
>;

/// Filter session and records map through the intake gate & construct logical sources for disjoint partitions
pub fn filter_reconciliation_session_and_records(
    session: &ReconciliationSession,
    source_records_map: &HashMap<String, Vec<CanonicalRecord>>,
    raw_file_hashes: &HashMap<String, String>,
) -> FilteredIntakeResult {
    let mut analysis = analyze_intake_data_sources(session, source_records_map, raw_file_hashes);

    if analysis.requires_user_confirmation {
        let issues: Vec<String> = analysis
            .source_analyses
            .iter()
            .filter(|a| {
                matches!(
                    a.relation,
                    DatasetRelation::PartialOverlap { .. }
                        | DatasetRelation::SubsetDuplicate { .. }
                )
            })
            .map(|a| a.diagnostic_message.clone())
            .collect();
        return Err(format!(
            "DATASET_CONFLICT: Yêu cầu xác nhận của người dùng trước khi đối chiếu (fail-closed): {}",
            issues.join("; ")
        ));
    }

    let eligible_source_ids: HashSet<String> = analysis
        .source_analyses
        .iter()
        .filter(|a| a.is_eligible_for_reconciliation)
        .map(|a| a.source_id.clone())
        .collect();

    // Map duplicate sources to their canonical original source ID
    let mut remap_map: HashMap<String, String> = HashMap::new();
    for a in &analysis.source_analyses {
        match &a.relation {
            DatasetRelation::ExactDuplicate {
                original_source_id, ..
            } => {
                remap_map.insert(a.source_id.clone(), original_source_id.clone());
            }
            DatasetRelation::ContentDuplicate {
                original_source_id, ..
            } => {
                remap_map.insert(a.source_id.clone(), original_source_id.clone());
            }
            _ => {}
        }
    }

    // Group eligible sources by LogicalSourceCompatibilityKey for accounting-aware partition merge
    let mut partition_groups: HashMap<LogicalSourceCompatibilityKey, Vec<&DataSource>> =
        HashMap::new();
    for ds in &session.data_sources {
        if eligible_source_ids.contains(&ds.id) {
            let recs = source_records_map.get(&ds.id).cloned().unwrap_or_default();
            let key = LogicalSourceCompatibilityKey::from_data_source(ds, &recs);
            partition_groups.entry(key).or_default().push(ds);
        }
    }

    let mut unified_data_sources: Vec<DataSource> = Vec::new();
    let mut unified_records_map: HashMap<String, Vec<CanonicalRecord>> = HashMap::new();

    // Sort compatibility keys to guarantee deterministic order
    let mut sorted_keys: Vec<LogicalSourceCompatibilityKey> =
        partition_groups.keys().cloned().collect();
    sorted_keys.sort_by_key(|k| format!("{:?}_{:?}_{:?}", k.kind, k.role, k.accounting_entity_id));

    for key in sorted_keys {
        let group = partition_groups.get(&key).unwrap();
        if group.len() == 1 {
            let single_ds = group[0];
            unified_data_sources.push(single_ds.clone());
            let recs = source_records_map
                .get(&single_ds.id)
                .cloned()
                .unwrap_or_default();
            unified_records_map.insert(single_ds.id.clone(), recs);
        } else {
            // Disjoint multi-file partition merge into a single LogicalSource
            let mut sorted_group: Vec<&DataSource> = group.clone();
            sorted_group.sort_by_key(|ds| {
                compute_canonical_content_fingerprint(
                    source_records_map
                        .get(&ds.id)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]),
                    &ds.kind,
                )
            });

            let mut all_dataset_fps: Vec<String> = sorted_group
                .iter()
                .map(|ds| {
                    compute_canonical_content_fingerprint(
                        source_records_map
                            .get(&ds.id)
                            .map(|v| v.as_slice())
                            .unwrap_or(&[]),
                        &ds.kind,
                    )
                })
                .collect();
            all_dataset_fps.sort();

            let mut id_hasher = Sha256::new();
            id_hasher.update(format!("{:?}_{:?}", key.kind, key.role).as_bytes());
            for fp in &all_dataset_fps {
                id_hasher.update(fp.as_bytes());
                id_hasher.update(b";");
            }
            let id_hash = format!("{:x}", id_hasher.finalize());
            let logical_id = format!("log_{}_{}", key.kind.as_str(), &id_hash[0..10]);

            let primary_rep = sorted_group[0];
            let mut raw_merged_records: Vec<CanonicalRecord> = Vec::new();

            for part_ds in &sorted_group {
                remap_map.insert(part_ds.id.clone(), logical_id.clone());

                if let Some(part_recs) = source_records_map.get(&part_ds.id) {
                    for rec in part_recs {
                        raw_merged_records.push(rec.clone());
                    }
                }
            }

            // Sort merged records deterministically
            raw_merged_records.sort_by(|a, b| {
                a.date
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.date.as_deref().unwrap_or(""))
                    .then_with(|| {
                        a.doc_no
                            .as_deref()
                            .unwrap_or("")
                            .cmp(b.doc_no.as_deref().unwrap_or(""))
                    })
                    .then_with(|| {
                        a.total_amount
                            .partial_cmp(&b.total_amount)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| {
                        compute_record_fingerprint(a, &key.kind)
                            .cmp(&compute_record_fingerprint(b, &key.kind))
                    })
            });

            let mut finalized_merged_records: Vec<CanonicalRecord> = Vec::new();
            for (idx, mut rec) in raw_merged_records.into_iter().enumerate() {
                rec.source_id = logical_id.clone();
                rec.id = format!("{}_rec_{:05}", logical_id, idx + 1);
                finalized_merged_records.push(rec);
            }

            let logical_ds = DataSource {
                id: logical_id.clone(),
                name: format!(
                    "{} (Hợp nhất {} phân đoạn)",
                    primary_rep.name,
                    sorted_group.len()
                ),
                file_path: primary_rep.file_path.clone(),
                sheet_name: primary_rep.sheet_name.clone(),
                kind: key.kind.clone(),
                role: key.role,
                header_row: primary_rep.header_row,
                data_start_row: primary_rep.data_start_row,
                column_mapping: primary_rep.column_mapping.clone(),
            };

            unified_data_sources.push(logical_ds);
            unified_records_map.insert(logical_id, finalized_merged_records);
        }
    }

    analysis.logical_sources_count = unified_data_sources.len();

    // Helper to resolve canonical id through chain of duplicates & partitions
    let resolve_id = |id: &str| -> String {
        let mut target = id.to_string();
        while let Some(mapped) = remap_map.get(&target) {
            if mapped == &target {
                break;
            }
            target = mapped.clone();
        }
        target
    };

    let final_valid_ids: HashSet<String> =
        unified_data_sources.iter().map(|s| s.id.clone()).collect();
    let mut filtered_session = session.clone();
    filtered_session.data_sources = unified_data_sources;

    // 1. Remap primary_source_id
    if let Some(ref pri_id) = filtered_session.primary_source_id {
        let canonical_pri = resolve_id(pri_id);
        if final_valid_ids.contains(&canonical_pri) {
            filtered_session.primary_source_id = Some(canonical_pri);
        } else if let Some(first_primary) = filtered_session
            .data_sources
            .iter()
            .find(|s| s.role == SourceRole::Primary || s.kind == DataSourceKind::EInvoice)
        {
            filtered_session.primary_source_id = Some(first_primary.id.clone());
        }
    }

    // 2. Remap required_source_ids
    if let Some(ref req_ids) = filtered_session.required_source_ids {
        let mut remapped_req: Vec<String> = Vec::new();
        for id in req_ids {
            let canonical_id = resolve_id(id);
            if final_valid_ids.contains(&canonical_id) && !remapped_req.contains(&canonical_id) {
                remapped_req.push(canonical_id);
            }
        }
        filtered_session.required_source_ids = Some(remapped_req);
    }

    // 3. Remap optional_source_ids
    if let Some(ref opt_ids) = filtered_session.optional_source_ids {
        let mut remapped_opt: Vec<String> = Vec::new();
        for id in opt_ids {
            let canonical_id = resolve_id(id);
            if final_valid_ids.contains(&canonical_id) && !remapped_opt.contains(&canonical_id) {
                remapped_opt.push(canonical_id);
            }
        }
        filtered_session.optional_source_ids = Some(remapped_opt);
    }

    Ok((filtered_session, unified_records_map, analysis))
}

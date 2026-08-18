use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use crate::models::{CanonicalRecord, DataSourceKind, ReconciliationSession, SourceRole};

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

/// Computes a stable canonical fingerprint for an individual accounting record
pub fn compute_record_fingerprint(r: &CanonicalRecord, kind: &DataSourceKind) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", kind).as_bytes());
    hasher.update(b"|");
    hasher.update(CanonicalRecord::normalize_doc_no(r.doc_no.as_deref().unwrap_or("")).as_bytes());
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
    hasher.update(r.date.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(
        CanonicalRecord::normalize_tax_id(r.partner_tax_id.as_deref().unwrap_or("")).as_bytes(),
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

#[derive(Debug, Clone)]
struct AcceptedSourceMeta {
    source_id: String,
    source_name: String,
    kind: DataSourceKind,
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
        let record_fps: HashSet<String> = recs
            .iter()
            .map(|r| compute_record_fingerprint(r, &ds.kind))
            .collect();

        let mut matched_relation = DatasetRelation::Unique;
        let mut is_eligible = true;
        let mut diag_msg = format!("Nguồn '{}' hợp lệ để đối chiếu.", ds.name);

        // Check against previously accepted sources of the same kind
        for prev in &accepted_sources {
            if ds.kind != prev.kind {
                continue;
            }

            // 1. Exact raw byte duplicate
            if let (Some(curr_raw), Some(prev_raw)) = (&raw_hash, &prev.raw_hash) {
                if curr_raw == prev_raw {
                    matched_relation = DatasetRelation::ExactDuplicate {
                        original_source_id: prev.source_id.clone(),
                        raw_sha256: curr_raw.clone(),
                    };
                    is_eligible = false;
                    diag_msg = format!(
                        "File '{}' giống hoàn toàn file '{}'. Bản sao không được tính lại.",
                        ds.name, prev.source_name
                    );
                    exact_dup_count += 1;
                    break;
                }
            }

            // 2. Canonical content duplicate (reformatted Excel file)
            if content_fp == prev.content_fingerprint && !recs.is_empty() {
                matched_relation = DatasetRelation::ContentDuplicate {
                    original_source_id: prev.source_id.clone(),
                    content_fingerprint: content_fp.clone(),
                };
                is_eligible = false;
                diag_msg = format!(
                    "Nội dung kế toán của '{}' trùng lặp hoàn toàn với '{}'. Không tính lần hai.",
                    ds.name, prev.source_name
                );
                content_dup_count += 1;
                break;
            }

            // 3. Subset / Partial Overlap checks
            if !record_fps.is_empty() && !prev.record_fingerprints.is_empty() {
                let overlap_count = record_fps.intersection(&prev.record_fingerprints).count();
                if overlap_count == record_fps.len()
                    && record_fps.len() < prev.record_fingerprints.len()
                {
                    // Current is strict subset of previous
                    matched_relation = DatasetRelation::SubsetDuplicate {
                        superset_source_id: prev.source_id.clone(),
                        record_count: record_fps.len(),
                        total_in_superset: prev.record_fingerprints.len(),
                    };
                    is_eligible = false;
                    diag_msg = format!(
                        "Tập dữ liệu '{}' ({} dòng) là tập con của '{}' ({} dòng). Bỏ qua để tránh tính lặp.",
                        ds.name, record_fps.len(), prev.source_name, prev.record_fingerprints.len()
                    );
                    subset_dup_count += 1;
                    break;
                } else if overlap_count > 0 {
                    let ratio = (overlap_count as f64 / record_fps.len() as f64) * 100.0;
                    matched_relation = DatasetRelation::PartialOverlap {
                        overlapping_source_id: prev.source_id.clone(),
                        overlap_count,
                        overlap_ratio: format!("{:.1}%", ratio),
                    };
                    is_eligible = false;
                    diag_msg = format!(
                        "Phát hiện trùng lặp {} dòng ({:.1}%) giữa '{}' và '{}'. Cần xác nhận của người dùng (fail-closed).",
                        overlap_count, ratio, ds.name, prev.source_name
                    );
                    partial_overlap_count += 1;
                    requires_user_confirmation = true;
                    break;
                }
            }
        }

        if is_eligible {
            accepted_sources.push(AcceptedSourceMeta {
                source_id: ds.id.clone(),
                source_name: ds.name.clone(),
                kind: ds.kind.clone(),
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
    let logical_count = unique_count;

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

/// Filter session and records map through the intake gate
/// Returns filtered ReconciliationSession and records map containing only unique logical sources
pub fn filter_reconciliation_session_and_records(
    session: &ReconciliationSession,
    source_records_map: &HashMap<String, Vec<CanonicalRecord>>,
    raw_file_hashes: &HashMap<String, String>,
) -> FilteredIntakeResult {
    let analysis = analyze_intake_data_sources(session, source_records_map, raw_file_hashes);

    if analysis.requires_user_confirmation {
        let overlap_details: Vec<String> = analysis
            .source_analyses
            .iter()
            .filter(|a| matches!(a.relation, DatasetRelation::PartialOverlap { .. }))
            .map(|a| a.diagnostic_message.clone())
            .collect();
        return Err(format!(
            "PARTIAL_OVERLAP: Yêu cầu xác nhận của người dùng trước khi hợp nhất dữ liệu: {}",
            overlap_details.join("; ")
        ));
    }

    let eligible_source_ids: HashSet<String> = analysis
        .source_analyses
        .iter()
        .filter(|a| a.is_eligible_for_reconciliation)
        .map(|a| a.source_id.clone())
        .collect();

    let mut filtered_session = session.clone();
    filtered_session
        .data_sources
        .retain(|ds| eligible_source_ids.contains(&ds.id));

    // Build remap lookup: duplicate_id -> original_source_id
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

    // Helper to resolve canonical id through chain of duplicates
    let resolve_id = |id: &str| -> String {
        let mut target = id.to_string();
        while let Some(mapped) = remap_map.get(&target) {
            target = mapped.clone();
        }
        target
    };

    // 1. Remap primary_source_id
    if let Some(ref pri_id) = filtered_session.primary_source_id {
        let canonical_pri = resolve_id(pri_id);
        if eligible_source_ids.contains(&canonical_pri) {
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
            if eligible_source_ids.contains(&canonical_id) && !remapped_req.contains(&canonical_id)
            {
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
            if eligible_source_ids.contains(&canonical_id) && !remapped_opt.contains(&canonical_id)
            {
                remapped_opt.push(canonical_id);
            }
        }
        filtered_session.optional_source_ids = Some(remapped_opt);
    }

    let mut filtered_records_map = HashMap::new();
    for (src_id, recs) in source_records_map {
        if eligible_source_ids.contains(src_id) {
            filtered_records_map.insert(src_id.clone(), recs.clone());
        }
    }

    Ok((filtered_session, filtered_records_map, analysis))
}

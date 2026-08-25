use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::{Deserialize, Serialize};

use crate::{CanonicalRecord, PartnerRecord, SalesAnalysisRecord};

pub const NORMALIZATION_SCHEMA_VERSION: &str = "v17-1";
pub const MAX_AUDIT_SOURCE_BYTES: usize = 256 * 1024 * 1024;

pub fn validate_audit_source_size(
    source_id: impl Into<String>,
    source_bytes: usize,
) -> Result<(), AuditExecutionError> {
    if source_bytes <= MAX_AUDIT_SOURCE_BYTES {
        return Ok(());
    }
    Err(AuditExecutionError::source(
        AuditErrorCode::OutOfMemoryRisk,
        source_id,
        "Nguồn vượt giới hạn kích thước an toàn của phiên hiện tại.",
        format!("source_bytes={source_bytes}; max_source_bytes={MAX_AUDIT_SOURCE_BYTES}"),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditErrorCode {
    SourceReadError,
    InvalidWorkbook,
    UnsupportedFormat,
    PasswordProtectedWorkbook,
    NoVisibleSheet,
    HeaderNotDetected,
    MappingIncomplete,
    InvalidDate,
    AccountingPeriodInvalid,
    CapabilityAmbiguous,
    CapabilityMissing,
    ControlPreconditionFailed,
    ControlExecutionFailed,
    ComplexityLimit,
    ExportFailed,
    OutOfMemoryRisk,
    Cancelled,
    InternalError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditErrorScope {
    Session,
    Source,
    Control,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditRecoverability {
    Retry,
    UserActionRequired,
    ContinueOtherControls,
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditExecutionError {
    pub code: AuditErrorCode,
    pub scope: AuditErrorScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_id: Option<String>,
    pub safe_user_message: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_detail: Option<Box<str>>,
    pub recoverability: AuditRecoverability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_action: Option<Box<str>>,
}

impl AuditExecutionError {
    pub fn source(
        code: AuditErrorCode,
        source_id: impl Into<String>,
        safe_user_message: impl Into<String>,
        technical_detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            scope: AuditErrorScope::Source,
            source_id: Some(source_id.into()),
            control_id: None,
            safe_user_message: safe_user_message.into().into_boxed_str(),
            technical_detail: Some(technical_detail.into().into_boxed_str()),
            recoverability: AuditRecoverability::ContinueOtherControls,
            recommended_action: Some(
                "Kiểm tra file, sheet và thiết lập cột rồi thử lại nguồn này.".into(),
            ),
        }
    }

    pub fn control(control_id: impl Into<String>, technical_detail: impl Into<String>) -> Self {
        Self {
            code: AuditErrorCode::ControlExecutionFailed,
            scope: AuditErrorScope::Control,
            source_id: None,
            control_id: Some(control_id.into()),
            safe_user_message:
                "Không thể hoàn tất kiểm tra này; các kiểm tra độc lập khác vẫn được giữ.".into(),
            technical_detail: Some(technical_detail.into().into_boxed_str()),
            recoverability: AuditRecoverability::ContinueOtherControls,
            recommended_action: Some("Rà soát nguồn và thử chạy lại kiểm tra.".into()),
        }
    }

    pub fn cancelled() -> Self {
        Self {
            code: AuditErrorCode::Cancelled,
            scope: AuditErrorScope::Session,
            source_id: None,
            control_id: None,
            safe_user_message: "Đã dừng phiên kiểm tra theo yêu cầu.".into(),
            technical_detail: None,
            recoverability: AuditRecoverability::Retry,
            recommended_action: Some("Có thể chạy lại khi sẵn sàng.".into()),
        }
    }

    pub fn invalid_period(detail: impl Into<String>) -> Self {
        Self {
            code: AuditErrorCode::AccountingPeriodInvalid,
            scope: AuditErrorScope::Session,
            source_id: None,
            control_id: None,
            safe_user_message: "Kỳ kế toán không hợp lệ.".into(),
            technical_detail: Some(detail.into().into_boxed_str()),
            recoverability: AuditRecoverability::UserActionRequired,
            recommended_action: Some("Chọn lại ngày bắt đầu và kết thúc kỳ.".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditRunStatus {
    Completed,
    Partial,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditStageMetrics {
    pub file_read_ms: u64,
    pub excel_parse_ms: u64,
    pub normalization_ms: u64,
    pub period_filtering_ms: u64,
    pub capability_detection_ms: u64,
    pub index_construction_ms: u64,
    pub control_planning_ms: u64,
    pub result_serialization_ms: u64,
    pub total_backend_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ControlTimingMetric {
    pub control_id: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditExecutionMetrics {
    pub stages: AuditStageMetrics,
    pub control_execution: Vec<ControlTimingMetric>,
    pub source_count: usize,
    pub normalized_record_count: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub estimated_cache_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_memory_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipc_inclusive_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipc_round_trip_overhead_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_render_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct AuditCancellationToken(Arc<AtomicBool>);

impl AuditCancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreparedSourceCacheKey {
    pub source_identity: String,
    pub raw_sha256: String,
    pub sheet_name: String,
    pub mapping_fingerprint: String,
    pub source_kind: String,
    pub normalization_schema_version: String,
}

#[derive(Debug, Clone)]
pub enum CachedNormalizedDataset {
    Transactional(Vec<CanonicalRecord>),
    PartnerMaster(Vec<PartnerRecord>),
    SalesAnalysis(Vec<SalesAnalysisRecord>),
}

impl CachedNormalizedDataset {
    pub fn record_count(&self) -> usize {
        match self {
            Self::Transactional(records) => records.len(),
            Self::PartnerMaster(records) => records.len(),
            Self::SalesAnalysis(records) => records.len(),
        }
    }

    pub fn estimated_bytes(&self) -> u64 {
        // Conservative bound used only to prevent unbounded process cache growth.
        const ESTIMATED_RECORD_BYTES: u64 = 2_048;
        (self.record_count() as u64).saturating_mul(ESTIMATED_RECORD_BYTES)
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    dataset: CachedNormalizedDataset,
    estimated_bytes: u64,
}

#[derive(Debug)]
pub struct PreparedSourceCache {
    entries: HashMap<PreparedSourceCacheKey, CacheEntry>,
    session_keys: HashMap<String, HashSet<PreparedSourceCacheKey>>,
    max_entries: usize,
    max_estimated_bytes: u64,
    estimated_bytes: u64,
}

impl Default for PreparedSourceCache {
    fn default() -> Self {
        Self::new(32, 1024 * 1024 * 1024)
    }
}

impl PreparedSourceCache {
    pub fn new(max_entries: usize, max_estimated_bytes: u64) -> Self {
        Self {
            entries: HashMap::new(),
            session_keys: HashMap::new(),
            max_entries,
            max_estimated_bytes,
            estimated_bytes: 0,
        }
    }

    pub fn get(&self, key: &PreparedSourceCacheKey) -> Option<CachedNormalizedDataset> {
        self.entries.get(key).map(|entry| entry.dataset.clone())
    }

    pub fn insert(
        &mut self,
        session_id: &str,
        key: PreparedSourceCacheKey,
        dataset: CachedNormalizedDataset,
    ) -> Result<(), AuditExecutionError> {
        let estimated = dataset.estimated_bytes();
        let existing = self
            .entries
            .get(&key)
            .map_or(0, |entry| entry.estimated_bytes);
        let projected = self
            .estimated_bytes
            .saturating_sub(existing)
            .saturating_add(estimated);
        if (!self.entries.contains_key(&key) && self.entries.len() >= self.max_entries)
            || projected > self.max_estimated_bytes
        {
            return Err(AuditExecutionError {
                code: AuditErrorCode::OutOfMemoryRisk,
                scope: AuditErrorScope::Session,
                source_id: None,
                control_id: None,
                safe_user_message: "Bộ hồ sơ vượt giới hạn bộ nhớ an toàn của phiên hiện tại."
                    .into(),
                technical_detail: Some(
                    format!(
                        "projected_cache_bytes={projected}; max_cache_bytes={}",
                        self.max_estimated_bytes
                    )
                    .into_boxed_str(),
                ),
                recoverability: AuditRecoverability::UserActionRequired,
                recommended_action: Some(
                    "Gỡ bớt nguồn hoặc đặt lại phiên trước khi thử lại.".into(),
                ),
            });
        }
        self.estimated_bytes = projected;
        self.entries.insert(
            key.clone(),
            CacheEntry {
                dataset,
                estimated_bytes: estimated,
            },
        );
        self.session_keys
            .entry(session_id.to_string())
            .or_default()
            .insert(key);
        Ok(())
    }

    pub fn retain_session_keys(
        &mut self,
        session_id: &str,
        active_keys: &HashSet<PreparedSourceCacheKey>,
    ) {
        let previous = self
            .session_keys
            .insert(session_id.to_string(), active_keys.clone())
            .unwrap_or_default();
        for key in previous.difference(active_keys) {
            let still_referenced = self.session_keys.values().any(|keys| keys.contains(key));
            if !still_referenced {
                if let Some(entry) = self.entries.remove(key) {
                    self.estimated_bytes =
                        self.estimated_bytes.saturating_sub(entry.estimated_bytes);
                }
            }
        }
    }

    pub fn reset_session(&mut self, session_id: &str) {
        let Some(keys) = self.session_keys.remove(session_id) else {
            return;
        };
        for key in keys {
            let still_referenced = self
                .session_keys
                .values()
                .any(|session| session.contains(&key));
            if !still_referenced {
                if let Some(entry) = self.entries.remove(&key) {
                    self.estimated_bytes =
                        self.estimated_bytes.saturating_sub(entry.estimated_bytes);
                }
            }
        }
    }

    pub fn estimated_bytes(&self) -> u64 {
        self.estimated_bytes
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(hash: &str, mapping: &str) -> PreparedSourceCacheKey {
        PreparedSourceCacheKey {
            source_identity: "source-a".to_string(),
            raw_sha256: hash.to_string(),
            sheet_name: "Sheet1".to_string(),
            mapping_fingerprint: mapping.to_string(),
            source_kind: "e_invoice".to_string(),
            normalization_schema_version: NORMALIZATION_SCHEMA_VERSION.to_string(),
        }
    }

    #[test]
    fn cache_invalidates_by_content_mapping_and_session_reset() {
        let mut cache = PreparedSourceCache::new(4, 1024 * 1024);
        cache
            .insert(
                "session",
                key("sha-a", "mapping-a"),
                CachedNormalizedDataset::Transactional(vec![]),
            )
            .expect("insert");
        assert!(cache.get(&key("sha-a", "mapping-a")).is_some());
        assert!(cache.get(&key("sha-b", "mapping-a")).is_none());
        assert!(cache.get(&key("sha-a", "mapping-b")).is_none());
        let mut other_identity = key("sha-a", "mapping-a");
        other_identity.source_identity = "source-b".to_string();
        assert!(cache.get(&other_identity).is_none());
        cache.reset_session("session");
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn cache_releases_removed_sources_and_reuses_normalization_across_period_views() {
        let mut cache = PreparedSourceCache::new(4, 1024 * 1024);
        let first = key("sha-a", "mapping-a");
        let removed = key("sha-b", "mapping-b");
        cache
            .insert(
                "session",
                first.clone(),
                CachedNormalizedDataset::Transactional(vec![]),
            )
            .expect("first source");
        cache
            .insert(
                "session",
                removed.clone(),
                CachedNormalizedDataset::Transactional(vec![]),
            )
            .expect("second source");

        // Accounting period is deliberately not part of the prepared-data key:
        // normalization is reused while each execution rebuilds the period view.
        assert!(cache.get(&first).is_some());
        cache.retain_session_keys("session", &HashSet::from([first.clone()]));
        assert!(cache.get(&first).is_some());
        assert!(cache.get(&removed).is_none());
    }

    #[test]
    fn large_file_size_tiers_are_bounded_without_allocating_test_workbooks() {
        for megabytes in [1_usize, 10, 50, 100] {
            assert!(validate_audit_source_size("synthetic", megabytes * 1024 * 1024).is_ok());
        }
        let error = validate_audit_source_size("synthetic", MAX_AUDIT_SOURCE_BYTES + 1)
            .expect_err("source above the process safety bound");
        assert_eq!(error.code, AuditErrorCode::OutOfMemoryRisk);
    }

    #[test]
    fn cache_rejects_unbounded_growth() {
        let mut cache = PreparedSourceCache::new(1, 1024);
        let error = cache
            .insert(
                "session",
                key("sha-a", "mapping-a"),
                CachedNormalizedDataset::Transactional(vec![CanonicalRecord::default()]),
            )
            .expect_err("record estimate must exceed bound");
        assert_eq!(error.code, AuditErrorCode::OutOfMemoryRisk);
    }
}

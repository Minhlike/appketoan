use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use reconciliation_core::{
    evaluate_partner_master_control, evaluate_sales_analysis_control,
    execute_audit_session_with_cancellation, execute_reconciliation, execute_reference_controls,
    export_reconciliation_to_excel, filter_reconciliation_session_and_records, inspect_excel_bytes,
    inspect_excel_file, normalize_data_source_rows, normalize_partner_master_rows,
    normalize_sales_analysis_rows, prepare_audit_session, read_sheet_rows,
    read_sheet_rows_from_bytes, validate_audit_source_size, AccountingPeriod, AuditErrorCode,
    AuditErrorScope, AuditExecutionError, AuditRecoverability, AuditRunStatus,
    AuditWorkspaceReport, CachedNormalizedDataset, CanonicalRecord, DataSource, DataSourceKind,
    ExcelFileMetadata, ExportSummary, PartnerRecord, PreparedSourceCacheKey, ReconciliationResult,
    ReconciliationSession, SalesAnalysisRecord, MAX_AUDIT_SOURCE_BYTES,
    NORMALIZATION_SCHEMA_VERSION,
};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

use crate::audit_runtime::AuditRuntimeState;

fn sha256_file_streaming(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Không thể mở file để băm: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Không thể đọc file để băm: {}", e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub fn cmd_inspect_excel_file(file_path: String) -> Result<ExcelFileMetadata, AuditExecutionError> {
    if let Ok(metadata) = std::fs::metadata(&file_path) {
        validate_audit_source_size(file_path.clone(), metadata.len() as usize)?;
    }
    inspect_excel_file(&file_path).map_err(|detail| classify_inspection_error(file_path, detail))
}

#[tauri::command]
pub fn cmd_inspect_excel_bytes(
    bytes: Vec<u8>,
    file_name: String,
) -> Result<ExcelFileMetadata, AuditExecutionError> {
    validate_audit_source_size(file_name.clone(), bytes.len())?;
    inspect_excel_bytes(&bytes, &file_name)
        .map_err(|detail| classify_inspection_error(file_name, detail))
}

fn run_reconciliation_inner(
    session: ReconciliationSession,
    file_bytes_map: Option<HashMap<String, Vec<u8>>>,
) -> Result<ReconciliationResult, String> {
    let mut source_records_map: HashMap<String, Vec<CanonicalRecord>> = HashMap::new();
    let mut raw_file_hashes: HashMap<String, String> = HashMap::new();
    let mut reference_controls = Vec::new();
    let mut partner_masters = Vec::new();
    let bytes_map = file_bytes_map.unwrap_or_default();

    for source in &session.data_sources {
        // Read raw rows and record raw SHA256
        let (raw_rows, header_cols, raw_hash) = if let Some(bytes) = bytes_map
            .get(&source.id)
            .or_else(|| bytes_map.get(&source.file_path))
        {
            let hash = format!("{:x}", Sha256::digest(bytes));
            let meta = inspect_excel_bytes(bytes, &source.name)
                .map_err(|e| format!("Lỗi kiểm tra file {}: {}", source.name, e))?;
            let sheet_meta = if source.sheet_name.trim().is_empty() {
                meta.sheets.first().ok_or_else(|| {
                    format!(
                        "SHEET_NOT_FOUND: File {} không chứa bất kỳ sheet nào",
                        source.name
                    )
                })?
            } else {
                meta.sheets
                    .iter()
                    .find(|s| s.name.trim().eq_ignore_ascii_case(source.sheet_name.trim()))
                    .ok_or_else(|| {
                        format!(
                            "SHEET_NOT_FOUND: Không tìm thấy sheet '{}' trong file {}",
                            source.sheet_name, source.name
                        )
                    })?
            };

            let rows = read_sheet_rows_from_bytes(bytes, &sheet_meta.name).map_err(|e| {
                format!(
                    "Lỗi đọc sheet '{}' trong file {}: {}",
                    sheet_meta.name, source.name, e
                )
            })?;

            (rows, sheet_meta.columns.clone(), Some(hash))
        } else if Path::new(&source.file_path).exists() {
            let hash = sha256_file_streaming(Path::new(&source.file_path))?;
            let meta = inspect_excel_file(&source.file_path)
                .map_err(|e| format!("Lỗi kiểm tra file {}: {}", source.name, e))?;
            let sheet_meta = if source.sheet_name.trim().is_empty() {
                meta.sheets.first().ok_or_else(|| {
                    format!(
                        "SHEET_NOT_FOUND: File {} không chứa bất kỳ sheet nào",
                        source.name
                    )
                })?
            } else {
                meta.sheets
                    .iter()
                    .find(|s| s.name.trim().eq_ignore_ascii_case(source.sheet_name.trim()))
                    .ok_or_else(|| {
                        format!(
                            "SHEET_NOT_FOUND: Không tìm thấy sheet '{}' trong file {}",
                            source.sheet_name, source.name
                        )
                    })?
            };

            let rows = read_sheet_rows(&source.file_path, &sheet_meta.name).map_err(|e| {
                format!(
                    "Lỗi đọc sheet '{}' trong file {}: {}",
                    sheet_meta.name, source.name, e
                )
            })?;

            (rows, sheet_meta.columns.clone(), Some(hash))
        } else {
            return Err(format!(
                "Không tìm thấy file nguồn '{}' tại '{}'",
                source.name, source.file_path
            ));
        };

        if let Some(h) = raw_hash {
            raw_file_hashes.insert(source.id.clone(), h);
        }

        match source.kind {
            DataSourceKind::PartnerMaster => {
                let records = normalize_partner_master_rows(source, &header_cols, &raw_rows);
                reference_controls
                    .push(evaluate_partner_master_control(source.id.clone(), &records));
                partner_masters.push((source.id.clone(), records));
            }
            DataSourceKind::SalesAnalysisReport => {
                let records = normalize_sales_analysis_rows(source, &header_cols, &raw_rows);
                reference_controls
                    .push(evaluate_sales_analysis_control(source.id.clone(), &records));
            }
            _ => {
                let records = normalize_data_source_rows(source, &header_cols, &raw_rows);
                source_records_map.insert(source.id.clone(), records);
            }
        }
    }

    // Cross-source partner identity is a read-only control. It does not merge
    // or rewrite canonical transaction records and therefore preserves source
    // provenance while allowing master + transaction audit sessions.
    if !source_records_map.is_empty() {
        let partner_identity_source_ids: std::collections::HashSet<&str> = session
            .data_sources
            .iter()
            .filter(|source| {
                matches!(
                    source.kind,
                    DataSourceKind::EInvoice
                        | DataSourceKind::SalesRegister
                        | DataSourceKind::Ledger511
                        | DataSourceKind::Ledger112
                        | DataSourceKind::Ledger131
                        | DataSourceKind::Ledger133
                        | DataSourceKind::Ledger3331
                )
            })
            .map(|source| source.id.as_str())
            .collect();
        for (master_source_id, masters) in &partner_masters {
            reference_controls.push(reconciliation_core::evaluate_partner_identity_cross_source(
                master_source_id.clone(),
                masters,
                source_records_map
                    .iter()
                    .filter(|(source_id, _)| {
                        partner_identity_source_ids.contains(source_id.as_str())
                    })
                    .flat_map(|(_, records)| records.iter()),
            ));
        }
    }

    if source_records_map.is_empty() {
        return execute_reference_controls(&session, reference_controls);
    }

    // Reference sources remain typed controls, but are allowed to coexist with
    // transactional sources in one audit session. They are deliberately not
    // injected into the transaction matcher as empty pseudo-datasets.
    let transactional_source_ids: std::collections::HashSet<&str> =
        source_records_map.keys().map(String::as_str).collect();
    let transactional_session =
        reconciliation_core::transactional_session_from(&session, &transactional_source_ids);

    // 1. Pass through intake dedup & dataset identity gate
    let (filtered_session, filtered_records_map, intake_analysis) =
        filter_reconciliation_session_and_records(
            &transactional_session,
            &source_records_map,
            &raw_file_hashes,
        )?;

    // 2. Execute core reconciliation on clean logical datasets
    let mut result = execute_reconciliation(&filtered_session, &filtered_records_map)?;
    result.intake_analysis = Some(intake_analysis);
    result.reference_controls = reference_controls;

    Ok(result)
}

#[tauri::command]
pub fn cmd_run_reconciliation(
    session: ReconciliationSession,
    file_bytes_map: Option<HashMap<String, Vec<u8>>>,
) -> Result<ReconciliationResult, AuditExecutionError> {
    run_reconciliation_inner(session, file_bytes_map)
        .map_err(|detail| AuditExecutionError::control("LEGACY_SCENARIO", detail))
}

fn classify_inspection_error(source_id: String, detail: String) -> AuditExecutionError {
    let lowercase = detail.to_ascii_lowercase();
    let code = if lowercase.contains("password") || lowercase.contains("encrypted") {
        AuditErrorCode::PasswordProtectedWorkbook
    } else if lowercase.contains("sheet") {
        AuditErrorCode::NoVisibleSheet
    } else if lowercase.contains("header") {
        AuditErrorCode::HeaderNotDetected
    } else if lowercase.contains("format") || lowercase.contains("excel") {
        AuditErrorCode::InvalidWorkbook
    } else {
        AuditErrorCode::SourceReadError
    };
    AuditExecutionError::source(code, source_id, "Không thể mở nguồn dữ liệu này.", detail)
}

fn classify_source_error(source: &DataSource, detail: String) -> AuditExecutionError {
    let lowercase = detail.to_ascii_lowercase();
    let code = if lowercase.contains("password") || lowercase.contains("encrypted") {
        AuditErrorCode::PasswordProtectedWorkbook
    } else if lowercase.contains("sheet") {
        AuditErrorCode::NoVisibleSheet
    } else if lowercase.contains("header") {
        AuditErrorCode::HeaderNotDetected
    } else if lowercase.contains("format") || lowercase.contains("excel") {
        AuditErrorCode::InvalidWorkbook
    } else {
        AuditErrorCode::SourceReadError
    };
    AuditExecutionError::source(
        code,
        source.id.clone(),
        "Không thể đọc nguồn này; các kiểm tra độc lập khác vẫn có thể tiếp tục.",
        detail,
    )
}

fn source_cache_key(
    source: &DataSource,
    raw_sha256: String,
) -> Result<PreparedSourceCacheKey, AuditExecutionError> {
    let mapping_fingerprint = serde_json::to_string(&(
        source.header_row,
        source.data_start_row,
        &source.column_mapping,
    ))
    .map_err(|error| AuditExecutionError {
        code: AuditErrorCode::InternalError,
        scope: AuditErrorScope::Source,
        source_id: Some(source.id.clone()),
        control_id: None,
        safe_user_message: "Không thể chuẩn bị thiết lập cột cho nguồn này.".into(),
        technical_detail: Some(error.to_string().into_boxed_str()),
        recoverability: AuditRecoverability::ContinueOtherControls,
        recommended_action: Some("Mở lại thiết lập cột rồi thử lại.".into()),
    })?;
    Ok(PreparedSourceCacheKey {
        source_identity: source.id.clone(),
        raw_sha256,
        sheet_name: source.sheet_name.clone(),
        mapping_fingerprint,
        source_kind: source.kind.as_str().to_string(),
        normalization_schema_version: NORMALIZATION_SCHEMA_VERSION.to_string(),
    })
}

fn office_lock_file(source: &DataSource) -> bool {
    source.name.trim_start().starts_with("~$")
        || Path::new(&source.file_path)
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("~$"))
}

fn run_audit_workspace_inner(
    runtime: &AuditRuntimeState,
    session_id: String,
    accounting_period: AccountingPeriod,
    data_sources: Vec<DataSource>,
    matching_tolerance_vnd: Decimal,
    date_tolerance_days: u32,
    file_bytes_map: Option<HashMap<String, Vec<u8>>>,
) -> Result<AuditWorkspaceReport, AuditExecutionError> {
    let backend_started = Instant::now();
    let cancellation = runtime.begin(&session_id)?;
    let result = (|| {
        let bytes_map = file_bytes_map.unwrap_or_default();
        let mut transactional: HashMap<String, Vec<CanonicalRecord>> = HashMap::new();
        let mut partner_masters: HashMap<String, Vec<PartnerRecord>> = HashMap::new();
        let mut sales_analysis: HashMap<String, Vec<SalesAnalysisRecord>> = HashMap::new();
        let mut provenance = HashMap::new();
        let mut source_errors = Vec::new();
        let mut successful_sources = Vec::new();
        let mut active_cache_keys = HashSet::new();
        let mut seen_hashes: HashMap<(String, String), String> = HashMap::new();
        let mut cache_evidence: HashMap<String, (bool, usize, usize)> = HashMap::new();
        let mut file_read_ms = 0_u64;
        let mut excel_parse_ms = 0_u64;
        let mut normalization_ms = 0_u64;
        let mut cache_hits = 0_usize;
        let mut cache_misses = 0_usize;
        let header_columns =
            |source: &DataSource, rows: &[Vec<String>]| -> Result<Vec<String>, String> {
                rows.get(source.header_row.saturating_sub(1) as usize)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "HEADER_ROW_NOT_FOUND: dòng tiêu đề {} không tồn tại trong {}",
                            source.header_row, source.name
                        )
                    })
            };

        for source in &data_sources {
            if cancellation.is_cancelled() {
                break;
            }
            if office_lock_file(source) {
                source_errors.push(AuditExecutionError::source(
                    AuditErrorCode::UnsupportedFormat,
                    source.id.clone(),
                    "Đã bỏ qua tệp tạm do Microsoft Office tạo.",
                    "office lock file (~$)",
                ));
                continue;
            }

            let source_bytes = if let Some(bytes) = bytes_map
                .get(&source.id)
                .or_else(|| bytes_map.get(&source.file_path))
            {
                Some((bytes.as_slice(), false))
            } else if Path::new(&source.file_path).exists() {
                None
            } else {
                source_errors.push(classify_source_error(
                    source,
                    format!("SOURCE_NOT_FOUND: {}", source.file_path),
                ));
                continue;
            };

            let owned_bytes;
            let (bytes, was_path_read) = if let Some(pair) = source_bytes {
                pair
            } else {
                if std::fs::metadata(&source.file_path)
                    .is_ok_and(|metadata| metadata.len() > MAX_AUDIT_SOURCE_BYTES as u64)
                {
                    source_errors.push(AuditExecutionError::source(
                        AuditErrorCode::OutOfMemoryRisk,
                        source.id.clone(),
                        "Nguồn vượt giới hạn kích thước an toàn của phiên hiện tại.",
                        format!("max_source_bytes={MAX_AUDIT_SOURCE_BYTES}"),
                    ));
                    continue;
                }
                let read_started = Instant::now();
                match std::fs::read(&source.file_path) {
                    Ok(bytes) => {
                        file_read_ms =
                            file_read_ms.saturating_add(read_started.elapsed().as_millis() as u64);
                        owned_bytes = bytes;
                        (owned_bytes.as_slice(), true)
                    }
                    Err(error) => {
                        source_errors.push(classify_source_error(source, error.to_string()));
                        continue;
                    }
                }
            };
            if let Err(error) = validate_audit_source_size(source.id.clone(), bytes.len()) {
                source_errors.push(error);
                continue;
            }
            let raw_hash = format!("{:x}", Sha256::digest(bytes));
            let duplicate_key = (raw_hash.clone(), source.sheet_name.clone());
            if let Some(original_source_id) = seen_hashes.get(&duplicate_key) {
                source_errors.push(AuditExecutionError {
                    code: AuditErrorCode::CapabilityAmbiguous,
                    scope: AuditErrorScope::Source,
                    source_id: Some(source.id.clone()),
                    control_id: None,
                    safe_user_message:
                        "Tệp này trùng hoàn toàn với nguồn đã nạp và đã được bỏ qua.".into(),
                    technical_detail: Some(
                        format!("exact duplicate of source {original_source_id}").into_boxed_str(),
                    ),
                    recoverability: AuditRecoverability::ContinueOtherControls,
                    recommended_action: Some("Giữ lại một bản duy nhất của tệp.".into()),
                });
                continue;
            }
            seen_hashes.insert(duplicate_key, source.id.clone());
            let key = source_cache_key(source, raw_hash.clone())?;
            active_cache_keys.insert(key.clone());
            provenance.insert(source.id.clone(), raw_hash);

            let cached = runtime.cache_lock()?.get(&key);
            let (dataset, cache_hit) = if let Some(dataset) = cached {
                cache_hits += 1;
                (dataset, true)
            } else {
                cache_misses += 1;
                let parse_started = Instant::now();
                let raw_rows = match read_sheet_rows_from_bytes(bytes, &source.sheet_name) {
                    Ok(rows) => rows,
                    Err(error) => {
                        source_errors.push(classify_source_error(source, error));
                        continue;
                    }
                };
                excel_parse_ms =
                    excel_parse_ms.saturating_add(parse_started.elapsed().as_millis() as u64);
                let header_cols = match header_columns(source, &raw_rows) {
                    Ok(columns) => columns,
                    Err(error) => {
                        source_errors.push(classify_source_error(source, error));
                        continue;
                    }
                };
                let normalization_started = Instant::now();
                let dataset = match source.kind {
                    DataSourceKind::PartnerMaster => CachedNormalizedDataset::PartnerMaster(
                        normalize_partner_master_rows(source, &header_cols, &raw_rows),
                    ),
                    DataSourceKind::SalesAnalysisReport => CachedNormalizedDataset::SalesAnalysis(
                        normalize_sales_analysis_rows(source, &header_cols, &raw_rows),
                    ),
                    _ => CachedNormalizedDataset::Transactional(normalize_data_source_rows(
                        source,
                        &header_cols,
                        &raw_rows,
                    )),
                };
                normalization_ms = normalization_ms
                    .saturating_add(normalization_started.elapsed().as_millis() as u64);
                runtime
                    .cache_lock()?
                    .insert(&session_id, key, dataset.clone())?;
                (dataset, false)
            };

            let record_count = dataset.record_count();
            let read_count = usize::from(!cache_hit || was_path_read);
            cache_evidence.insert(source.id.clone(), (cache_hit, read_count, record_count));
            match dataset {
                CachedNormalizedDataset::PartnerMaster(records) => {
                    partner_masters.insert(source.id.clone(), records);
                }
                CachedNormalizedDataset::SalesAnalysis(records) => {
                    sales_analysis.insert(source.id.clone(), records);
                }
                CachedNormalizedDataset::Transactional(records) => {
                    transactional.insert(source.id.clone(), records);
                }
            }
            successful_sources.push(source.clone());
        }

        runtime
            .cache_lock()?
            .retain_session_keys(&session_id, &active_cache_keys);
        let mut audit = prepare_audit_session(
            session_id.clone(),
            accounting_period,
            successful_sources,
            transactional,
            partner_masters,
            sales_analysis,
            provenance,
        )
        .map_err(AuditExecutionError::invalid_period)?;
        audit.metrics.stages.file_read_ms = file_read_ms;
        audit.metrics.stages.excel_parse_ms = excel_parse_ms;
        audit.metrics.stages.normalization_ms = normalization_ms;
        audit.metrics.cache_hits = cache_hits;
        audit.metrics.cache_misses = cache_misses;
        audit.metrics.estimated_cache_bytes = runtime.cache_lock()?.estimated_bytes();
        for evidence in &mut audit.source_reuse {
            if let Some((cache_hit, read_count, record_count)) =
                cache_evidence.get(&evidence.source_id)
            {
                evidence.cache_hit = *cache_hit;
                evidence.read_count = *read_count;
                evidence.normalize_count = usize::from(!cache_hit);
                evidence.normalized_record_count = *record_count;
            }
        }
        let mut report = execute_audit_session_with_cancellation(
            audit,
            matching_tolerance_vnd,
            date_tolerance_days,
            &cancellation,
        )
        .map_err(|detail| AuditExecutionError::control("AUDIT_SESSION", detail))?;
        report.errors.extend(source_errors);
        if cancellation.is_cancelled() {
            report.run_status = AuditRunStatus::Cancelled;
        } else if !report.errors.is_empty() {
            report.run_status = if report.source_catalog.sources.is_empty() {
                AuditRunStatus::Failed
            } else {
                AuditRunStatus::Partial
            };
        }
        let serialization_started = Instant::now();
        serde_json::to_vec(&report).map_err(|error| AuditExecutionError {
            code: AuditErrorCode::InternalError,
            scope: AuditErrorScope::Session,
            source_id: None,
            control_id: None,
            safe_user_message: "Không thể chuẩn bị kết quả để hiển thị.".into(),
            technical_detail: Some(error.to_string().into_boxed_str()),
            recoverability: AuditRecoverability::Fatal,
            recommended_action: Some("Đặt lại phiên và thử lại.".into()),
        })?;
        report.metrics.stages.result_serialization_ms =
            serialization_started.elapsed().as_millis() as u64;
        report.metrics.stages.total_backend_ms = backend_started.elapsed().as_millis() as u64;
        Ok(report)
    })();
    let finish_result = runtime.finish(&session_id);
    match (result, finish_result) {
        (Ok(report), Ok(())) => Ok(report),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

#[tauri::command]
pub async fn cmd_run_audit_workspace(
    state: tauri::State<'_, AuditRuntimeState>,
    session_id: String,
    accounting_period: AccountingPeriod,
    data_sources: Vec<DataSource>,
    matching_tolerance_vnd: Decimal,
    date_tolerance_days: u32,
    file_bytes_map: Option<HashMap<String, Vec<u8>>>,
) -> Result<AuditWorkspaceReport, AuditExecutionError> {
    let runtime = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_audit_workspace_inner(
            &runtime,
            session_id,
            accounting_period,
            data_sources,
            matching_tolerance_vnd,
            date_tolerance_days,
            file_bytes_map,
        )
    })
    .await
    .map_err(|error| AuditExecutionError {
        code: AuditErrorCode::InternalError,
        scope: AuditErrorScope::Session,
        source_id: None,
        control_id: None,
        safe_user_message: "Tiến trình kiểm tra đã dừng ngoài dự kiến.".into(),
        technical_detail: Some(error.to_string().into_boxed_str()),
        recoverability: AuditRecoverability::Fatal,
        recommended_action: Some("Đặt lại phiên hoặc khởi động lại ứng dụng.".into()),
    })?
}

#[tauri::command]
pub fn cmd_cancel_audit_workspace(
    state: tauri::State<'_, AuditRuntimeState>,
    session_id: String,
) -> Result<bool, AuditExecutionError> {
    state.cancel(&session_id)
}

#[tauri::command]
pub fn cmd_reset_audit_workspace(
    state: tauri::State<'_, AuditRuntimeState>,
    session_id: String,
) -> Result<(), AuditExecutionError> {
    state.reset(&session_id)
}

#[tauri::command]
pub fn cmd_export_reconciliation_report(
    result: ReconciliationResult,
    output_path: Option<String>,
) -> Result<ExportSummary, AuditExecutionError> {
    let target_path = if let Some(p) = output_path {
        PathBuf::from(p)
    } else {
        let download_dir = std::env::var("USERPROFILE")
            .map(|p| PathBuf::from(p).join("Downloads"))
            .unwrap_or_else(|_| std::env::temp_dir());
        let safe_session_id: String = result
            .session_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                    character
                } else {
                    '_'
                }
            })
            .take(80)
            .collect();
        let file_name = format!("Bao_Cao_Doi_Chieu_{safe_session_id}.xlsx");
        download_dir.join(file_name)
    };

    export_reconciliation_to_excel(&result, &target_path).map_err(|detail| AuditExecutionError {
        code: AuditErrorCode::ExportFailed,
        scope: AuditErrorScope::Export,
        source_id: None,
        control_id: None,
        safe_user_message: "Không thể lưu báo cáo; kết quả hiện tại vẫn được giữ trong phiên."
            .into(),
        technical_detail: Some(detail.into_boxed_str()),
        recoverability: AuditRecoverability::Retry,
        recommended_action: Some(
            "Đóng file báo cáo đang mở hoặc chọn vị trí khác rồi thử lại.".into(),
        ),
    })
}

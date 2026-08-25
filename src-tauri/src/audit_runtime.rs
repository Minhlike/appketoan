use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use reconciliation_core::{
    AuditCancellationToken, AuditErrorCode, AuditErrorScope, AuditExecutionError,
    AuditRecoverability, PreparedSourceCache,
};

#[derive(Clone, Default)]
pub struct AuditRuntimeState {
    cache: Arc<Mutex<PreparedSourceCache>>,
    cancellations: Arc<Mutex<HashMap<String, AuditCancellationToken>>>,
}

impl AuditRuntimeState {
    pub fn begin(&self, session_id: &str) -> Result<AuditCancellationToken, AuditExecutionError> {
        let mut cancellations = self.cancellations_lock()?;
        if cancellations.contains_key(session_id) {
            return Err(AuditExecutionError {
                code: AuditErrorCode::ControlPreconditionFailed,
                scope: AuditErrorScope::Session,
                source_id: None,
                control_id: None,
                safe_user_message: "Phiên này đang được kiểm tra; không thể chạy trùng.".into(),
                technical_detail: Some("duplicate active session id".into()),
                recoverability: AuditRecoverability::Retry,
                recommended_action: Some(
                    "Chờ phiên hiện tại hoàn tất hoặc dừng phiên trước.".into(),
                ),
            });
        }
        let token = AuditCancellationToken::default();
        cancellations.insert(session_id.to_string(), token.clone());
        Ok(token)
    }

    pub fn cancel(&self, session_id: &str) -> Result<bool, AuditExecutionError> {
        let tokens = self.cancellations_lock()?;
        if let Some(token) = tokens.get(session_id) {
            token.cancel();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn finish(&self, session_id: &str) -> Result<(), AuditExecutionError> {
        self.cancellations_lock()?.remove(session_id);
        Ok(())
    }

    pub fn reset(&self, session_id: &str) -> Result<(), AuditExecutionError> {
        if let Some(token) = self.cancellations_lock()?.remove(session_id) {
            token.cancel();
        }
        self.cache_lock()?.reset_session(session_id);
        Ok(())
    }

    pub fn cache_lock(&self) -> Result<MutexGuard<'_, PreparedSourceCache>, AuditExecutionError> {
        self.cache.lock().map_err(|_| lock_error("prepared cache"))
    }

    fn cancellations_lock(
        &self,
    ) -> Result<MutexGuard<'_, HashMap<String, AuditCancellationToken>>, AuditExecutionError> {
        self.cancellations
            .lock()
            .map_err(|_| lock_error("cancellation registry"))
    }
}

fn lock_error(component: &str) -> AuditExecutionError {
    AuditExecutionError {
        code: AuditErrorCode::InternalError,
        scope: AuditErrorScope::Session,
        source_id: None,
        control_id: None,
        safe_user_message: "Phiên kiểm tra không ở trạng thái an toàn để tiếp tục.".into(),
        technical_detail: Some(format!("poisoned {component} lock").into_boxed_str()),
        recoverability: AuditRecoverability::Fatal,
        recommended_action: Some("Đặt lại phiên hoặc khởi động lại ứng dụng.".into()),
    }
}

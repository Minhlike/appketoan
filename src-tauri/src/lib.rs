pub mod audit_runtime;
pub mod commands;
pub use reconciliation_core::models;

pub use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(audit_runtime::AuditRuntimeState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            cmd_inspect_excel_file,
            cmd_inspect_excel_bytes,
            cmd_run_audit_workspace,
            cmd_cancel_audit_workspace,
            cmd_reset_audit_workspace,
            cmd_run_reconciliation,
            cmd_export_reconciliation_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

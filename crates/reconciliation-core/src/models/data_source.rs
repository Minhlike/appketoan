use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceKind {
    EInvoice,
    Ledger511,
    Ledger3331,
    Ledger133,
    Ledger131,
    BankStatement,
    CashBook,
    BranchLedger,
    #[default]
    Custom,
}

impl DataSourceKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::EInvoice => "Hóa đơn điện tử",
            Self::Ledger511 => "Sổ cái TK 511 (Doanh thu)",
            Self::Ledger3331 => "Sổ cái TK 3331 (Thuế GTGT)",
            Self::Ledger133 => "Sổ cái TK 133 (Thuế đầu vào)",
            Self::Ledger131 => "Sổ công nợ TK 131 (Phải thu)",
            Self::BankStatement => "Sao kê ngân hàng",
            Self::CashBook => "Sổ quỹ tiền mặt",
            Self::BranchLedger => "Sổ chi nhánh",
            Self::Custom => "Khác / Tùy chỉnh (Cần xác định)",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_no_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_code_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_code_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_tax_id_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub buyer_tax_id_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_tax_id_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_name_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretax_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debit_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_rate_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub debit_account_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_account_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub voucher_no_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account_column: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSource {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub sheet_name: String,
    pub kind: DataSourceKind,
    #[serde(default = "default_header_row")]
    pub header_row: u32,
    #[serde(default = "default_data_start_row")]
    pub data_start_row: u32,
    #[serde(default)]
    pub column_mapping: ColumnMapping,
}

fn default_header_row() -> u32 {
    1
}

fn default_data_start_row() -> u32 {
    2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetMetadata {
    pub name: String,
    pub total_rows: usize,
    pub total_cols: usize,
    pub detected_header_row: u32,
    pub detected_data_start_row: u32,
    pub columns: Vec<String>,
    pub suggested_mapping: ColumnMapping,
    pub suggested_kind: DataSourceKind,
    pub confidence_score: f64,
    pub preview_rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcelFileMetadata {
    pub file_path: String,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub sheets: Vec<SheetMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationSession {
    pub session_id: String,
    pub scenario_name: String,
    pub primary_source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_source_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_source_ids: Option<Vec<String>>,
    pub data_sources: Vec<DataSource>,
    pub matching_tolerance_vnd: Decimal,
    pub date_tolerance_days: u32,
    pub enable_aggregate_match: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub output_path: String,
    pub file_size_bytes: u64,
    pub total_groups_exported: usize,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_source_serialization() {
        let ds = DataSource {
            id: "ds_01".to_string(),
            name: "Hóa đơn điện tử Tháng 1".to_string(),
            file_path: "C:/data/hd_t1.xlsx".to_string(),
            sheet_name: "Sheet1".to_string(),
            kind: DataSourceKind::EInvoice,
            header_row: 1,
            data_start_row: 2,
            column_mapping: ColumnMapping {
                doc_no_column: Some("Số hóa đơn".to_string()),
                total_amount_column: Some("Tổng tiền thanh toán".to_string()),
                credit_amount_column: Some("Phát sinh Có".to_string()),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&ds).expect("Serialization failed");
        assert!(json.contains("e_invoice"));
        assert!(json.contains("ds_01"));
        assert!(json.contains("creditAmountColumn"));

        let deserialized: DataSource = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(ds, deserialized);
    }
}

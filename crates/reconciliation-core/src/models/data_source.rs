use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Custom,
}

impl Default for DataSourceKind {
    fn default() -> Self {
        Self::Custom
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
    pub series_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_code_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_tax_id_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_name_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pretax_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_amount_column: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_amount_column: Option<String>,

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
                total_amount_column: Some("Tổng tiền".to_string()),
                ..Default::default()
            },
        };

        let json = serde_json::to_string(&ds).expect("Serialization failed");
        assert!(json.contains("e_invoice"));
        assert!(json.contains("ds_01"));

        let deserialized: DataSource = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(ds, deserialized);
    }
}

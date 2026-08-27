import type {
  DocumentErrorCode,
  DocumentField,
  DocumentFieldCheck,
  DocumentIntegrityCase,
  DocumentIntegrityResult,
} from "../types/dataContract";

export type DocumentFilterId =
  | "ALL"
  | "INVOICE_REGISTER_EXACT"
  | "FULLY_MATCHED"
  | "DATE_MISMATCH"
  | "INVOICE_NUMBER_MISMATCH"
  | "PRETAX_MISMATCH"
  | "VAT_MISMATCH"
  | "TOTAL_MISMATCH"
  | "MISSING_IN_BK"
  | "EXTRA_IN_BK"
  | "DUPLICATE_INVOICE_NUMBER"
  | "AMBIGUOUS_MATCH"
  | "INVOICE_LIFECYCLE_NEEDS_REVIEW";

export interface DocumentFilterItem {
  id: DocumentFilterId;
  label: string;
  count: number;
}

const hasError = (document: DocumentIntegrityCase, code: DocumentErrorCode) =>
  document.errors.some((error) => error.code === code);

export const isInvoiceRegisterExact = (document: DocumentIntegrityCase) => {
  const checks = document.fieldChecks.filter(
    (check) => check.scope === "INVOICE_TO_SALES_REGISTER"
  );
  return Boolean(document.invoice && document.salesRegister)
    && checks.length === 5
    && checks.every((check) => check.status === "MATCH");
};

export function matchesDocumentFilter(
  document: DocumentIntegrityCase,
  filterId: DocumentFilterId
) {
  switch (filterId) {
    case "ALL":
      return true;
    case "INVOICE_REGISTER_EXACT":
      return isInvoiceRegisterExact(document);
    case "FULLY_MATCHED":
      return Boolean(document.invoice) && document.status === "FULLY_MATCHED";
    case "DUPLICATE_INVOICE_NUMBER":
      return Boolean(document.salesRegister) && hasError(document, filterId);
    default:
      return hasError(document, filterId);
  }
}

const filterLabels: Array<[DocumentFilterId, string]> = [
  ["INVOICE_REGISTER_EXACT", "Thuế ↔ BK khớp"],
  ["FULLY_MATCHED", "Khớp đủ 3 nguồn"],
  ["DATE_MISMATCH", "Sai ngày"],
  ["INVOICE_NUMBER_MISMATCH", "Sai số HĐ"],
  ["PRETAX_MISMATCH", "Sai tiền BK"],
  ["VAT_MISMATCH", "Sai VAT"],
  ["TOTAL_MISMATCH", "Sai phải thu"],
  ["MISSING_IN_BK", "Thiếu BK"],
  ["EXTRA_IN_BK", "Thừa BK"],
  ["DUPLICATE_INVOICE_NUMBER", "Trùng Số CT"],
  ["AMBIGUOUS_MATCH", "Mơ hồ"],
  ["INVOICE_LIFECYCLE_NEEDS_REVIEW", "Lifecycle cần rà soát"],
];

export function documentFilterItems(result: DocumentIntegrityResult): DocumentFilterItem[] {
  return filterLabels.map(([id, label]) => ({
    id,
    label,
    count: result.documents.filter((document) => matchesDocumentFilter(document, id)).length,
  }));
}

const invoiceRegisterLabels: Record<DocumentField, string> = {
  DATE: "Ngày Thuế ↔ Ngày BK",
  INVOICE_NUMBER: "Số HĐ Thuế ↔ Số CT BK",
  PRETAX: "Tiền chưa thuế Thuế ↔ Tiền BK (doanh thu)",
  VAT: "VAT Thuế ↔ Thuế BK",
  TOTAL: "Tổng thanh toán Thuế ↔ Phải thu BK",
};

const invoiceLedgerLabels: Record<DocumentField, string> = {
  DATE: "Ngày Thuế ↔ Ngày Sổ cái TK511",
  INVOICE_NUMBER: "Số HĐ Thuế ↔ Số CT Sổ cái TK511",
  PRETAX: "Tiền chưa thuế Thuế ↔ Phát sinh Có TK511",
  VAT: "VAT — không thuộc phạm vi TK511",
  TOTAL: "Phải thu — không thuộc phạm vi TK511",
};

export function documentCheckLabel(check: DocumentFieldCheck) {
  return check.scope === "INVOICE_TO_SALES_REGISTER"
    ? invoiceRegisterLabels[check.field]
    : invoiceLedgerLabels[check.field];
}

export function notCheckedReason(check: DocumentFieldCheck, ledger511Checked: boolean) {
  if (check.scope !== "INVOICE_TO_LEDGER511") return "CHƯA ĐỐI CHIẾU";
  if (check.field === "VAT" || check.field === "TOTAL") {
    return "KHÔNG THUỘC PHẠM VI TK511";
  }
  return ledger511Checked ? "CHƯA ĐỐI CHIẾU" : "CHƯA TẢI FILE SỔ CÁI TK511";
}

export function documentStatusLabel(
  document: DocumentIntegrityCase,
  ledger511Checked: boolean
) {
  if (document.status === "FULLY_MATCHED") {
    return "Khớp đủ Thuế ↔ BK ↔ Sổ cái TK511";
  }

  const errorCodes = document.errors.map((error) => error.code);
  if (!ledger511Checked && isInvoiceRegisterExact(document)) {
    const lifecycle = errorCodes.includes("INVOICE_LIFECYCLE_NEEDS_REVIEW")
      ? " · Lifecycle cần rà soát"
      : "";
    return `Thuế ↔ BK khớp 5 trường${lifecycle} · Chưa tải Sổ cái TK511`;
  }

  return errorCodes.join(", ") || "Cần rà soát";
}

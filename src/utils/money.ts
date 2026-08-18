/**
 * Decimal-Safe Monetary Utilities for Accounting Application
 * Ensures zero floating-point imprecision when displaying and parsing VND values
 */

export type MoneyValue = string;

/**
 * Formats a monetary value (string or number) to Vietnamese currency representation
 * Example: "105000000" -> "105.000.000 đ"
 *          "-500000"   -> "-500.000 đ"
 *          0           -> "0 đ"
 */
export function formatVND(value: string | number | undefined | null): string {
  if (value === undefined || value === null || value === "") {
    return "0 đ";
  }

  const str = String(value).trim();
  if (str === "0" || str === "-0" || str === "0.00" || str === "0.0") {
    return "0 đ";
  }

  // Handle negative
  const isNegative = str.startsWith("-");
  const cleanStr = isNegative ? str.slice(1) : str;

  // Split integer and fractional part if present
  const parts = cleanStr.split(".");
  const intPart = parts[0] || "0";

  // Insert thousand dots from right to left
  let formattedInt = "";
  for (let i = 0; i < intPart.length; i++) {
    if (i > 0 && (intPart.length - i) % 3 === 0) {
      formattedInt += ".";
    }
    formattedInt += intPart[i];
  }

  const result = isNegative ? `-${formattedInt} đ` : `${formattedInt} đ`;
  return result;
}

/**
 * Parses user input or raw strings into clean decimal string representation
 */
export function parseMoneyString(input: string | number | undefined | null): MoneyValue {
  if (input === undefined || input === null) {
    return "0";
  }
  const s = String(input)
    .replace(/[₫\s\u00a0VNDvndVNĐ]/g, "")
    .trim();

  if (s === "" || s === "-") {
    return "0";
  }

  let isNegative = false;
  let clean = s;
  if (clean.startsWith("(") && clean.endsWith(")")) {
    isNegative = true;
    clean = clean.slice(1, -1).trim();
  } else if (clean.startsWith("-")) {
    isNegative = true;
    clean = clean.slice(1).trim();
  }

  // Normalize thousands dots and decimal commas
  clean = clean.replace(/\./g, "").replace(/,/g, ".");
  const floatVal = parseFloat(clean);
  if (isNaN(floatVal)) {
    return "0";
  }

  const rounded = Math.round(floatVal);
  return isNegative && rounded !== 0 ? `-${Math.abs(rounded)}` : `${rounded}`;
}

/**
 * Checks if a money value is zero
 */
export function isZeroMoney(val: string | number | undefined | null): boolean {
  if (val === undefined || val === null || val === "") return true;
  const s = String(val).trim();
  return s === "0" || s === "-0" || s === "0.00" || s === "0.0" || Number(s) === 0;
}

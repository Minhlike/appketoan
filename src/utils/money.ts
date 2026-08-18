/**
 * Decimal-Safe Monetary Utilities for Accounting Application
 * Ensures zero floating-point imprecision when displaying and parsing VND values
 */

export type MoneyValue = string;

/**
 * Formats a monetary value (string or number) to Vietnamese currency representation
 * Preserves decimal fractions if present (e.g. "1250000.50" -> "1.250.000,50 đ")
 * Example: "105000000"  -> "105.000.000 đ"
 *          "-500000"    -> "-500.000 đ"
 *          "1250000.50" -> "1.250.000,50 đ"
 *          0            -> "0 đ"
 */
export function formatVND(value: string | number | undefined | null): string {
  if (value === undefined || value === null || value === "") {
    return "0 đ";
  }

  const str = String(value).trim();
  if (str === "0" || str === "-0" || str === "0.00" || str === "0.0" || str === "0,00" || str === "0,0") {
    return "0 đ";
  }

  // Handle negative
  const isNegative = str.startsWith("-");
  const cleanStr = isNegative ? str.slice(1) : str;

  // Split integer and fractional part if present
  const parts = cleanStr.includes(",") ? cleanStr.split(",") : cleanStr.split(".");
  const intPart = parts[0] || "0";
  const fracPart = parts[1];

  // Insert thousand dots from right to left on integer part
  let formattedInt = "";
  for (let i = 0; i < intPart.length; i++) {
    if (i > 0 && (intPart.length - i) % 3 === 0) {
      formattedInt += ".";
    }
    formattedInt += intPart[i];
  }

  const formattedValue =
    fracPart !== undefined && fracPart.length > 0 && fracPart !== "0" && fracPart !== "00"
      ? `${formattedInt},${fracPart}`
      : formattedInt;

  return isNegative ? `-${formattedValue} đ` : `${formattedValue} đ`;
}

/**
 * Parses user input or raw strings into clean exact decimal string representation
 * Avoids any floating-point conversions (parseFloat / Math.round / toFixed)
 */
export function parseMoneyString(input: string | number | undefined | null): MoneyValue {
  if (input === undefined || input === null) {
    return "0";
  }
  const s = String(input)
    .replace(/[₫đĐ\s\u00a0VNDvndVNĐ]/g, "")
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

  // If string contains comma, treat comma as decimal separator (Vietnamese standard)
  if (clean.includes(",")) {
    const [intP, fracP] = clean.split(",");
    const normInt = intP.replace(/\./g, "").replace(/\s/g, "") || "0";
    const res = fracP && fracP !== "0" && fracP !== "00" ? `${normInt}.${fracP}` : normInt;
    return isNegative && res !== "0" ? `-${res}` : res;
  }

  // If multiple dots, they are thousand separators
  const dotCount = (clean.match(/\./g) || []).length;
  if (dotCount > 1) {
    const res = clean.replace(/\./g, "");
    return isNegative && res !== "0" ? `-${res}` : res;
  } else if (dotCount === 1) {
    const parts = clean.split(".");
    if (parts[1].length === 3 && parts[0].length <= 3) {
      // Thousand separator (e.g. 500.000)
      const res = clean.replace(/\./g, "");
      return isNegative && res !== "0" ? `-${res}` : res;
    } else {
      // Decimal point (e.g. 1250000.50)
      return isNegative && clean !== "0" ? `-${clean}` : clean;
    }
  }

  return isNegative && clean !== "0" ? `-${clean}` : clean;
}

/**
 * Checks if a money value is zero strictly without float conversion
 */
export function isZeroMoney(val: string | number | undefined | null): boolean {
  if (val === undefined || val === null || val === "") return true;
  const s = String(val).trim().replace(/[₫đĐ\s\u00a0VNDvndVNĐ]/g, "");
  if (s === "0" || s === "-0" || s === "0.00" || s === "0.0" || s === "0,00" || s === "0,0") return true;
  const clean = s.replace(/[-+]/g, "").replace(/\./g, "").replace(/,/g, "");
  return /^0*$/.test(clean);
}

/**
 * Normalizes user input for monetary settings/tolerance without floating-point conversion
 */
export function normalizeMoneyInput(val: string): MoneyValue {
  const clean = val.replace(/[^0-9.-]/g, "");
  return clean === "" ? "0" : clean;
}

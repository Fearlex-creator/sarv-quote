const ARABIC_INDIC = "٠١٢٣٤٥٦٧٨٩";
const PERSIAN = "۰۱۲۳۴۵۶۷۸۹";
// Digits typed in any script become a whole number; the display unit becomes IRR.
export function toIrr(text: string, unit: "Rial" | "Toman"): bigint {
  const digits = text
    .replace(/[۰-۹]/g, (d) => String(PERSIAN.indexOf(d)))
    .replace(/[٠-٩]/g, (d) => String(ARABIC_INDIC.indexOf(d)))
    .replace(/\D/g, "");
  return digits ? BigInt(digits) * (unit === "Toman" ? 10n : 1n) : 0n;
}
// IRR back to the text a person would type in the display unit.
export function fromIrr(value: bigint | string | null, unit: "Rial" | "Toman") {
  if (value === null || value === "") return "";
  const n = BigInt(value);
  return String(unit === "Toman" ? n / 10n : n);
}
// "1.5" or "۱٫۵" → quantity thousandths, or null when it is not a number.
export function toThousandths(text: string): bigint | null {
  const clean = text
    .replace(/[۰-۹]/g, (d) => String(PERSIAN.indexOf(d)))
    .replace(/[٠-٩]/g, (d) => String(ARABIC_INDIC.indexOf(d)))
    .replace(/[٫,]/g, ".")
    .trim();
  const m = /^(\d+)(?:\.(\d{1,3}))?$/.exec(clean);
  if (!m) return null;
  return BigInt(m[1]) * 1000n + BigInt((m[2] ?? "").padEnd(3, "0") || "0");
}

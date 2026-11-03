// Amount in Persian words («مبلغ به حروف»), exact for any non-negative integer below 10^18.
// Works on decimal strings or bigint: money never passes through a JS number.
import { normalizeDigits } from "./normalize-fa.ts";

const ONES = [
  "صفر",
  "یک",
  "دو",
  "سه",
  "چهار",
  "پنج",
  "شش",
  "هفت",
  "هشت",
  "نه",
  "ده",
  "یازده",
  "دوازده",
  "سیزده",
  "چهارده",
  "پانزده",
  "شانزده",
  "هفده",
  "هجده",
  "نوزده",
];
const TENS = [
  "",
  "",
  "بیست",
  "سی",
  "چهل",
  "پنجاه",
  "شصت",
  "هفتاد",
  "هشتاد",
  "نود",
];
const HUNDREDS = [
  "",
  "صد",
  "دویست",
  "سیصد",
  "چهارصد",
  "پانصد",
  "ششصد",
  "هفتصد",
  "هشتصد",
  "نهصد",
];
const SCALES = ["", "هزار", "میلیون", "میلیارد", "تریلیون", "کوادریلیون"];
const AND = " و ";

function belowThousand(n: number): string {
  const parts: string[] = [];
  const h = Math.floor(n / 100);
  const rest = n % 100;
  if (h) parts.push(HUNDREDS[h]);
  if (rest) {
    if (rest < 20) parts.push(ONES[rest]);
    else {
      const t = Math.floor(rest / 10);
      const o = rest % 10;
      parts.push(o ? TENS[t] + AND + ONES[o] : TENS[t]);
    }
  }
  return parts.join(AND);
}

/**
 * Whole number from a decimal string in any digit system. The sign is read explicitly
 * (hyphens are otherwise grouping noise in pasted values) and any stray hyphen, decimal
 * point or letter rejects the input instead of silently changing the amount.
 */
function parseWholeNumber(value: string): bigint | undefined {
  const trimmed = value.trim();
  const negative = /^[-\u2212]/.test(trimmed);
  const body = negative ? trimmed.slice(1) : trimmed;
  if (/[-\u2212]/.test(body)) return undefined;
  const digits = normalizeDigits(body);
  if (!/^[0-9]+$/.test(digits)) return undefined;
  return negative ? -BigInt(digits) : BigInt(digits);
}

/** Words for a whole number given as a decimal string (any digit system) or bigint. */
export function numberToPersianWords(value: string | bigint): string {
  const n = typeof value === "bigint" ? value : parseWholeNumber(value);
  if (n === undefined) throw new RangeError(`Not a whole number: ${value}`);
  if (n < 0n) return "منفی " + numberToPersianWords(-n);
  if (n >= 10n ** 18n) throw new RangeError("Amount is too large to spell out");
  if (n === 0n) return ONES[0];
  const groups: string[] = [];
  let scale = 0;
  let rest = n;
  while (rest > 0n) {
    const group = Number(rest % 1000n);
    if (group)
      groups.unshift(
        belowThousand(group) + (SCALES[scale] ? " " + SCALES[scale] : ""),
      );
    rest /= 1000n;
    scale += 1;
  }
  return groups.join(AND);
}

/**
 * «… ریال» or «… تومان». Toman is used only when the rial amount divides evenly by ten,
 * otherwise the exact rial amount is spelled out. Returns undefined for blank or invalid input.
 */
export function amountInWords(
  rial: string,
  unit: "rial" | "toman" = "rial",
): string | undefined {
  const n = parseWholeNumber(rial);
  if (n === undefined) return undefined;
  try {
    if (unit === "toman" && n % 10n === 0n)
      return `${numberToPersianWords(n / 10n)} تومان`;
    return `${numberToPersianWords(n)} ریال`;
  } catch {
    return undefined;
  }
}

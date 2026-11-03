// Advisory validators for Iranian identifiers. Mirrors crates/sarv-quote/src/iran
// and is tested against the same fixtures (crates/sarv-quote/tests/fixtures/iran_ids.json).
// Saved contact data stays free text: foreign parties have no Iranian identifiers.
import { normalizeDigits } from "./normalize-fa.ts";

export type IdReason = "length" | "digits" | "repeated" | "checksum";
export type IdResult = { ok: true } | { ok: false; reason: IdReason };
const OK: IdResult = { ok: true };
const fail = (reason: IdReason): IdResult => ({ ok: false, reason });

function digitsOf(input: string, length: number): number[] | IdReason {
  const s = normalizeDigits(input);
  if ([...s].length !== length) return "length";
  if (!/^[0-9]+$/.test(s)) return "digits";
  return [...s].map(Number);
}

export function nationalId(input: string): IdResult {
  const d = digitsOf(input, 10);
  if (typeof d === "string") return fail(d);
  if (d.every((x) => x === d[0])) return fail("repeated");
  const sum = d.slice(0, 9).reduce((acc, x, i) => acc + x * (10 - i), 0);
  const r = sum % 11;
  return (r < 2 ? d[9] === r : d[9] === 11 - r) ? OK : fail("checksum");
}

const LEGAL_COEF = [29, 27, 23, 19, 17, 29, 27, 23, 19, 17];
export function legalId(input: string): IdResult {
  const d = digitsOf(input, 11);
  if (typeof d === "string") return fail(d);
  if (d.every((x) => x === d[0])) return fail("repeated");
  const c = d[9] + 2;
  const sum = d
    .slice(0, 10)
    .reduce((acc, x, i) => acc + (x + c) * LEGAL_COEF[i], 0);
  const r = sum % 11 === 10 ? 0 : sum % 11;
  return d[10] === r ? OK : fail("checksum");
}

export function postalCode(input: string): IdResult {
  const d = digitsOf(input, 10);
  return typeof d === "string" ? fail(d) : OK;
}

export function sayadId(input: string): IdResult {
  const d = digitsOf(input, 16);
  return typeof d === "string" ? fail(d) : OK;
}

export function shebaCanonical(input: string): string {
  return normalizeDigits(input).toUpperCase();
}

export function sheba(input: string): IdResult {
  const s = shebaCanonical(input);
  if ([...s].length !== 26) return fail("length");
  if (!s.startsWith("IR") || !/^[0-9]{24}$/.test(s.slice(2)))
    return fail("digits");
  let r = 0;
  for (const ch of s.slice(4) + s.slice(0, 4)) {
    r = /[0-9]/.test(ch)
      ? (r * 10 + Number(ch)) % 97
      : (r * 100 + ch.charCodeAt(0) - 55) % 97;
  }
  return r === 1 ? OK : fail("checksum");
}

/** Which identifier a free-text value looks like, so the UI can hint without forcing a type. */
export function classifyPartyId(
  input: string,
): "national" | "legal" | "other" | "empty" {
  const s = normalizeDigits(input);
  if (!s.trim()) return "empty";
  if (nationalId(s).ok) return "national";
  if (legalId(s).ok) return "legal";
  return "other";
}

/** Non-blocking advice strings (English keys; the UI translates them). Empty = nothing to say. */
export function partyIdAdvice(input: string): string | undefined {
  return classifyPartyId(input) === "other"
    ? "This does not look like an Iranian national or legal ID. You can still save it."
    : undefined;
}
export function postalCodeAdvice(input: string): string | undefined {
  return normalizeDigits(input).trim() && !postalCode(input).ok
    ? "Iranian postal codes have exactly 10 digits. You can still save it."
    : undefined;
}

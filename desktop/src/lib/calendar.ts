import { startOfYear, endOfYear, getYear, parse } from "date-fns-jalali";
import type { Locale } from "./i18n";

// Date-only values stay Gregorian ISO in the accounting API. Local dates avoid
// UTC conversion shifting a selected day in Tehran or other time zones.
export function parseDate(value: string): Date {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day);
}

export function isoDate(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function fiscalYear(locale: Locale, date: Date) {
  return locale === "fa"
    ? {
        name: new Intl.NumberFormat("fa-IR", { useGrouping: false }).format(
          getYear(date),
        ),
        start: isoDate(startOfYear(date)),
        end: isoDate(endOfYear(date)),
      }
    : {
        name: String(date.getFullYear()),
        start: isoDate(new Date(date.getFullYear(), 0, 1)),
        end: isoDate(new Date(date.getFullYear(), 11, 31)),
      };
}

export function formatCalendarDate(
  value: string,
  locale: Locale,
  calendar = locale === "fa" ? "persian" : "gregory",
): string {
  return new Intl.DateTimeFormat(
    locale === "fa"
      ? `fa-IR-u-ca-${calendar}-nu-arabext`
      : `en-US-u-ca-${calendar}-nu-latn`,
    { year: "numeric", month: "long", day: "numeric" },
  ).format(parseDate(value));
}

export function canonicalDraftDate(value: string, display: "Iso" | "Jalali") {
  if (display !== "Jalali" || !value.trim()) return value;
  const latin = value
    .replace(/[۰-۹]/g, (digit) => String("۰۱۲۳۴۵۶۷۸۹".indexOf(digit)))
    .replace(/[٠-٩]/g, (digit) => String("٠١٢٣٤٥٦٧٨٩".indexOf(digit)))
    .replaceAll("/", "-");
  const date = parse(latin, "yyyy-MM-dd", new Date());
  return Number.isNaN(date.getTime()) ? value : isoDate(date);
}

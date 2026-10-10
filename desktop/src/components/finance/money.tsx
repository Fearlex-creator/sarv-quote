import { useContext } from "react";
import { BooksContext } from "../../app/books";
import { Badge } from "../ui/badge";
import { useTranslation } from "../../lib/i18n";
import type { Integer } from "../../lib/types";
export function formatMoney(
  value: Integer,
  unit: "Rial" | "Toman" = "Rial",
  locale = "en",
) {
  const n = BigInt(value);
  if (unit === "Rial")
    return new Intl.NumberFormat(locale === "fa" ? "fa-IR" : "en-US")
      .format(n)
      .replaceAll("-", "−");
  const negative = n < 0n;
  const a = negative ? -n : n;
  const whole = new Intl.NumberFormat(
    locale === "fa" ? "fa-IR" : "en-US",
  ).format(a / 10n);
  return `${negative ? "−" : ""}${whole}${locale === "fa" ? "٫" + String(a % 10n).replace(/\d/g, (d) => "۰۱۲۳۴۵۶۷۸۹"[Number(d)]) : "." + String(a % 10n)}`;
}
export function Money({
  value,
  unit = "Rial",
  locale,
  currency = false,
}: {
  value: Integer;
  unit?: "Rial" | "Toman";
  locale?: string;
  currency?: boolean;
}) {
  const books = useContext(BooksContext);
  return (
    <span className="money tabular-nums" dir="ltr">
      {formatMoney(value, unit, locale ?? books?.numberLocale ?? "en")}
      {currency && (
        <span className="currency"> {unit === "Rial" ? "IRR" : "toman"}</span>
      )}
    </span>
  );
}
export function Status({
  children,
  kind = "neutral",
}: {
  children: React.ReactNode;
  kind?: "neutral" | "positive" | "warning" | "danger";
}) {
  const t = useTranslation();
  return (
    <Badge variant={kind} className="status">
      <span className="status-dot" />
      {typeof children === "string" ? t(children) : children}
    </Badge>
  );
}

export function formatQuantity(value: Integer, locale?: string) {
  const n = BigInt(value);
  const a = n < 0n ? -n : n;
  const fraction = String(a % 1000n)
    .padStart(3, "0")
    .replace(/0+$/, "");
  const text = `${n < 0n ? "−" : ""}${a / 1000n}${fraction ? "." + fraction : ""}`;
  return locale === "fa"
    ? text.replace(/\d/g, (d) => "۰۱۲۳۴۵۶۷۸۹"[Number(d)]).replace(".", "٫")
    : text;
}

export function formatDate(
  value: string,
  locale = "en",
  display?: "Iso" | "DayMonthYear" | "Jalali",
) {
  const date = new Date(value.length === 10 ? `${value}T12:00:00Z` : value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(
    locale === "fa" ? "fa-IR-u-ca-gregory" : locale === "en" ? "en-US" : locale,
    {
      year: "numeric",
      month: display === "DayMonthYear" ? "2-digit" : "short",
      day: display === "DayMonthYear" ? "2-digit" : "numeric",
      timeZone: "UTC",
    },
  ).format(date);
}

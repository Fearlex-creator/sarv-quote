import { cn } from "../../lib/utils";
import { useContext } from "react";
import { BooksContext } from "../../app/books";
import { Badge } from "../ui/badge";
import { useTranslation } from "../../lib/i18n";
import type { Integer } from "../../lib/types";
const numberFormats = {
  en: new Intl.NumberFormat("en-US"),
  fa: new Intl.NumberFormat("fa-IR"),
};
const dateFormats = new Map<string, Intl.DateTimeFormat>();
export function formatMoney(
  value: Integer,
  unit: "Rial" | "Toman" = "Rial",
  locale = "en",
) {
  const n = BigInt(value);
  if (unit === "Rial")
    return numberFormats[locale === "fa" ? "fa" : "en"]
      .format(n)
      .replaceAll("-", "−");
  const negative = n < 0n;
  const a = negative ? -n : n;
  const whole = numberFormats[locale === "fa" ? "fa" : "en"].format(a / 10n);
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
    <Badge
      variant={kind === "danger" ? "destructive" : "secondary"}
      className={cn("status", {
        "status-positive": kind === "positive",
        "status-warning": kind === "warning",
      })}
    >
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
  const key = `${locale}:${display ?? "default"}`;
  let formatter = dateFormats.get(key);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(
      locale === "fa"
        ? display === "Jalali"
          ? "fa-IR-u-ca-persian"
          : "fa-IR-u-ca-gregory"
        : locale === "en"
          ? display === "Jalali"
            ? "en-US-u-ca-persian"
            : "en-US-u-ca-gregory"
          : locale,
      {
        year: "numeric",
        month: display === "DayMonthYear" ? "2-digit" : "short",
        day: display === "DayMonthYear" ? "2-digit" : "numeric",
        timeZone: "UTC",
      },
    );
    dateFormats.set(key, formatter);
  }
  return formatter.format(date);
}

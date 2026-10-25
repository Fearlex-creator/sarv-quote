import type { ProductProfile } from "./types";
export function priceRials(raw: string | null | undefined): bigint | null {
  const n = (raw ?? "")
    .replace(/[۰-۹]/g, (c) => String("۰۱۲۳۴۵۶۷۸۹".indexOf(c)))
    .replace(/[٠-٩]/g, (c) => String("٠١٢٣٤٥٦٧٨٩".indexOf(c)))
    .trim();
  if (!/^\d+(\.0+)?$/.test(n)) return null;
  return BigInt(n.split(".")[0]);
}
export function estimatedPricing(p: ProductProfile) {
  const buy = priceRials(p.purchase_price),
    sale = priceRials(p.sale_price);
  if (buy === null || sale === null) return null;
  const profit = sale - buy;
  return {
    profit: profit.toString(),
    margin: sale > 0n ? ((profit * 10000n) / sale).toString() : null,
    markup: buy > 0n ? ((profit * 10000n) / buy).toString() : null,
  };
}
export function formatPercent(basisPoints: string | null, locale = "en") {
  if (basisPoints === null) return "—";
  const n = BigInt(basisPoints),
    a = n < 0n ? -n : n;
  let text = `${n < 0n ? "−" : ""}${new Intl.NumberFormat(locale).format(a / 100n)}.${String(a % 100n).padStart(2, "0")}`;
  if (locale === "fa")
    text = text
      .replace(/\d/g, (c) => "۰۱۲۳۴۵۶۷۸۹"[Number(c)])
      .replace(".", "٫");
  return `${text}${locale === "fa" ? "٪" : "%"}`;
}

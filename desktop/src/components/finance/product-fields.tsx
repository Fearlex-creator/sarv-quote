import { useBooks } from "../../app/books";
import { Field, Note } from "../ui/shared";
import { FieldGroup } from "../ui/field";
import { Input } from "../ui/input";
import { Textarea } from "../ui/textarea";
import { NativeSelect, NativeSelectOption } from "../ui/native-select";
import { Money, formatQuantity } from "./money";
import { estimatedPricing, formatPercent } from "../../lib/product-pricing";
import type { ProductProfile } from "../../lib/types";
export function ProductFields({
  value,
  onChange,
}: {
  value: ProductProfile;
  onChange: (p: ProductProfile) => void;
}) {
  const b = useBooks(),
    t = b.t,
    estimate = estimatedPricing(value);
  return (
    <>
      <Note>
        {t(
          "Prices are in rials per base unit. Defaults fill new sales/purchases; actual stock cost comes from posted receipts.",
        )}
      </Note>
      <FieldGroup className="form-grid">
        <Field label={t("Default purchase price · IRR")}>
          <Input
            inputMode="decimal"
            value={value.purchase_price ?? ""}
            onChange={(e) =>
              onChange({ ...value, purchase_price: e.target.value || null })
            }
          />
        </Field>
        <Field label={t("Default selling price · IRR")}>
          <Input
            inputMode="decimal"
            value={value.sale_price ?? ""}
            onChange={(e) =>
              onChange({ ...value, sale_price: e.target.value || null })
            }
          />
        </Field>
        <Field label={t("Barcode")}>
          <Input
            maxLength={160}
            value={value.barcode}
            onChange={(e) => onChange({ ...value, barcode: e.target.value })}
          />
        </Field>
        <Field label={t("Category")}>
          <Input
            maxLength={160}
            value={value.category}
            onChange={(e) => onChange({ ...value, category: e.target.value })}
          />
        </Field>
        <Field label={t("Brand")}>
          <Input
            maxLength={160}
            value={value.brand}
            onChange={(e) => onChange({ ...value, brand: e.target.value })}
          />
        </Field>
        <Field label={t("Minimum stock · base units")}>
          <Input
            inputMode="decimal"
            value={
              value.minimum_stock ??
              formatQuantity(value.minimum_stock_thousandths)
            }
            onChange={(e) =>
              onChange({ ...value, minimum_stock: e.target.value })
            }
          />
        </Field>
        <Field label={t("Preferred supplier")}>
          <NativeSelect
            value={value.preferred_supplier_id}
            onChange={(e) =>
              onChange({ ...value, preferred_supplier_id: e.target.value })
            }
          >
            <NativeSelectOption value="">
              {t("No preference")}
            </NativeSelectOption>
            {b.workspace.resources?.parties
              .filter((p) => p.supplier)
              .map((p) => (
                <NativeSelectOption key={p.id} value={p.id}>
                  {p.name}
                </NativeSelectOption>
              ))}
          </NativeSelect>
        </Field>
        <Field label={t("Product notes")}>
          <Textarea
            maxLength={2000}
            value={value.notes}
            onChange={(e) => onChange({ ...value, notes: e.target.value })}
          />
        </Field>
      </FieldGroup>
      {estimate && (
        <div className="grouped-list">
          <div className="summary-line px-4 py-3">
            <span>{t("Estimated profit per base unit · IRR")}</span>
            <Money value={estimate.profit} />
          </div>
          <div className="summary-line px-4 py-3">
            <span>{t("Estimated gross margin")}</span>
            <strong>{formatPercent(estimate.margin, b.numberLocale)}</strong>
          </div>
          <div className="summary-line px-4 py-3">
            <span>{t("Estimated markup on cost")}</span>
            <strong>{formatPercent(estimate.markup, b.numberLocale)}</strong>
          </div>
        </div>
      )}
    </>
  );
}

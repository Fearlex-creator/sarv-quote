import { FieldGroup } from "../ui/field";
import { Field } from "../ui/shared";
import { Input } from "../ui/input";
import { Textarea } from "../ui/textarea";
import { useTranslation } from "../../lib/i18n";
import type { BusinessProfile } from "../../lib/types";

export function BusinessFields({
  value,
  onChange,
}: {
  value: BusinessProfile;
  onChange: (value: BusinessProfile) => void;
}) {
  const t = useTranslation();
  return (
    <FieldGroup className="form-grid">
      <Field label={t("Business phone")}>
        <Input
          type="tel"
          maxLength={160}
          value={value.phone}
          onChange={(e) => onChange({ ...value, phone: e.target.value })}
        />
      </Field>
      <Field label={t("Registration / national ID")}>
        <Input
          maxLength={160}
          value={value.registration_id}
          onChange={(e) =>
            onChange({ ...value, registration_id: e.target.value })
          }
        />
      </Field>
      <Field label={t("Tax / economic ID")}>
        <Input
          maxLength={160}
          value={value.tax_id}
          onChange={(e) => onChange({ ...value, tax_id: e.target.value })}
        />
      </Field>
      <Field label={t("Invoice reference prefix")}>
        <Input
          maxLength={160}
          value={value.invoice_prefix}
          onChange={(e) =>
            onChange({ ...value, invoice_prefix: e.target.value })
          }
          placeholder="INV"
        />
      </Field>
      <Field label={t("Business address")}>
        <Textarea
          maxLength={2000}
          value={value.address}
          onChange={(e) => onChange({ ...value, address: e.target.value })}
        />
      </Field>
      <Field
        label={t("Bank & payment details")}
        hint={t("Printed on every invoice so customers know where to pay.")}
      >
        <Textarea
          maxLength={2000}
          value={value.bank_details}
          onChange={(e) => onChange({ ...value, bank_details: e.target.value })}
        />
      </Field>
      <Field label={t("Invoice terms")}>
        <Textarea
          maxLength={2000}
          value={value.terms}
          onChange={(e) => onChange({ ...value, terms: e.target.value })}
        />
      </Field>
      <Field label={t("Invoice footer")}>
        <Textarea
          maxLength={2000}
          value={value.footer}
          onChange={(e) => onChange({ ...value, footer: e.target.value })}
        />
      </Field>
    </FieldGroup>
  );
}

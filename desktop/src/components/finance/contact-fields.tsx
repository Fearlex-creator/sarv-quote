import { FieldGroup } from "../ui/field";
import { Field } from "../ui/shared";
import { Input } from "../ui/input";
import { Textarea } from "../ui/textarea";
import { useTranslation } from "../../lib/i18n";
import type { PartyProfile } from "../../lib/types";

export function ContactFields({
  value,
  onChange,
}: {
  value: PartyProfile;
  onChange: (value: PartyProfile) => void;
}) {
  const t = useTranslation();
  const fields: { key: keyof PartyProfile; label: string; type?: string }[] = [
    { key: "first_name", label: "First name" },
    { key: "last_name", label: "Last name" },
    { key: "organization", label: "Company / organization" },
    { key: "phone", label: "Phone", type: "tel" },
    { key: "mobile", label: "Mobile", type: "tel" },
    { key: "email", label: "Email", type: "email" },
    { key: "city", label: "City" },
    { key: "postal_code", label: "Postal code" },
    { key: "national_id", label: "National / registration ID" },
    { key: "tax_id", label: "Tax / economic ID" },
    { key: "website", label: "Website" },
  ];
  return (
    <FieldGroup className="form-grid">
      {fields.map((f) => (
        <Field key={f.key} label={t(f.label)}>
          <Input
            type={f.type ?? "text"}
            maxLength={160}
            value={value[f.key]}
            onChange={(e) => onChange({ ...value, [f.key]: e.target.value })}
          />
        </Field>
      ))}
      <Field label={t("Address")}>
        <Textarea
          maxLength={2000}
          value={value.address}
          onChange={(e) => onChange({ ...value, address: e.target.value })}
        />
      </Field>
      <Field label={t("Contact notes")}>
        <Textarea
          maxLength={2000}
          value={value.notes}
          onChange={(e) => onChange({ ...value, notes: e.target.value })}
        />
      </Field>
    </FieldGroup>
  );
}

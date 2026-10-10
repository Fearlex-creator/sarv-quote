import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { Preview } from "../lib/types";
import { Modal } from "../components/ui/dialog";
import { Button } from "../components/ui/button";
import { Field, ErrorState, Note } from "../components/ui/shared";
import { Money } from "../components/finance/money";

export function TransferDialog({ onClose }: { onClose: () => void }) {
  const t = useTranslation();
  const b = useBooks();
  const resources = b.workspace.resources;
  const [key] = useState(uid);
  const [item, setItem] = useState(resources?.items[0]?.id ?? "");
  const [source, setSource] = useState(resources?.warehouses[0]?.id ?? "");
  const [target, setTarget] = useState(resources?.warehouses[1]?.id ?? "");
  const [quantity, setQuantity] = useState("1");
  const [date, setDate] = useState(today);
  const [reason, setReason] = useState("");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const payload = {
    company_id: b.companyId,
    period_id: b.periodId,
    source_id: key,
    request_key: key,
    item_id: item,
    source_warehouse_id: source,
    target_warehouse_id: target,
    unit_id: resources?.items.find((x) => x.id === item)?.base_unit_id,
    quantity,
    date,
    reason,
  };
  async function run(post = false) {
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command("transfer_post", {
          ...payload,
          expected_generation: preview?._generation,
        });
        await b.refresh();
        toast.success(t("Warehouse transfer recorded"));
        onClose();
      } else setPreview(await command("transfer_preview", payload));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title={t("Transfer stock")}
      confirmation={!!preview}
      description={t(
        "Transfer {value1} from {value2} to {value3}. Moves stock at its carried cost.",
        {
          value1:
            resources?.items.find((x) => x.id === item)?.name ?? t("goods"),
          value2:
            resources?.warehouses.find((x) => x.id === source)?.name ??
            t("source warehouse"),
          value3:
            resources?.warehouses.find((x) => x.id === target)?.name ??
            t("target warehouse"),
        },
      )}
    >
      <form
        onChange={() => setPreview(null)}
        onSubmit={(e) => {
          e.preventDefault();
          void run(!!preview);
        }}
      >
        {error && <ErrorState error={error} />}
        <div className="form-grid">
          <Field label={t("Product")}>
            <NativeSelect
              required
              value={item}
              onChange={(e) => setItem(e.target.value)}
            >
              <NativeSelectOption value="">
                {t("Select product")}
              </NativeSelectOption>
              {resources?.items.map((x) => (
                <NativeSelectOption key={x.id} value={x.id}>
                  {x.name}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </Field>
          <Field label={t("Quantity in base units")}>
            <Input
              required
              inputMode="decimal"
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
            />
          </Field>
          <Field label={t("Source warehouse")}>
            <NativeSelect
              required
              value={source}
              onChange={(e) => setSource(e.target.value)}
            >
              <NativeSelectOption value="">
                {t("Select warehouse")}
              </NativeSelectOption>
              {resources?.warehouses.map((x) => (
                <NativeSelectOption key={x.id} value={x.id}>
                  {x.name}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </Field>
          <Field label={t("Target warehouse")}>
            <NativeSelect
              required
              value={target}
              onChange={(e) => setTarget(e.target.value)}
            >
              <NativeSelectOption value="">
                {t("Select warehouse")}
              </NativeSelectOption>
              {resources?.warehouses.map((x) => (
                <NativeSelectOption key={x.id} value={x.id}>
                  {x.name}
                </NativeSelectOption>
              ))}
            </NativeSelect>
          </Field>
          <Field label={t("Financial date")}>
            <Input
              required
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
            />
          </Field>
          <Field label={t("Reason")}>
            <Input
              required
              value={reason}
              onChange={(e) => setReason(e.target.value)}
            />
          </Field>
        </div>
        {preview && (
          <Note>
            {t("Value:")} <Money value={preview.total} currency />
          </Note>
        )}
        <div className="form-actions">
          <Button
            type="button"
            variant="outline"
            onClick={onClose}
            disabled={busy}
          >
            {t("Cancel")}
          </Button>
          <Button disabled={busy || !item || !source || !target || !reason}>
            {busy
              ? t("Working…")
              : preview
                ? t("Confirm and transfer")
                : t("Review transfer")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

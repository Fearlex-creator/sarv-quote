import { useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, downloadWorkbook, today } from "../lib/client";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { ErrorState, PageHeader } from "../components/ui/shared";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";

type Kind = "products" | "customers" | "suppliers" | "opening_stock";
interface Preview {
  committed: boolean;
  accepted: string;
  errors: { row: string; message: string }[];
  warnings: string[];
  opening_stock_effect: string;
  currency: string;
}
interface GraphStatus {
  configured: boolean;
  status: string;
  instructions: string;
}

async function fileBase64(file: File) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = "";
  const size = 0x8000;
  for (let i = 0; i < bytes.length; i += size) {
    binary += String.fromCharCode(...bytes.subarray(i, i + size));
  }
  return btoa(binary);
}

export function ExcelIO() {
  const b = useBooks();
  const t = b.t;
  const [kind, setKind] = useState<Kind>("products");
  const [policy, setPolicy] = useState<"skip" | "update">("skip");
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [graph, setGraph] = useState<GraphStatus | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function loadGraph() {
    setGraph(await command<GraphStatus>("excel_graph_status"));
  }

  async function runPreview() {
    if (!file) return;
    setBusy(true);
    setError("");
    setPreview(null);
    setConfirm(false);
    try {
      const content_base64 = await fileBase64(file);
      setPreview(
        await command<Preview>("excel_preview", { kind, content_base64 }),
      );
      if (!graph) await loadGraph();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function commit() {
    if (!file || !preview || preview.errors.length) return;
    setBusy(true);
    setError("");
    try {
      const content_base64 = await fileBase64(file);
      const result = await command<{ created: string; updated: string; skipped: string; replayed: boolean }>(
        "excel_commit",
        {
          kind,
          content_base64,
          company_id: b.companyId,
          period_id: b.periodId,
          date: today(),
          request_key: crypto.randomUUID(),
          policy,
          confirm: true,
        },
      );
      toast.success(
        result.replayed
          ? t("This workbook was already committed")
          : t("Imported {created} new, {updated} updated, {skipped} skipped", {
              created: result.created,
              updated: result.updated,
              skipped: result.skipped,
            }),
      );
      setPreview(null);
      setFile(null);
      setConfirm(false);
      await b.refresh();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t("Excel")}
        actions={
          <Button
            variant="outline"
            onClick={() =>
              void downloadWorkbook("excel_template", { kind }).catch((e: Error) =>
                setError(e.message),
              )
            }
          >
            {t("Download template")}
          </Button>
        }
      />
      <p className="text-sm text-muted-foreground">
        {t("Rial amounts, not tomans. Formulas and macros are refused.")}
      </p>
      <div className="grid gap-4 rounded-xl border border-border bg-card p-4 md:grid-cols-2">
        <label className="flex flex-col gap-1 text-sm">
          {t("Sheet")}
          <NativeSelect value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
            <NativeSelectOption value="products">{t("Products")}</NativeSelectOption>
            <NativeSelectOption value="customers">{t("Customers")}</NativeSelectOption>
            <NativeSelectOption value="suppliers">{t("Suppliers")}</NativeSelectOption>
            <NativeSelectOption value="opening_stock">{t("Opening stock")}</NativeSelectOption>
          </NativeSelect>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          {t("If the row already exists")}
          <NativeSelect
            value={policy}
            onChange={(e) => setPolicy(e.target.value as "skip" | "update")}
          >
            <NativeSelectOption value="skip">{t("Skip existing rows")}</NativeSelectOption>
            <NativeSelectOption value="update">{t("Update prices and contact details")}</NativeSelectOption>
          </NativeSelect>
        </label>
        <label className="flex flex-col gap-1 text-sm md:col-span-2">
          {t("Workbook")}
          <Input
            type="file"
            accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            onChange={(e) => {
              setFile(e.target.files?.[0] ?? null);
              setPreview(null);
            }}
          />
        </label>
        <div className="flex flex-wrap gap-2 md:col-span-2">
          <Button type="button" disabled={!file || busy} onClick={() => void runPreview()}>
            {t("Preview workbook")}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() =>
              void downloadWorkbook("excel_export", {
                kind: kind === "opening_stock" ? "products" : kind,
                company_id: b.companyId,
                to: today(),
              }).catch((e: Error) => setError(e.message))
            }
          >
            {t("Export current rows")}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() =>
              void downloadWorkbook("excel_export", {
                kind: "trial_balance",
                company_id: b.companyId,
                to: today(),
              }).catch((e: Error) => setError(e.message))
            }
          >
            {t("Export trial balance")}
          </Button>
        </div>
      </div>
      {error && <ErrorState error={error} />}
      {preview && (
        <section className="flex flex-col gap-3 rounded-xl border border-border p-4" aria-live="polite">
          <p className="text-sm">
            {t("{count} rows can be committed. Currency: {currency}. The whole workbook is saved or nothing is.", {
              count: preview.accepted,
              currency: preview.currency,
            })}
          </p>
          {preview.opening_stock_effect && (
            <p className="text-sm text-muted-foreground">{preview.opening_stock_effect}</p>
          )}
          {preview.errors.length > 0 && (
            <ul className="flex flex-col gap-1 text-sm text-destructive">
              {preview.errors.map((row) => (
                <li key={`${row.row}-${row.message}`}>
                  {t("Row {row}: {message}", row)}
                </li>
              ))}
            </ul>
          )}
          {preview.errors.length === 0 && (
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={confirm}
                onChange={(e) => setConfirm(e.target.checked)}
              />
              {t("Commit these rows into the books")}
            </label>
          )}
          <Button type="button" disabled={!confirm || busy || preview.errors.length > 0} onClick={() => void commit()}>
            {t("Commit workbook")}
          </Button>
        </section>
      )}
      <section className="rounded-xl border border-dashed border-border p-4">
        <h2 className="text-sm font-medium">{t("Microsoft 365 connection")}</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          {graph?.instructions ??
            t("Local Excel files work without a Microsoft 365 subscription. Live sync is not configured.")}
        </p>
        <p className="mt-2 text-sm font-medium" dir="ltr">
          {t("Status")}: {graph?.status ?? "not_configured"}
        </p>
        {!graph && (
          <Button className="mt-3" variant="outline" type="button" onClick={() => void loadGraph().catch((e: Error) => setError(e.message))}>
            {t("Check live connection")}
          </Button>
        )}
      </section>
    </div>
  );
}

import { FieldGroup } from "../components/ui/field";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "../components/ui/table";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { Textarea } from "../components/ui/textarea";
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { TrashIcon as Trash2 } from "@phosphor-icons/react/dist/csr/Trash";
import { FileTextIcon as FileText } from "@phosphor-icons/react/dist/csr/FileText";
import { ArrowLeftIcon as ArrowLeft } from "@phosphor-icons/react/dist/csr/ArrowLeft";
import { CheckCircleIcon as CheckCircle2 } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { toast } from "sonner";
import { useUnsavedChanges } from "../lib/use-unsaved";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { Draft, InputLine, Preview, RecordEntry } from "../lib/types";
import { PageHeader, Field, ErrorState } from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { DataGrid } from "../components/ui/data-grid";
import { Modal } from "../components/layout/modal";
import { Money, Status, formatDate } from "../components/finance/money";
import { Customers } from "./resources";
export function recordKind(r: RecordEntry) {
  return Object.keys(r.data)[0];
}
export function recordData(r: RecordEntry) {
  return Object.values(r.data)[0];
}
export function recordCommand(r: RecordEntry) {
  return Object.values(r.command)[0];
}
export function Sales({
  onEdit,
  onDetail,
}: {
  onEdit: (draft?: Draft) => void;
  onDetail: (record: RecordEntry) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Orders");
  const [offset, setOffset] = useState(0);
  const [records, setRecords] = useState<RecordEntry[] | null>(null);
  const [recordsSyncedAt, setRecordsSyncedAt] = useState<Date | null>(null);
  const [error, setError] = useState("");
  const rows = (records ?? b.workspace.sales).filter((x) =>
    tab === "Invoices"
      ? recordKind(x) === "Fulfill"
      : recordKind(x) === (tab === "Returns" ? "Return" : "Order"),
  );
  async function page(next: number) {
    setError("");
    try {
      const w = await command<{ sales: RecordEntry[] }>("workspace", {
        company_id: b.companyId,
        offset: String(next),
      });
      setRecords(w.sales);
      setRecordsSyncedAt(new Date());
      setOffset(next);
    } catch (e) {
      setError((e as Error).message);
    }
  }
  return (
    <Tabs
      value={tab}
      onValueChange={setTab}
      dir={b.locale === "fa" ? "rtl" : "ltr"}
    >
      {tab !== "Customers" && (
        <PageHeader
          title={b.t("Sales")}
          syncedAt={tab !== "Drafts" && records ? recordsSyncedAt : undefined}
          actions={
            <Button
              disabled={!b.workspace.initialized}
              onClick={() => onEdit()}
            >
              <Plus />
              {b.t("New order")}
            </Button>
          }
        />
      )}
      <TabsList>
        {["Orders", "Invoices", "Returns", "Drafts", "Customers"].map((x) => (
          <TabsTrigger key={x} value={x}>
            {b.t(x)}
            {x === "Drafts" && b.workspace.drafts.length > 0
              ? ` (${b.workspace.drafts.length})`
              : ""}
          </TabsTrigger>
        ))}
      </TabsList>
      <TabsContent value={tab}>
        {error && <ErrorState error={error} />}{" "}
        {tab === "Customers" ? (
          <Customers />
        ) : tab === "Drafts" ? (
          <section className="panel">
            <DataGrid
              data={b.workspace.drafts}
              id={(x) => x.id}
              searchText={(x) => x.reference}
              empty={t("No drafts")}
              columns={[
                {
                  header: t("Reference"),
                  accessorKey: "reference",
                  cell: ({ row }) => (
                    <Button
                      variant="link"
                      className="text-link h-auto justify-start p-0"
                      onClick={() => onEdit(row.original)}
                    >
                      {row.original.reference || t("Untitled order")}
                    </Button>
                  ),
                },
                {
                  header: t("Customer"),
                  cell: ({ row }) =>
                    b.workspace.resources?.parties.find(
                      (x) => x.id === row.original.customer_id,
                    )?.name ?? t("Not selected"),
                },
                {
                  header: t("Date"),
                  accessorKey: "document_date",
                  cell: ({ row }) =>
                    formatDate(
                      row.original.document_date,
                      b.dateLocale,
                      b.workspace.settings.date_display,
                    ),
                },
                {
                  header: t("State"),
                  cell: () => <Status>{t("Draft")}</Status>,
                },
                {
                  header: "",
                  cell: ({ row }) => (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => onEdit(row.original)}
                    >
                      {t("Continue editing")}
                    </Button>
                  ),
                },
              ]}
            />
          </section>
        ) : (
          <section className="panel">
            <DataGrid
              data={rows}
              id={(x) => x.id}
              searchText={(x) =>
                (recordCommand(x).reference ??
                  recordCommand(x).invoice_reference ??
                  "") + (recordData(x).customer?.name ?? "")
              }
              empty={tab === "Invoices" ? t("No invoices") : t("No orders")}
              columns={[
                {
                  header: b.t("Reference"),
                  cell: ({ row }) => (
                    <Button
                      variant="link"
                      className="text-link h-auto justify-start p-0"
                      onClick={() => onDetail(row.original)}
                    >
                      {recordCommand(row.original).reference ??
                        recordCommand(row.original).invoice_reference ??
                        recordCommand(row.original).return_reference}
                    </Button>
                  ),
                },
                {
                  header: b.t("Customer"),
                  cell: ({ row }) =>
                    recordData(row.original).customer?.name ??
                    b.workspace.resources?.parties.find(
                      (x) => x.id === recordData(row.original).customer_id,
                    )?.name,
                },
                {
                  header: b.t("Date"),
                  cell: ({ row }) =>
                    formatDate(
                      recordCommand(row.original).context.date,
                      b.dateLocale,
                      b.workspace.settings.date_display,
                    ),
                },
                {
                  header: `${b.t("Amount")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                  meta: { numeric: true },
                  cell: ({ row }) => (
                    <Money
                      value={recordData(row.original).total ?? "0"}
                      unit={b.unit}
                      locale={b.numberLocale}
                    />
                  ),
                },
                {
                  header: b.t("Status"),
                  cell: ({ row }) => (
                    <Status
                      kind={
                        recordKind(row.original) === "Fulfill"
                          ? "positive"
                          : "neutral"
                      }
                    >
                      {recordKind(row.original) === "Fulfill"
                        ? t("Posted · Delivered")
                        : recordKind(row.original) === "Return"
                          ? t("Posted · Restocked")
                          : t("Recorded order")}
                    </Status>
                  ),
                },
                {
                  header: "",
                  cell: ({ row }) => (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => onDetail(row.original)}
                    >
                      {t("Open")}
                    </Button>
                  ),
                },
              ]}
            />
            <div className="pagination">
              <span>
                {t("Source records")}
                {offset + 1}–
                {Math.min(offset + 100, Number(b.workspace.sales_count))}{" "}
                {t("of")} {b.workspace.sales_count}
              </span>
              <div className="toolbar-actions">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={offset === 0}
                  onClick={() => page(Math.max(0, offset - 100))}
                >
                  {t("Previous")}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={offset + 100 >= Number(b.workspace.sales_count)}
                  onClick={() => page(offset + 100)}
                >
                  {t("Next")}
                </Button>
              </div>
            </div>
          </section>
        )}
      </TabsContent>
    </Tabs>
  );
}
function newLine(): InputLine {
  return {
    id: uid(),
    item_id: "",
    warehouse_id: "",
    unit_id: "",
    quantity: "1",
    unit_price: "",
    line_discount: "0",
    price_reference: "Manual price",
    price_date: today(),
    override_reason: "",
  };
}
export function OrderEditor({
  initial,
  onClose,
}: {
  initial?: Draft;
  onClose: () => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [draft, setDraft] = useState<Draft>(
    () =>
      initial ?? {
        id: uid(),
        revision: "0",
        company_id: b.companyId,
        period_id: b.periodId,
        setup_id: b.workspace.setups[0]?.id ?? "",
        customer_id: "",
        reference: "",
        document_date: today(),
        financial_date: today(),
        date_display: "Iso",
        display_unit: "Rial",
        document_discount: "0",
        supplied_total: "0",
        reason: "Goods sales order",
        quotation: null,
        lines: [newLine()],
      },
  );
  const [saved, setSaved] = useState(initial ? JSON.stringify(initial) : "");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [review, setReview] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [key] = useState(uid);
  const [discard, setDiscard] = useState(false);
  const dirty = JSON.stringify(draft) !== saved;
  const r = b.workspace.resources;
  useUnsavedChanges(dirty);
  function update<K extends keyof Draft>(k: K, v: Draft[K]) {
    setDraft((d) => ({ ...d, [k]: v }));
    setPreview(null);
    setReview(false);
  }
  function line(id: string, k: keyof InputLine, v: string) {
    setDraft((d) => ({
      ...d,
      lines: d.lines.map((l) =>
        l.id === id
          ? {
              ...l,
              [k]: v,
              ...(k === "item_id"
                ? {
                    unit_id:
                      r?.items.find((x) => x.id === v)?.base_unit_id ?? "",
                  }
                : {}),
            }
          : l,
      ),
    }));
    setPreview(null);
    setReview(false);
  }
  async function save(d: Draft = draft) {
    const rev = await command<string>("draft_save", {
      draft: d,
      expected_revision: d.revision === "0" ? null : d.revision,
    });
    const next = { ...d, revision: rev };
    setDraft(next);
    setSaved(JSON.stringify(next));
    await b.refresh();
    return next;
  }
  async function run(fn: () => Promise<void>) {
    setBusy(true);
    setError("");
    try {
      await fn();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function inspect() {
    const result = await command<{ preview: Preview; supplied_total: string }>(
      "draft_preview",
      { draft },
    );
    const next = { ...draft, supplied_total: result.supplied_total };
    setDraft(next);
    const savedDraft = await save(next);
    const reviewed = await command<{ preview: Preview; _generation: string }>(
      "draft_preview",
      { draft: savedDraft },
    );
    setPreview({ ...reviewed.preview, _generation: reviewed._generation });
    setReview(true);
  }
  return (
    <>
      <PageHeader
        title={t("Sales order")}
        actions={
          <>
            <Button
              variant="outline"
              disabled={busy}
              onClick={() => (dirty ? setDiscard(true) : onClose())}
            >
              <ArrowLeft className="directional-icon" />
              {t("Back to sales")}
            </Button>
            <Button
              variant="outline"
              disabled={busy}
              onClick={() =>
                run(async () => {
                  await save();
                  toast.success(t("Draft saved"));
                })
              }
            >
              {b.t("Save draft")}
            </Button>
            <Button
              disabled={busy || !draft.lines.length}
              onClick={() => run(inspect)}
            >
              {busy ? t("Working…") : b.t("Review order")}
            </Button>
          </>
        }
      />
      {error && <ErrorState error={error} />}
      <div className="editor">
        <section className="panel editor-document">
          <div className="editor-doc-header">
            <div>
              <h2>
                {b.workspace.companies.find((x) => x.id === b.companyId)?.name}
              </h2>
              <p style={{ fontSize: 14, marginTop: 4 }}>
                {t("Goods sales order · IRR")}
              </p>
            </div>
            <FileText
              style={{
                width: 24,
                height: 24,
                color: "var(--muted-foreground)",
              }}
            />
          </div>
          <FieldGroup className="editor-meta">
            <Field label={b.t("Customer")}>
              <NativeSelect
                required
                value={draft.customer_id}
                onChange={(e) => update("customer_id", e.target.value)}
              >
                <NativeSelectOption value="">
                  {t("Select customer")}
                </NativeSelectOption>
                {r?.parties
                  .filter((x) => x.customer)
                  .map((x) => (
                    <NativeSelectOption key={x.id} value={x.id}>
                      {x.name}
                    </NativeSelectOption>
                  ))}
              </NativeSelect>
            </Field>
            <Field label={t("Order reference")}>
              <Input
                value={draft.reference}
                onChange={(e) => update("reference", e.target.value)}
                placeholder={t("Your reference")}
              />
            </Field>
            <Field label={t("Document date")}>
              <Input
                type="date"
                value={draft.document_date}
                onChange={(e) => update("document_date", e.target.value)}
              />
            </Field>
            <Field label={t("Financial date")}>
              <Input
                type="date"
                value={draft.financial_date}
                onChange={(e) => update("financial_date", e.target.value)}
              />
            </Field>
          </FieldGroup>
          <div className="table-scroll">
            <Table className="editor-lines">
              <TableHeader>
                <TableRow>
                  <TableHead>{t("Product")}</TableHead>
                  <TableHead>{t("Warehouse")}</TableHead>
                  <TableHead>{t("Quantity")}</TableHead>
                  <TableHead className="numeric">{t("Price · IRR")}</TableHead>
                  <TableHead className="numeric">
                    {t("Discount · IRR")}
                  </TableHead>
                  <TableHead>
                    <span className="screen-reader-only">{t("Remove")}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {draft.lines.map((l, i) => (
                  <TableRow key={l.id}>
                    <TableCell>
                      <NativeSelect
                        aria-label={t("Product {index}", { index: i + 1 })}
                        value={l.item_id}
                        onChange={(e) => line(l.id, "item_id", e.target.value)}
                      >
                        <NativeSelectOption value="">
                          {t("Select product")}
                        </NativeSelectOption>
                        {r?.items.map((x) => (
                          <NativeSelectOption key={x.id} value={x.id}>
                            {x.sku} · {x.name}
                          </NativeSelectOption>
                        ))}
                      </NativeSelect>
                    </TableCell>
                    <TableCell>
                      <NativeSelect
                        aria-label={t("Warehouse {index}", { index: i + 1 })}
                        value={l.warehouse_id}
                        onChange={(e) =>
                          line(l.id, "warehouse_id", e.target.value)
                        }
                      >
                        <NativeSelectOption value="">
                          {t("Select warehouse")}
                        </NativeSelectOption>
                        {r?.warehouses.map((x) => (
                          <NativeSelectOption key={x.id} value={x.id}>
                            {x.name}
                          </NativeSelectOption>
                        ))}
                      </NativeSelect>
                    </TableCell>
                    <TableCell>
                      <Input
                        aria-label={t("Quantity {index}", { index: i + 1 })}
                        inputMode="decimal"
                        value={l.quantity}
                        onChange={(e) => line(l.id, "quantity", e.target.value)}
                      />
                    </TableCell>
                    <TableCell>
                      <Input
                        aria-label={t("Price {index}", { index: i + 1 })}
                        inputMode="numeric"
                        value={l.unit_price}
                        onChange={(e) =>
                          line(l.id, "unit_price", e.target.value)
                        }
                      />
                    </TableCell>
                    <TableCell>
                      <Input
                        aria-label={t("Discount {index}", { index: i + 1 })}
                        inputMode="numeric"
                        value={l.line_discount}
                        onChange={(e) =>
                          line(l.id, "line_discount", e.target.value)
                        }
                      />
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={t("Remove line {index}", { index: i + 1 })}
                        onClick={() =>
                          update(
                            "lines",
                            draft.lines.filter((x) => x.id !== l.id),
                          )
                        }
                      >
                        <Trash2 />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <Button
            variant="ghost"
            size="sm"
            style={{ marginTop: 12 }}
            onClick={() => update("lines", [...draft.lines, newLine()])}
          >
            <Plus />
            {t("Add line")}
          </Button>
          <div className="editor-summary">
            <Field label={t("Order discount · IRR")}>
              <Input
                inputMode="numeric"
                value={draft.document_discount}
                onChange={(e) => update("document_discount", e.target.value)}
              />
            </Field>
            <div className="summary-line summary-total">
              <span>{t("Total")}</span>
              {preview ? (
                <Money value={preview.total} currency />
              ) : (
                <span className="muted">{t("Review to calculate")}</span>
              )}
            </div>
          </div>
          <Field label={t("Reason / internal note")}>
            <Textarea
              style={{ marginTop: 16 }}
              value={draft.reason}
              onChange={(e) => update("reason", e.target.value)}
            />
          </Field>
        </section>
        <aside className="panel inspector">
          <h3>{t("Order state")}</h3>
          <Status>
            {t("Editable draft")}
            {dirty ? t(" · unsaved changes") : ""}
          </Status>
        </aside>
      </div>
      <Modal
        open={review}
        onOpenChange={(v) => {
          if (!busy) setReview(v);
        }}
        title={t("Review sales order")}
        confirmation
        description={t(
          "Record order {value1}. This records the order without invoicing or moving stock.",
          { value1: draft.reference || t("Untitled order") },
        )}
      >
        {preview && (
          <>
            <div className="detail-summary">
              <div>
                <small>{t("Reference")}</small>
                <strong>{draft.reference}</strong>
              </div>
              <div>
                <small>{t("Lines")}</small>
                <strong>{draft.lines.length}</strong>
              </div>
              <div>
                <small>{t("Total IRR")}</small>
                <Money value={preview.total} />
              </div>
            </div>
            <div className="form-actions">
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => setReview(false)}
              >
                {t("Keep editing")}
              </Button>
              <Button
                disabled={busy}
                onClick={() =>
                  run(async () => {
                    await command("draft_post", {
                      id: draft.id,
                      revision: draft.revision,
                      request_key: key,
                      expected_generation: preview?._generation,
                    });
                    await b.refresh();
                    toast.success(t("Sales order recorded"));
                    onClose();
                  })
                }
              >
                <CheckCircle2 />
                {t("Record order")}
              </Button>
            </div>
          </>
        )}
      </Modal>
      <Modal
        open={discard}
        onOpenChange={setDiscard}
        title={t("Leave unsaved changes?")}
        confirmation
        description={t(
          "Discard unsaved edits to {value1}. The saved draft remains available.",
          { value1: draft.reference || t("Untitled order") },
        )}
      >
        <div className="form-actions">
          <Button variant="outline" onClick={() => setDiscard(false)}>
            {t("Keep editing")}
          </Button>
          <Button variant="destructive" onClick={onClose}>
            {t("Discard changes")}
          </Button>
        </div>
      </Modal>
    </>
  );
}

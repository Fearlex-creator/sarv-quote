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
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { TrashIcon as Trash2 } from "@phosphor-icons/react/dist/csr/Trash";
import { toast } from "sonner";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { Preview, RecordEntry } from "../lib/types";
import { recordCommand, recordData, recordKind } from "./sales";
import { PageHeader, Field, ErrorState, Note } from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { Modal } from "../components/layout/modal";
import { DataGrid } from "../components/ui/data-grid";
import { Money, Status, formatDate } from "../components/finance/money";
import { Customers } from "./resources";
export function Purchases({
  onDetail,
}: {
  onDetail: (r: RecordEntry) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Orders");
  const [create, setCreate] = useState(false);
  const [error, setError] = useState("");
  const [offset, setOffset] = useState(0);
  const [records, setRecords] = useState<RecordEntry[] | null>(null);
  const [recordsSyncedAt, setRecordsSyncedAt] = useState<Date | null>(null);
  const kind =
    tab === "Orders"
      ? "Order"
      : tab === "Receipts"
        ? "Receive"
        : tab === "Returns"
          ? "Return"
          : "Bill";
  const rows = (records ?? b.workspace.purchases).filter(
    (x) => recordKind(x) === kind,
  );
  async function page(next: number) {
    try {
      const w = await command<{ purchases: RecordEntry[] }>("workspace", {
        company_id: b.companyId,
        offset: String(next),
      });
      setRecords(w.purchases);
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
      {tab !== "Vendors" && (
        <PageHeader
          title={b.t("Purchases")}
          syncedAt={records ? recordsSyncedAt : undefined}
          actions={
            <Button
              disabled={!b.workspace.initialized}
              onClick={() => setCreate(true)}
            >
              <Plus />
              {t("New purchase order")}
            </Button>
          }
        />
      )}
      <TabsList>
        {["Orders", "Receipts", "Bills", "Returns", "Vendors"].map((x) => (
          <TabsTrigger key={x} value={x}>
            {b.t(x)}
          </TabsTrigger>
        ))}
      </TabsList>
      <TabsContent value={tab}>
        {error && <ErrorState error={error} />}{" "}
        {tab === "Vendors" ? (
          <Customers vendor />
        ) : (
          <section className="panel">
            <DataGrid
              data={rows}
              id={(x) => x.id}
              searchText={(x) =>
                (recordCommand(x).reference ??
                  recordCommand(x).return_reference ??
                  "") + (recordData(x).supplier?.name ?? "")
              }
              empty={`No ${tab.toLowerCase()} yet.`}
              columns={[
                {
                  header: t("Reference"),
                  cell: ({ row }) => (
                    <Button
                      variant="link"
                      className="text-link h-auto justify-start p-0"
                      onClick={() => onDetail(row.original)}
                    >
                      {recordCommand(row.original).reference ??
                        recordCommand(row.original).return_reference}
                    </Button>
                  ),
                },
                {
                  header: t("Supplier"),
                  cell: ({ row }) =>
                    recordData(row.original).supplier?.name ??
                    b.workspace.resources?.parties.find(
                      (x) => x.id === recordData(row.original).supplier_id,
                    )?.name ??
                    "—",
                },
                {
                  header: t("Date"),
                  cell: ({ row }) =>
                    formatDate(
                      recordCommand(row.original).context.date,
                      b.dateLocale,
                      b.workspace.settings.date_display,
                    ),
                },
                {
                  header: `${t("Amount")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
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
                  header: t("State"),
                  cell: ({ row }) => (
                    <Status
                      kind={row.original.journal_id ? "positive" : "neutral"}
                    >
                      {row.original.journal_id
                        ? t("Posted")
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
                {b.workspace.purchase_count} {t("source records")}{" "}
              </span>
              <div className="toolbar-actions">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={!offset}
                  onClick={() => page(Math.max(0, offset - 100))}
                >
                  {t("Previous")}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={offset + 100 >= Number(b.workspace.purchase_count)}
                  onClick={() => page(offset + 100)}
                >
                  {t("Next")}
                </Button>
              </div>
            </div>
          </section>
        )}
        {create && <PurchaseEditor onClose={() => setCreate(false)} />}
      </TabsContent>
    </Tabs>
  );
}
function newLine() {
  return {
    id: uid(),
    item_id: "",
    warehouse_id: "",
    unit_id: "",
    quantity: "1",
    unit_price: "",
    line_discount: "0",
  };
}
function PurchaseEditor({ onClose }: { onClose: () => void }) {
  const t = useTranslation();
  const b = useBooks();
  const r = b.workspace.resources;
  const [key] = useState(uid);
  const [supplier, setSupplier] = useState("");
  const [reference, setReference] = useState("");
  const [date, setDate] = useState(today);
  const [lines, setLines] = useState([newLine()]);
  const [discount, setDiscount] = useState("0");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [discard, setDiscard] = useState(false);
  const payload = {
    company_id: b.companyId,
    period_id: b.periodId,
    source_id: key,
    request_key: key,
    date,
    reference,
    supplier_id: supplier,
    reason: "Purchase order",
    discount,
    lines,
  };
  function updateLine(id: string, k: string, v: string) {
    setPreview(null);
    setLines((ls) =>
      ls.map((l) =>
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
    );
  }
  async function run(post = false) {
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command("purchase_post", {
          ...payload,
          expected_generation: preview?._generation,
        });
        await b.refresh();
        toast.success(t("Purchase order recorded"));
        onClose();
      } else setPreview(await command("purchase_preview", payload));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <Modal
        open
        onOpenChange={(v) => {
          if (!v && !busy) setDiscard(true);
        }}
        title={t("New purchase order")}
        confirmation={!!preview}
        description={t(
          "Record purchase order {value1}. Stock changes when goods are received.",
          { value1: reference || t("Untitled order") },
        )}
        wide
      >
        {error && <ErrorState error={error} />}
        <FieldGroup className="form-grid">
          <Field label={t("Supplier")}>
            <NativeSelect
              value={supplier}
              onChange={(e) => {
                setSupplier(e.target.value);
                setPreview(null);
              }}
            >
              <NativeSelectOption value="">
                {t("Select supplier")}
              </NativeSelectOption>
              {r?.parties
                .filter((x) => x.supplier)
                .map((x) => (
                  <NativeSelectOption key={x.id} value={x.id}>
                    {x.name}
                  </NativeSelectOption>
                ))}
            </NativeSelect>
          </Field>
          <Field label={t("Reference")}>
            <Input
              value={reference}
              onChange={(e) => {
                setReference(e.target.value);
                setPreview(null);
              }}
            />
          </Field>
          <Field label={t("Financial / document date")}>
            <Input
              type="date"
              value={date}
              onChange={(e) => {
                setDate(e.target.value);
                setPreview(null);
              }}
            />
          </Field>
          <Field label={t("Order discount · IRR")}>
            <Input
              inputMode="numeric"
              value={discount}
              onChange={(e) => {
                setDiscount(e.target.value);
                setPreview(null);
              }}
            />
          </Field>
        </FieldGroup>
        <div className="table-scroll section-gap">
          <Table className="editor-lines">
            <TableHeader>
              <TableRow>
                <TableHead>{t("Product")}</TableHead>
                <TableHead>{t("Warehouse")}</TableHead>
                <TableHead>{t("Quantity")}</TableHead>
                <TableHead className="numeric">{t("Price · IRR")}</TableHead>
                <TableHead className="numeric">{t("Discount · IRR")}</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {lines.map((l, i) => (
                <TableRow key={l.id}>
                  <TableCell>
                    <NativeSelect
                      aria-label={t("Purchase product {index}", {
                        index: i + 1,
                      })}
                      value={l.item_id}
                      onChange={(e) =>
                        updateLine(l.id, "item_id", e.target.value)
                      }
                    >
                      <NativeSelectOption value="">
                        {t("Select product")}
                      </NativeSelectOption>
                      {r?.items.map((x) => (
                        <NativeSelectOption key={x.id} value={x.id}>
                          {x.name}
                        </NativeSelectOption>
                      ))}
                    </NativeSelect>
                  </TableCell>
                  <TableCell>
                    <NativeSelect
                      aria-label={t("Purchase warehouse {index}", {
                        index: i + 1,
                      })}
                      value={l.warehouse_id}
                      onChange={(e) =>
                        updateLine(l.id, "warehouse_id", e.target.value)
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
                      aria-label={t("Purchase quantity {index}", {
                        index: i + 1,
                      })}
                      value={l.quantity}
                      onChange={(e) =>
                        updateLine(l.id, "quantity", e.target.value)
                      }
                    />
                  </TableCell>
                  <TableCell>
                    <Input
                      aria-label={t("Purchase price {index}", { index: i + 1 })}
                      value={l.unit_price}
                      onChange={(e) =>
                        updateLine(l.id, "unit_price", e.target.value)
                      }
                    />
                  </TableCell>
                  <TableCell>
                    <Input
                      aria-label={t("Purchase discount {index}", {
                        index: i + 1,
                      })}
                      value={l.line_discount}
                      onChange={(e) =>
                        updateLine(l.id, "line_discount", e.target.value)
                      }
                    />
                  </TableCell>
                  <TableCell>
                    <Button
                      aria-label={t("Remove purchase line {index}", {
                        index: i + 1,
                      })}
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        setLines(lines.filter((x) => x.id !== l.id));
                        setPreview(null);
                      }}
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
          onClick={() => {
            setLines([...lines, newLine()]);
            setPreview(null);
          }}
        >
          <Plus />
          {t("Add line")}
        </Button>
        {preview && (
          <Note>
            {t("Total:")} <Money value={preview.total} currency />
          </Note>
        )}
        <div className="form-actions">
          <Button
            variant="outline"
            disabled={busy}
            onClick={() => setDiscard(true)}
          >
            {t("Cancel")}
          </Button>
          <Button variant="outline" disabled={busy} onClick={() => run()}>
            {t("Review order")}
          </Button>
          <Button disabled={busy || !preview} onClick={() => run(true)}>
            {busy ? t("Working…") : t("Record purchase order")}
          </Button>
        </div>
      </Modal>
      <Modal
        open={discard}
        onOpenChange={setDiscard}
        title={t("Discard purchase editor?")}
        confirmation
        description={t(
          "Discard purchase order {value1} and its unsaved edits.",
          { value1: reference || t("Untitled order") },
        )}
      >
        <div className="form-actions">
          <Button variant="outline" onClick={() => setDiscard(false)}>
            {t("Keep editing")}
          </Button>
          <Button variant="destructive" onClick={onClose}>
            {t("Discard editor")}
          </Button>
        </div>
      </Modal>
    </>
  );
}

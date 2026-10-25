import { DatePicker } from "../components/ui/date-picker";
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
import { useEffect, useState } from "react";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { TrashIcon as Trash2 } from "@phosphor-icons/react/dist/csr/Trash";
import { InfoIcon as Info } from "@phosphor-icons/react/dist/csr/Info";
import { SidebarSimpleIcon as PanelRight } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { UserPlusIcon as UserPlus } from "@phosphor-icons/react/dist/csr/UserPlus";
import { FileTextIcon as FileText } from "@phosphor-icons/react/dist/csr/FileText";
import { toast } from "sonner";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "../components/ui/popover";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import { toThousandths } from "../lib/money-input";
import { priceRials, formatPercent } from "../lib/product-pricing";
import {
  emptyPartyProfile,
  emptyProductProfile,
  type Preview,
  type RecordEntry,
} from "../lib/types";
import { recordCommand, recordData, recordKind } from "./sales";
import { NewProduct } from "./quick-sale";
import { PageHeader, Field, ErrorState, Note } from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { Modal } from "../components/layout/modal";
import { DataGrid } from "../components/ui/data-grid";
import {
  Money,
  Status,
  formatDate,
  formatQuantity,
} from "../components/finance/money";
import { TabHint } from "../components/ui/tab-hint";

export function Purchases({
  onDetail,
}: {
  onDetail: (r: RecordEntry) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Orders");
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
      {
        <PageHeader
          title={b.t("Purchases")}
          syncedAt={records ? recordsSyncedAt : undefined}
          actions={
            <Button onClick={() => b.runAction("purchase")}>
              <Plus />
              {t("New purchase order")}
            </Button>
          }
        />
      }
      <TabsList>
        {["Orders", "Receipts", "Bills", "Returns"].map((x) => (
          <TabsTrigger key={x} value={x}>
            {b.t(x)}
          </TabsTrigger>
        ))}
      </TabsList>
      <TabHint>
        {
          {
            Orders: "Goods you ordered from suppliers.",
            Receipts: "Goods that have arrived.",
            Bills: "Supplier bills waiting to be paid.",
            Returns: "Goods sent back to suppliers.",
          }[tab] ?? ""
        }
      </TabHint>
      <TabsContent value={tab}>
        {error && <ErrorState error={error} />}{" "}
        {
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
        }
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

export function PurchaseEditor({
  onClose,
  seed = [],
}: {
  onClose: () => void;
  seed?: { item_id: string; quantity: string }[];
}) {
  const t = useTranslation();
  const b = useBooks();
  const r = b.workspace.resources;
  const profiles = b.workspace.product_profiles ?? {};
  const partyProfiles = b.workspace.party_profiles ?? {};
  const [key] = useState(uid);
  const [supplier, setSupplier] = useState("");
  const [reference, setReference] = useState("");
  const [date, setDate] = useState(today);
  const [buyMode, setBuyMode] = useState<"order" | "receive_bill">("order");
  const [payMethod, setPayMethod] = useState<"later" | "cash" | "bank">("later");
  const [showPreview, setShowPreview] = useState(true);
  const [vendorPopover, setVendorPopover] = useState(false);
  const [vendorDraft, setVendorDraft] = useState({
    name: "",
    phone: "",
    taxId: "",
    address: "",
  });
  const [quickProductLine, setQuickProductLine] = useState<string | null>(null);

  // A backorder opens this with the missing goods already listed.
  const [lines, setLines] = useState(() =>
    seed.length
      ? seed.map((s) => ({
          ...newLine(),
          item_id: s.item_id,
          quantity: s.quantity,
          unit_id:
            b.workspace.resources?.items.find((x) => x.id === s.item_id)
              ?.base_unit_id ?? "",
          unit_price:
            b.workspace.product_profiles?.[s.item_id]?.purchase_price ?? "",
          warehouse_id: b.workspace.resources?.warehouses[0]?.id ?? "",
        }))
      : [newLine()],
  );
  const [discount, setDiscount] = useState("0");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [discard, setDiscard] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.altKey && !e.ctrlKey && !e.metaKey && e.code === "KeyP") {
        e.preventDefault();
        setShowPreview((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

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
                    unit_price:
                      b.workspace.product_profiles?.[v]?.purchase_price ?? "",
                    unit_id:
                      r?.items.find((x) => x.id === v)?.base_unit_id ?? "",
                    warehouse_id:
                      l.warehouse_id || (r?.warehouses[0]?.id ?? ""),
                  }
                : {}),
            }
          : l,
      ),
    );
  }

  async function quickCreateSupplier() {
    const label = vendorDraft.name.trim() || vendorDraft.phone.trim();
    if (!label) return;
    const vKey = uid();
    try {
      await command("resource", {
        kind: "vendor",
        id: vKey,
        name: label,
        profile: {
          ...emptyPartyProfile,
          company_id: b.companyId,
          phone: vendorDraft.phone.trim(),
          tax_id: vendorDraft.taxId.trim(),
          address: vendorDraft.address.trim(),
        },
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: vKey,
        request_key: vKey,
        date: today(),
        default_unit_name: t("Piece"),
        reason: "Register vendor",
      });
      await b.refresh();
      setSupplier(vKey);
      setPreview(null);
      setVendorDraft({ name: "", phone: "", taxId: "", address: "" });
      setVendorPopover(false);
      toast.success(t("vendor created"));
    } catch (e) {
      setError((e as Error).message);
    }
  }

  function lineNet(l: (typeof lines)[number]) {
    const q = toThousandths(l.quantity) ?? 0n;
    const p = priceRials(l.unit_price) ?? 0n;
    const d = priceRials(l.line_discount) ?? 0n;
    const gross = (p * q) / 1000n;
    return gross > d ? gross - d : 0n;
  }

  const filledLines = lines.filter((l) => l.item_id);
  const subtotal = filledLines.reduce((acc, l) => acc + lineNet(l), 0n);
  const orderOff = priceRials(discount) ?? 0n;
  const liveTotal = subtotal > orderOff ? subtotal - orderOff : 0n;

  async function run(post = false) {
    setBusy(true);
    setError("");
    try {
      if (post) {
        const poRes = await command<{ record_id: string }>("purchase_post", {
          ...payload,
          expected_generation: preview?._generation,
        });
        // Remember updated purchase costs on product profiles
        for (const l of filledLines) {
          const p = profiles[l.item_id] ?? {
            ...emptyProductProfile,
            company_id: b.companyId,
          };
          if (l.unit_price.trim() && p.purchase_price !== l.unit_price.trim()) {
            void command("product_profile", {
              company_id: b.companyId,
              item_id: l.item_id,
              profile: { ...p, purchase_price: l.unit_price.trim() },
            }).catch(() => undefined);
          }
        }
        // Optional one-step Receive + Bill + Payment when buyMode === "receive_bill"
        if (buyMode === "receive_bill" && poRes?.record_id) {
          const recOpts = await command<{
            lines: { id: string; remaining_quantity: string }[];
          }>("operation_options", {
            company_id: b.companyId,
            record_id: poRes.record_id,
            operation: "receive",
          });
          const grKey = uid();
          const grRef = `${reference || "PO"}-GR`;
          const grPayload = {
            company_id: b.companyId,
            period_id: b.periodId,
            source_id: grKey,
            request_key: grKey,
            date,
            due_date: date,
            reference: grRef,
            reason: "Receive goods",
            record_id: poRes.record_id,
            lines: recOpts.lines.map((x) => ({
              id: x.id,
              quantity: formatQuantity(x.remaining_quantity),
            })),
            order_id: poRes.record_id,
            receipt_id: poRes.record_id,
          };
          const grPreview = await command<{ _generation: string }>(
            "receive_preview",
            grPayload,
          );
          const grPosted = await command<{ record_id: string }>("receive_post", {
            ...grPayload,
            expected_generation: grPreview._generation,
          });

          const billOpts = await command<{
            lines: { id: string; remaining_quantity: string }[];
          }>("operation_options", {
            company_id: b.companyId,
            record_id: grPosted.record_id,
            operation: "bill",
          });
          const billKey = uid();
          const billRef = `${reference || "PO"}-BILL`;
          const billPayload = {
            company_id: b.companyId,
            period_id: b.periodId,
            source_id: billKey,
            request_key: billKey,
            date,
            due_date: date,
            reference: billRef,
            reason: "Bill received goods",
            record_id: grPosted.record_id,
            lines: billOpts.lines.map((x) => ({
              id: x.id,
              quantity: formatQuantity(x.remaining_quantity),
            })),
            order_id: grPosted.record_id,
            receipt_id: grPosted.record_id,
          };
          const billPreview = await command<{ _generation: string }>(
            "bill_preview",
            billPayload,
          );
          const billPosted = await command<{ record_id: string }>("bill_post", {
            ...billPayload,
            expected_generation: billPreview._generation,
          });

          if (payMethod !== "later" && billPosted?.record_id) {
            const open = await command<{
              items: { id: string; remaining: string }[];
            }>("payment_options", {
              company_id: b.companyId,
              side: "make",
              party_id: supplier,
            });
            const owed = BigInt(
              open.items.find((i) => i.id === billPosted.record_id)?.remaining ??
                "0",
            );
            if (owed > 0n) {
              const payKey = uid();
              const payPayload = {
                company_id: b.companyId,
                period_id: b.periodId,
                source_id: payKey,
                request_key: payKey,
                date,
                reason: "Supplier bill settlement",
                side: "make",
                party_id: supplier,
                funds_role: payMethod,
                reference: billRef,
                advance_amount: "0",
                allocations: [
                  { open_item_id: billPosted.record_id, amount: String(owed) },
                ],
              };
              const payPrev = await command<{ _generation: string }>(
                "payment_preview",
                payPayload,
              );
              await command("payment_post", {
                ...payPayload,
                expected_generation: payPrev._generation,
              });
            }
          }
        }
        await b.refresh();
        toast.success(
          t(
            buyMode === "receive_bill"
              ? "Goods received and supplier bill recorded"
              : "Purchase order recorded",
          ),
        );
        onClose();
      } else {
        setPreview(await command("purchase_preview", payload));
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  const selectedSupplier = r?.parties.find((p) => p.id === supplier);
  const selectedSupplierProfile = supplier ? partyProfiles[supplier] : undefined;
  const companyName =
    b.workspace.companies.find((c) => c.id === b.companyId)?.name ??
    t("Your business");

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
        <div
          className="facture-workspace facture-buy"
          data-preview={showPreview ? "open" : "closed"}
        >
          {/* Color-Coded Buy Mode Banner */}
          <div className="facture-mode-bar" data-mode="buy">
            <div className="facture-mode-identity">
              <span className="facture-mode-pill">{t("BUY · PURCHASE")}</span>
              <strong className="facture-doc-number">
                {reference || t("Untitled order")}
              </strong>
              <span className="muted">
                {selectedSupplier ? selectedSupplier.name : t("Select supplier")}
              </span>
            </div>
            <div className="facture-mode-actions">
              <div
                className="pay-methods facture-mode-switch"
                role="radiogroup"
                aria-label={t("Purchase workflow mode")}
              >
                <button
                  type="button"
                  role="radio"
                  tabIndex={-1}
                  aria-checked={buyMode === "order"}
                  data-active={buyMode === "order"}
                  onClick={() => setBuyMode("order")}
                >
                  {t("Purchase order")}
                </button>
                <button
                  type="button"
                  role="radio"
                  tabIndex={-1}
                  aria-checked={buyMode === "receive_bill"}
                  data-active={buyMode === "receive_bill"}
                  onClick={() => setBuyMode("receive_bill")}
                >
                  {t("Receive & Bill")}
                </button>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setShowPreview((v) => !v)}
                title="Alt + P"
              >
                <PanelRight />
                {t(showPreview ? "Hide preview" : "Live preview")}
              </Button>
            </div>
          </div>

          {error && <ErrorState error={error} />}

          <div className="facture-body">
            <div className="facture-main-column">
              {/* ZONE 1: Top Identity Bar */}
              <div className="panel facture-identity-bar">
                <FieldGroup className="form-grid">
                  <Field
                    label={t("Supplier")}
                    action={
                      <div className="toolbar-actions">
                        <Popover
                          open={vendorPopover}
                          onOpenChange={setVendorPopover}
                        >
                          <PopoverTrigger asChild>
                            <button
                              type="button"
                              tabIndex={-1}
                              className="facture-inline-chip"
                            >
                              <UserPlus />
                              {t("+ Quick supplier")}
                            </button>
                          </PopoverTrigger>
                          <PopoverContent
                            align="start"
                            className="facture-upsert-popover"
                          >
                            <div className="facture-popover-head">
                              <strong>{t("Quick add supplier")}</strong>
                            </div>
                            <div className="facture-popover-fields">
                              <Field label={t("Name")}>
                                <Input
                                  value={vendorDraft.name}
                                  placeholder={t("Supplier name")}
                                  onChange={(e) =>
                                    setVendorDraft((d) => ({
                                      ...d,
                                      name: e.target.value,
                                    }))
                                  }
                                />
                              </Field>
                              <Field label={t("Phone")}>
                                <Input
                                  type="tel"
                                  value={vendorDraft.phone}
                                  placeholder="021..."
                                  onChange={(e) =>
                                    setVendorDraft((d) => ({
                                      ...d,
                                      phone: e.target.value,
                                    }))
                                  }
                                />
                              </Field>
                              <Field label={t("Tax / National ID")}>
                                <Input
                                  value={vendorDraft.taxId}
                                  onChange={(e) =>
                                    setVendorDraft((d) => ({
                                      ...d,
                                      taxId: e.target.value,
                                    }))
                                  }
                                />
                              </Field>
                              <Field label={t("Address")}>
                                <Input
                                  value={vendorDraft.address}
                                  onChange={(e) =>
                                    setVendorDraft((d) => ({
                                      ...d,
                                      address: e.target.value,
                                    }))
                                  }
                                />
                              </Field>
                              <div className="facture-popover-actions">
                                <Button
                                  type="button"
                                  size="sm"
                                  disabled={
                                    !vendorDraft.name.trim() &&
                                    !vendorDraft.phone.trim()
                                  }
                                  onClick={() => void quickCreateSupplier()}
                                >
                                  {t("Save & select")}
                                </Button>
                              </div>
                            </div>
                          </PopoverContent>
                        </Popover>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() =>
                            b.openResource("vendor", (id) => {
                              setSupplier(id);
                              setPreview(null);
                            })
                          }
                        >
                          {t("New vendor")}
                        </Button>
                      </div>
                    }
                  >
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

                  <Field
                    label={t("Reference")}
                    action={
                      !reference ? (
                        <button
                          type="button"
                          tabIndex={-1}
                          className="facture-inline-chip"
                          onClick={() => {
                            setReference(
                              `PO-${Number(b.workspace.purchase_count) + 1}`,
                            );
                            setPreview(null);
                          }}
                        >
                          {t("Auto #")}
                        </button>
                      ) : undefined
                    }
                  >
                    <Input
                      value={reference}
                      placeholder={`PO-${Number(b.workspace.purchase_count) + 1}`}
                      onChange={(e) => {
                        setReference(e.target.value);
                        setPreview(null);
                      }}
                    />
                  </Field>
                  <Field label={t("Financial / document date")}>
                    <DatePicker
                      value={date}
                      onChange={(value) => {
                        setDate(value);
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
              </div>

              {/* ZONE 2: Center Line-Item Grid */}
              <div className="panel facture-lines-panel">
                <div className="toolbar-actions facture-grid-toolbar">
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => b.openResource("item")}
                  >
                    {t("New product")}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      setQuickProductLine(lines[lines.length - 1]?.id ?? null)
                    }
                  >
                    <Plus />
                    {t("Quick-create item")}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => b.openResource("warehouse")}
                  >
                    {t("New warehouse")}
                  </Button>
                </div>
                <div className="table-scroll">
                  <Table className="editor-lines facture-grid-table">
                    <TableHeader>
                      <TableRow>
                        <TableHead className="col-item">{t("Product")}</TableHead>
                        <TableHead className="col-warehouse">
                          {t("Warehouse")}
                        </TableHead>
                        <TableHead className="col-qty">{t("Quantity")}</TableHead>
                        <TableHead className="col-price numeric">
                          {t("Price · IRR")}
                        </TableHead>
                        <TableHead className="col-discount numeric">
                          {t("Discount · IRR")}
                        </TableHead>
                        <TableHead className="col-total numeric">
                          {t("Amount")}
                        </TableHead>
                        <TableHead className="col-action" />
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {lines.map((l, i) => {
                        const profile = profiles[l.item_id];
                        const prevCost = priceRials(profile?.purchase_price);
                        const currCost = priceRials(l.unit_price);
                        const deltaBps =
                          prevCost && prevCost > 0n && currCost !== null && currCost > 0n
                            ? ((currCost - prevCost) * 10000n) / prevCost
                            : null;
                        return (
                          <TableRow key={l.id}>
                            <TableCell className="col-item">
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
                            <TableCell className="col-warehouse">
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
                            <TableCell className="col-qty">
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
                            <TableCell className="col-price">
                              <div className="price-cell">
                                <Input
                                  aria-label={t("Purchase price {index}", {
                                    index: i + 1,
                                  })}
                                  value={l.unit_price}
                                  onChange={(e) =>
                                    updateLine(l.id, "unit_price", e.target.value)
                                  }
                                />
                                {l.item_id && deltaBps !== null && deltaBps !== 0n && (
                                  <span
                                    className="margin-pill"
                                    data-tone={deltaBps <= 0n ? "positive" : "warning"}
                                  >
                                    {deltaBps > 0n ? "+" : ""}
                                    {formatPercent(String(deltaBps), b.numberLocale)}
                                  </span>
                                )}
                                {l.item_id && (
                                  <Popover>
                                    <PopoverTrigger asChild>
                                      <Button
                                        type="button"
                                        variant="ghost"
                                        size="icon"
                                        tabIndex={-1}
                                        aria-label={t("Price history")}
                                      >
                                        <Info />
                                      </Button>
                                    </PopoverTrigger>
                                    <PopoverContent align="end" className="price-memory">
                                      <dl>
                                        <dt>{t("Last purchase cost")}</dt>
                                        <dd>
                                          {prevCost === null ? (
                                            "—"
                                          ) : (
                                            <Money
                                              value={String(prevCost)}
                                              unit="Rial"
                                              locale={b.numberLocale}
                                            />
                                          )}
                                        </dd>
                                        <dt>{t("Listed selling price")}</dt>
                                        <dd>
                                          {profile?.sale_price ? (
                                            <Money
                                              value={profile.sale_price}
                                              unit="Rial"
                                              locale={b.numberLocale}
                                            />
                                          ) : (
                                            "—"
                                          )}
                                        </dd>
                                      </dl>
                                      {prevCost !== null && (
                                        <Button
                                          type="button"
                                          size="sm"
                                          variant="outline"
                                          onClick={() =>
                                            updateLine(
                                              l.id,
                                              "unit_price",
                                              String(prevCost),
                                            )
                                          }
                                        >
                                          {t("Use previous cost")}
                                        </Button>
                                      )}
                                    </PopoverContent>
                                  </Popover>
                                )}
                              </div>
                            </TableCell>
                            <TableCell className="col-discount">
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
                            <TableCell className="col-total numeric">
                              {l.item_id ? (
                                <Money
                                  value={String(lineNet(l))}
                                  unit="Rial"
                                  locale={b.numberLocale}
                                />
                              ) : (
                                "—"
                              )}
                            </TableCell>
                            <TableCell className="col-action">
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
                        );
                      })}
                    </TableBody>
                  </Table>
                </div>
                <div className="facture-add-line-bar">
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
                </div>
              </div>

              {preview && (
                <Note>
                  {t("Total:")} <Money value={preview.total} currency />
                </Note>
              )}

              {/* ZONE 4: Sticky Settlement Footer */}
              <div className="panel facture-settlement-footer">
                <div className="facture-settlement-left">
                  <div className="facture-settlement-label">
                    <span>
                      {t(
                        buyMode === "receive_bill"
                          ? "Receive & Bill settlement"
                          : "Purchase order summary",
                      )}
                    </span>
                    <small className="muted">
                      {t(
                        buyMode === "receive_bill"
                          ? "Records PO, receives stock into warehouse, and posts the supplier bill."
                          : "Records the purchase order. Stock updates when goods are received.",
                      )}
                    </small>
                  </div>
                  {buyMode === "receive_bill" && (
                    <div
                      className="pay-methods"
                      role="radiogroup"
                      aria-label={t("Supplier payment method")}
                    >
                      {(
                        [
                          ["later", "On account (AP)"],
                          ["cash", "Cash"],
                          ["bank", "Bank transfer"],
                        ] as const
                      ).map(([id, label]) => (
                        <button
                          key={id}
                          type="button"
                          role="radio"
                          tabIndex={-1}
                          aria-checked={payMethod === id}
                          data-active={payMethod === id}
                          onClick={() => setPayMethod(id)}
                        >
                          {t(label)}
                        </button>
                      ))}
                    </div>
                  )}
                </div>

                <div className="facture-settlement-right">
                  <div className="facture-totals-strip">
                    <div className="summary-line">
                      <span>{t("Subtotal")}</span>
                      <Money
                        value={String(subtotal)}
                        unit="Rial"
                        locale={b.numberLocale}
                      />
                    </div>
                    <div className="summary-line summary-total">
                      <span>{t("Total")}</span>
                      <Money
                        value={preview ? preview.total : String(liveTotal)}
                        unit="Rial"
                        locale={b.numberLocale}
                        currency
                      />
                    </div>
                  </div>
                  <div className="form-actions facture-buy-actions">
                    <Button
                      variant="outline"
                      disabled={busy}
                      onClick={() => setDiscard(true)}
                    >
                      {t("Cancel")}
                    </Button>
                    <Button
                      variant="outline"
                      disabled={busy}
                      onClick={() => void run()}
                    >
                      {t("Review order")}
                    </Button>
                    <Button
                      disabled={busy || !preview}
                      onClick={() => void run(true)}
                    >
                      {busy
                        ? t("Working…")
                        : buyMode === "receive_bill"
                          ? t("Record purchase order")
                          : t("Record purchase order")}
                    </Button>
                  </div>
                </div>
              </div>
            </div>

            {/* ZONE 3: Collapsible Right Live Preview Drawer */}
            {showPreview && (
              <aside
                className="panel facture-preview-drawer"
                aria-label={t("Live purchase preview")}
              >
                <div className="facture-paper">
                  <header className="facture-paper-header">
                    <div>
                      <span className="facture-paper-kicker">
                        {t(
                          buyMode === "receive_bill"
                            ? "SUPPLIER BILL & RECEIPT"
                            : "PURCHASE ORDER",
                        )}
                      </span>
                      <h4>{companyName}</h4>
                    </div>
                    <div className="facture-paper-meta">
                      <strong>{reference || t("Untitled order")}</strong>
                      <span>
                        {formatDate(
                          date,
                          b.dateLocale,
                          b.workspace.settings.date_display,
                        )}
                      </span>
                      <span
                        className="facture-status-stamp"
                        data-status={buyMode === "receive_bill" ? "paid" : "due"}
                      >
                        {buyMode === "receive_bill"
                          ? t("RECEIVE & BILL")
                          : t("ORDER")}
                      </span>
                    </div>
                  </header>

                  <div className="facture-paper-party">
                    <small>{t("Supplier")}</small>
                    <strong>
                      {selectedSupplier
                        ? selectedSupplier.name
                        : t("Select supplier")}
                    </strong>
                    {selectedSupplierProfile?.phone && (
                      <span>{selectedSupplierProfile.phone}</span>
                    )}
                    {selectedSupplierProfile?.address && (
                      <span>{selectedSupplierProfile.address}</span>
                    )}
                  </div>

                  <table className="facture-paper-table">
                    <thead>
                      <tr>
                        <th>{t("Product")}</th>
                        <th className="numeric">{t("Quantity")}</th>
                        <th className="numeric">{t("Price")}</th>
                        <th className="numeric">{t("Amount")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filledLines.length === 0 ? (
                        <tr>
                          <td colSpan={4} className="facture-paper-empty">
                            <FileText />
                            <span>{t("Items you add appear here live")}</span>
                          </td>
                        </tr>
                      ) : (
                        filledLines.map((l) => {
                          const item = r?.items.find((x) => x.id === l.item_id);
                          return (
                            <tr key={l.id}>
                              <td>{item?.name ?? "—"}</td>
                              <td className="numeric">{l.quantity}</td>
                              <td className="numeric">
                                <Money
                                  value={String(priceRials(l.unit_price) ?? 0n)}
                                  unit="Rial"
                                  locale={b.numberLocale}
                                />
                              </td>
                              <td className="numeric">
                                <Money
                                  value={String(lineNet(l))}
                                  unit="Rial"
                                  locale={b.numberLocale}
                                />
                              </td>
                            </tr>
                          );
                        })
                      )}
                    </tbody>
                  </table>

                  <div className="facture-paper-totals">
                    <div className="summary-line">
                      <span>{t("Subtotal")}</span>
                      <Money
                        value={String(subtotal)}
                        unit="Rial"
                        locale={b.numberLocale}
                      />
                    </div>
                    {orderOff > 0n && (
                      <div className="summary-line">
                        <span>{t("Order discount")}</span>
                        <Money
                          value={String(orderOff)}
                          unit="Rial"
                          locale={b.numberLocale}
                        />
                      </div>
                    )}
                    <div className="summary-line summary-total">
                      <span>{t("Total")}</span>
                      <Money
                        value={preview ? preview.total : String(liveTotal)}
                        unit="Rial"
                        locale={b.numberLocale}
                        currency
                      />
                    </div>
                  </div>
                </div>
              </aside>
            )}
          </div>
        </div>
      </Modal>
      {quickProductLine && (
        <NewProduct
          name=""
          onClose={() => setQuickProductLine(null)}
          onCreated={(itemId, _salePrice, costIrr) => {
            updateLine(quickProductLine, "item_id", itemId);
            if (costIrr) updateLine(quickProductLine, "unit_price", costIrr);
          }}
        />
      )}
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

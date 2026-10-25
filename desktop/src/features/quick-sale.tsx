import { useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { TrashIcon as Trash2 } from "@phosphor-icons/react/dist/csr/Trash";
import { ArrowLeftIcon as ArrowLeft } from "@phosphor-icons/react/dist/csr/ArrowLeft";
import { CheckCircleIcon as CheckCircle2 } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { InfoIcon as Info } from "@phosphor-icons/react/dist/csr/Info";
import { PrinterIcon as Printer } from "@phosphor-icons/react/dist/csr/Printer";
import { WarningIcon as Warning } from "@phosphor-icons/react/dist/csr/Warning";
import { SidebarSimpleIcon as PanelRight } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { UserPlusIcon as UserPlus } from "@phosphor-icons/react/dist/csr/UserPlus";
import { FileTextIcon as FileText } from "@phosphor-icons/react/dist/csr/FileText";
import { useBooks } from "../app/books";
import { useTranslation } from "../lib/i18n";
import { command, exportReport, today, uid } from "../lib/client";
import { toIrr, fromIrr, toThousandths } from "../lib/money-input";
import { priceRials, formatPercent } from "../lib/product-pricing";
import { useUnsavedChanges } from "../lib/use-unsaved";
import {
  emptyPartyProfile,
  emptyProductProfile,
  type Draft,
  type Report,
  type Workspace,
} from "../lib/types";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Textarea } from "../components/ui/textarea";
import { Combobox, type ComboOption } from "../components/ui/combobox";
import { DatePicker } from "../components/ui/date-picker";
import { Popover, PopoverContent, PopoverTrigger } from "../components/ui/popover";
import { Modal } from "../components/layout/modal";
import { ErrorState, Field, PageHeader } from "../components/ui/shared";
import { Money, formatQuantity, formatDate } from "../components/finance/money";

type Method = "cash" | "bank" | "later";
interface SaleLine {
  id: string;
  item_id: string;
  qty: string;
  price: string;
  discount: string;
  touched: boolean;
}
interface Shortage {
  item_id: string;
  name: string;
  missing: string;
}
interface Outcome {
  kind: "done" | "backorder";
  number: string;
  total: bigint;
  paid: bigint;
  invoiceId: string;
  shortages: Shortage[];
  printed: boolean;
}
const blankLine = (): SaleLine => ({
  id: uid(),
  item_id: "",
  qty: "1",
  price: "",
  discount: "",
  touched: false,
});
function plainQuantity(thousandths: bigint) {
  const frac = String(thousandths % 1000n)
    .padStart(3, "0")
    .replace(/0+$/, "");
  return `${thousandths / 1000n}${frac ? "." + frac : ""}`;
}
function addDays(iso: string, days: number) {
  const d = new Date(`${iso}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

export function QuickSale({ onClose }: { onClose: () => void }) {
  const [session, setSession] = useState(0);
  return (
    <SaleForm key={session} onClose={onClose} onNew={() => setSession((s) => s + 1)} />
  );
}

function SaleForm({ onClose, onNew }: { onClose: () => void; onNew: () => void }) {
  const t = useTranslation();
  const b = useBooks();
  const r = b.workspace.resources;
  const unit = b.unit;
  const unitName = unit === "Rial" ? "IRR" : t("toman");
  const profiles = b.workspace.product_profiles ?? {};
  const partyProfiles = b.workspace.party_profiles ?? {};
  const [customerId, setCustomerId] = useState("");
  const [date, setDate] = useState(today());
  const [lines, setLines] = useState<SaleLine[]>([blankLine()]);
  const [orderDiscount, setOrderDiscount] = useState("");
  const [note, setNote] = useState("");
  const [method, setMethod] = useState<Method>("cash");
  const [received, setReceived] = useState("");
  const [terms, setTerms] = useState("30");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [partial, setPartial] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [newProduct, setNewProduct] = useState<{ lineId: string; name: string } | null>(null);
  const [serverTotal, setServerTotal] = useState<bigint | null>(null);
  const [showPreview, setShowPreview] = useState(true);
  const [rememberDefaultPrice, setRememberDefaultPrice] = useState(true);
  const [contactOpen, setContactOpen] = useState(false);
  const [contactDraft, setContactDraft] = useState({
    name: "",
    phone: "",
    taxId: "",
    address: "",
  });
  const root = useRef<HTMLDivElement>(null);

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

  const stock = useMemo(() => {
    const map = new Map<string, bigint>();
    for (const s of b.report?.reconciliation?.stock ?? [])
      map.set(s.item_id, (map.get(s.item_id) ?? 0n) + BigInt(s.quantity_thousandths));
    return map;
  }, [b.report]);

  const customers = useMemo<ComboOption[]>(
    () =>
      (r?.parties ?? [])
        .filter((p) => p.customer)
        .map((p) => {
          const prof = b.workspace.party_profiles?.[p.id];
          return {
            id: p.id,
            label: p.name,
            meta: prof?.phone || prof?.tax_id || undefined,
            hint: prof?.city || undefined,
            search: `${prof?.phone ?? ""} ${prof?.tax_id ?? ""} ${prof?.email ?? ""}`,
          };
        }),
    [r?.parties, b.workspace.party_profiles],
  );

  const products = useMemo<ComboOption[]>(
    () =>
      (r?.items ?? []).map((i) => {
        const s = stock.get(i.id) ?? 0n;
        const prof = profiles[i.id];
        const salePrice = prof?.sale_price ? fromIrr(prof.sale_price, unit) : "";
        return {
          id: i.id,
          label: i.name,
          code: i.sku || undefined,
          meta: salePrice ? `${salePrice} ${unitName}` : undefined,
          search: `${i.sku} ${prof?.barcode ?? ""}`,
          hint:
            s > 0n
              ? `${formatQuantity(String(s), b.numberLocale)} ${t("in stock")}`
              : t("Out of stock"),
          tone: s > 0n ? "positive" : "warning",
        };
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [r?.items, stock, profiles, unit, unitName, b.numberLocale, b.locale],
  );

  const filled = lines.filter((l) => l.item_id);
  const net = (l: SaleLine) => {
    const q = toThousandths(l.qty) ?? 0n;
    const gross = (toIrr(l.price, unit) * q) / 1000n;
    const n = gross - toIrr(l.discount, unit);
    return n > 0n ? n : 0n;
  };
  const subtotal = filled.reduce((sum, l) => sum + net(l), 0n);
  const orderOff = toIrr(orderDiscount, unit);
  const estimate = subtotal > orderOff ? subtotal - orderOff : 0n;
  const total = serverTotal ?? estimate;
  const receivedIrr = received.trim() ? toIrr(received, unit) : total;
  const short = filled.filter((l) => {
    const q = toThousandths(l.qty) ?? 0n;
    return q > (stock.get(l.item_id) ?? 0n);
  });
  const dirty = !outcome && (filled.length > 0 || !!customerId || !!note);
  useUnsavedChanges(dirty);

  // The engine's own total (tax, rounding) replaces the estimate once it is calculable.
  const previewKey = JSON.stringify([
    filled.map((l) => [l.item_id, l.qty, l.price, l.discount]),
    orderDiscount,
    customerId,
    date,
  ]);
  useEffect(() => {
    setServerTotal(null);
    const customer = customerId || customers[0]?.id;
    const ready =
      customer &&
      filled.length > 0 &&
      filled.every((l) => toThousandths(l.qty) && l.price.trim() !== "");
    if (!ready) return;
    let active = true;
    const timer = setTimeout(() => {
      const draft = buildDraft(
        customer!,
        "preview",
        filled.map((l) => ({ ...l, warehouse: r?.warehouses[0]?.id ?? "" })),
      );
      command<{ preview: { total: string } }>("draft_preview", { draft })
        .then((res) => {
          if (active) setServerTotal(BigInt(res.preview.total));
        })
        .catch(() => undefined);
    }, 350);
    return () => {
      active = false;
      clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [previewKey]);

  function buildDraft(
    customer: string,
    reference: string,
    rows: (SaleLine & { warehouse: string })[],
  ): Draft {
    return {
      id: uid(),
      revision: "0",
      company_id: b.companyId,
      period_id: b.periodId,
      setup_id: b.workspace.setups[0]?.id ?? "",
      customer_id: customer,
      reference,
      document_date: date,
      financial_date: date,
      date_display: "Iso",
      display_unit: "Rial",
      document_discount: String(orderOff),
      supplied_total: "0",
      reason: note.trim() || "Goods sales order",
      quotation: null,
      lines: rows.map((l) => ({
        id: uid(),
        item_id: l.item_id,
        warehouse_id: l.warehouse,
        unit_id: r?.items.find((i) => i.id === l.item_id)?.base_unit_id ?? "",
        quantity: plainQuantity(toThousandths(l.qty) ?? 1000n),
        unit_price: String(toIrr(l.price, unit)),
        line_discount: String(toIrr(l.discount, unit)),
        price_reference: "Manual price",
        price_date: date,
        override_reason: "",
      })),
    };
  }
  function setLine(id: string, patch: Partial<SaleLine>) {
    setLines((ls) => {
      const next = ls.map((l) => (l.id === id ? { ...l, ...patch } : l));
      const last = next[next.length - 1];
      return last.item_id ? [...next, blankLine()] : next;
    });
    setError("");
  }
  function pickProduct(lineId: string, itemId: string) {
    const profile = profiles[itemId];
    setLine(lineId, {
      item_id: itemId,
      price: fromIrr(profile?.sale_price ?? null, unit),
      touched: false,
    });
  }
  function focusLine(id: string, field: string) {
    setTimeout(
      () =>
        root.current
          ?.querySelector<HTMLElement>(`[data-line="${id}"] [data-field="${field}"]`)
          ?.focus(),
      30,
    );
  }
  async function createCustomer(
    text: string,
    extra?: { phone?: string; taxId?: string; address?: string },
  ) {
    const phone = /^[\d+\-\s()۰-۹]{7,}$/.test(text);
    const key = uid();
    try {
      await command("resource", {
        kind: "customer",
        id: key,
        name: phone ? `${t("Guest")} ${text}` : text,
        profile: {
          ...emptyPartyProfile,
          company_id: b.companyId,
          ...(phone ? { phone: text } : {}),
          ...(extra?.phone ? { phone: extra.phone } : {}),
          ...(extra?.taxId ? { tax_id: extra.taxId } : {}),
          ...(extra?.address ? { address: extra.address } : {}),
        },
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: key,
        request_key: key,
        date: today(),
        default_unit_name: t("Piece"),
        reason: "Register customer",
      });
      await b.refresh();
      setCustomerId(key);
      toast.success(t("customer created"));
    } catch (e) {
      setError((e as Error).message);
    }
  }
  async function walkIn() {
    const label = t("Walk-in customer");
    const found = r?.parties.find((p) => p.customer && p.name === label);
    if (found) return found.id;
    const key = uid();
    await command("resource", {
      kind: "customer",
      id: key,
      name: label,
      profile: { ...emptyPartyProfile, company_id: b.companyId },
      company_id: b.companyId,
      period_id: b.periodId,
      source_id: key,
      request_key: key,
      date: today(),
      default_unit_name: t("Piece"),
      reason: "Register customer",
    });
    return key;
  }
  function nextNumber() {
    const taken = JSON.stringify([b.workspace.sales, b.workspace.drafts]);
    const prefix = b.workspace.business_profile?.invoice_prefix || "INV";
    let n = Math.floor(Number(b.workspace.sales_count) / 2) + 1;
    while (taken.includes(`"${prefix}-${n}"`) || taken.includes(`"ORD-${n}"`)) n++;
    return { invoice: `${prefix}-${n}`, order: `ORD-${n}` };
  }
  async function saveDraft() {
    setBusy(true);
    setError("");
    try {
      const customer = customerId || customers[0]?.id || "";
      const draft = buildDraft(
        customer,
        nextNumber().order,
        filled.map((l) => ({ ...l, warehouse: r?.warehouses[0]?.id ?? "" })),
      );
      const pv = await command<{ supplied_total: string }>("draft_preview", {
        draft,
      }).catch(() => null);
      await command("draft_save", {
        draft: { ...draft, supplied_total: pv?.supplied_total ?? "0" },
        expected_revision: null,
      });
      await b.refresh();
      toast.success(t("Draft saved"));
      onClose();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function complete() {
    if (busy) return;
    setBusy(true);
    setError("");
    setPartial(false);
    let orderDone = false;
    try {
      if (!filled.length) throw new Error(t("Add at least one product."));
      for (const l of filled) {
        const q = toThousandths(l.qty);
        if (!q || q <= 0n) throw new Error(t("Check the quantity on every line."));
        if (l.price.trim() === "") throw new Error(t("Enter a price on every line."));
      }
      const customer = customerId || (await walkIn());
      // Fresh stock: decide per line which warehouse can serve it.
      const fresh = await command<Report>("report", {
        company_id: b.companyId,
        from: b.from,
        to: today(),
      });
      const left = new Map<string, bigint>();
      for (const s of fresh.reconciliation?.stock ?? [])
        left.set(`${s.item_id}|${s.warehouse_id}`, BigInt(s.quantity_thousandths));
      const warehouses = r?.warehouses ?? [];
      const shortages: Shortage[] = [];
      const rows = filled.map((l) => {
        const q = toThousandths(l.qty)!;
        let best = warehouses[0]?.id ?? "";
        let bestStock = -1n;
        for (const w of warehouses) {
          const s = left.get(`${l.item_id}|${w.id}`) ?? 0n;
          if (s > bestStock) {
            best = w.id;
            bestStock = s;
          }
        }
        const have = bestStock < 0n ? 0n : bestStock;
        left.set(`${l.item_id}|${best}`, have > q ? have - q : 0n);
        if (have < q)
          shortages.push({
            item_id: l.item_id,
            name: r?.items.find((i) => i.id === l.item_id)?.name ?? "",
            missing: plainQuantity(q - have),
          });
        return { ...l, warehouse: best };
      });
      const { invoice, order } = nextNumber();
      const draft = buildDraft(customer, order, rows);
      const pv = await command<{ supplied_total: string }>("draft_preview", { draft });
      const saveable = { ...draft, supplied_total: pv.supplied_total };
      await command("draft_save", { draft: saveable, expected_revision: null });
      const stored = (
        await command<Workspace>("workspace", { company_id: b.companyId })
      ).drafts.find((d) => d.id === draft.id);
      if (!stored) throw new Error(t("Draft could not be saved."));
      const reviewed = await command<{ _generation: string }>("draft_preview", {
        draft: stored,
      });
      const posted = await command<{ record_id: string }>("draft_post", {
        id: stored.id,
        revision: stored.revision,
        request_key: uid(),
        expected_generation: reviewed._generation,
      });
      orderDone = true;
      let result: Outcome;
      if (shortages.length) {
        // Not enough stock: the order is on the books, the invoice waits for goods.
        result = {
          kind: "backorder",
          number: order,
          total: estimate,
          paid: 0n,
          invoiceId: "",
          shortages,
          printed: false,
        };
      } else {
        const opts = await command<{ lines: { id: string; remaining_quantity: string }[] }>(
          "operation_options",
          {
            company_id: b.companyId,
            record_id: posted.record_id,
            operation: "fulfill",
          },
        );
        const key = uid();
        const due = method === "later" ? addDays(date, Number(terms)) : date;
        const fp = {
          company_id: b.companyId,
          period_id: b.periodId,
          source_id: key,
          request_key: key,
          date,
          due_date: due,
          reference: invoice,
          reason: "Deliver goods and issue invoice",
          record_id: posted.record_id,
          lines: opts.lines.map((l) => ({
            id: l.id,
            quantity: formatQuantity(l.remaining_quantity),
          })),
          order_id: posted.record_id,
          receipt_id: posted.record_id,
        };
        const review = await command<{ _generation: string }>("fulfill_preview", fp);
        await command("fulfill_post", { ...fp, expected_generation: review._generation });
        const invoiceId = `${key}-invoice`;
        const open = await command<{ items: { id: string; remaining: string }[] }>(
          "payment_options",
          {
            company_id: b.companyId,
            side: "receive",
            party_id: customer,
          },
        );
        const owed = BigInt(open.items.find((i) => i.id === invoiceId)?.remaining ?? "0");
        let paid = 0n;
        if (method !== "later" && owed > 0n) {
          paid = receivedIrr < owed ? receivedIrr : owed;
          if (paid > 0n) {
            const pk = uid();
            const pay = {
              company_id: b.companyId,
              period_id: b.periodId,
              source_id: pk,
              request_key: pk,
              date,
              reason: "Invoice settlement",
              side: "receive",
              party_id: customer,
              funds_role: method,
              reference: invoice,
              advance_amount: "0",
              allocations: [{ open_item_id: invoiceId, amount: String(paid) }],
            };
            const pr = await command<{ _generation: string }>("payment_preview", pay);
            await command("payment_post", { ...pay, expected_generation: pr._generation });
          }
        }
        result = {
          kind: "done",
          number: invoice,
          total: owed,
          paid,
          invoiceId,
          shortages: [],
          printed: false,
        };
        if (isTauri()) {
          try {
            result.printed = await exportReport("sales_pdf", {
              company_id: b.companyId,
              document_id: invoiceId,
              locale: b.locale,
            });
          } catch {
            /* the sale is already complete; printing can be retried */
          }
        }
      }
      // Remember prices the owner typed so the next sale starts from them.
      if (rememberDefaultPrice) {
        for (const l of filled.filter((x) => x.touched)) {
          const p = profiles[l.item_id] ?? {
            ...emptyProductProfile,
            company_id: b.companyId,
          };
          const price = String(toIrr(l.price, unit));
          if (p.sale_price !== price)
            void command("product_profile", {
              company_id: b.companyId,
              item_id: l.item_id,
              profile: { ...p, sale_price: price },
            }).catch(() => undefined);
        }
      }
      await b.refresh();
      setOutcome(result);
      toast.success(
        t(result.kind === "done" ? "Sale completed" : "Backorder recorded"),
      );
    } catch (e) {
      setError((e as Error).message);
      setPartial(orderDone);
      if (orderDone) await b.refresh();
    } finally {
      setBusy(false);
    }
  }
  async function print() {
    if (!outcome?.invoiceId) return;
    try {
      if (
        await exportReport("sales_pdf", {
          company_id: b.companyId,
          document_id: outcome.invoiceId,
          locale: b.locale,
        })
      )
        toast.success(t("PDF exported"));
    } catch (e) {
      setError((e as Error).message);
    }
  }

  if (outcome)
    return (
      <>
        <PageHeader title={t("New sales invoice")} />
        <section className="panel sale-done" data-enter-scope>
          <CheckCircle2 className="sale-done-icon" data-kind={outcome.kind} />
          <h2>
            {t(outcome.kind === "done" ? "Sale completed" : "Recorded as a backorder")}
          </h2>
          <p>
            {outcome.number} ·{" "}
            <Money value={String(outcome.total)} unit={unit} locale={b.numberLocale} currency />
          </p>
          {outcome.kind === "done" ? (
            <ul className="sale-done-facts">
              <li>{t("Stock deducted, revenue and cost of goods posted.")}</li>
              <li>
                {outcome.paid > 0n
                  ? t("Payment recorded.")
                  : t("Left on account — it will show under receivables.")}
              </li>
              {outcome.paid > 0n && outcome.paid < outcome.total && (
                <li>
                  {t("Balance still due")}:{" "}
                  <Money
                    value={String(outcome.total - outcome.paid)}
                    unit={unit}
                    locale={b.numberLocale}
                    currency
                  />
                </li>
              )}
            </ul>
          ) : (
            <div className="sale-backorder">
              <Warning />
              <div>
                <strong>{t("Not enough stock to deliver yet")}</strong>
                <ul>
                  {outcome.shortages.map((s, i) => (
                    <li key={i}>
                      {s.name}: {t("short by")} {s.missing}
                    </li>
                  ))}
                </ul>
                <p>
                  {t(
                    "Order the missing goods, then deliver and invoice this order from Sales.",
                  )}
                </p>
              </div>
            </div>
          )}
          <div className="form-actions">
            {outcome.kind === "backorder" && (
              <Button
                onClick={() =>
                  b.openPurchase(
                    outcome.shortages.map((s) => ({
                      item_id: s.item_id,
                      quantity: s.missing,
                    })),
                  )
                }
              >
                {t("Create purchase order")}
              </Button>
            )}
            {outcome.kind === "done" && (
              <Button variant="outline" onClick={() => void print()}>
                <Printer />
                {t(outcome.printed ? "Print again" : "Print / save PDF")}
              </Button>
            )}
            <Button
              variant={outcome.kind === "done" ? "default" : "outline"}
              data-enter-submit
              onClick={onNew}
              autoFocus
            >
              <Plus />
              {t("New sale")}
            </Button>
            <Button variant="ghost" onClick={onClose}>
              {t("Back to sales")}
            </Button>
          </div>
        </section>
      </>
    );

  const lastIndex = lines.length - 1;
  const upcomingNumber = nextNumber().invoice;
  const selectedCustomer = r?.parties.find((p) => p.id === customerId);
  const selectedCustomerProfile = customerId ? partyProfiles[customerId] : undefined;
  const companyName =
    b.workspace.companies.find((c) => c.id === b.companyId)?.name ?? t("Your business");
  const biz = b.workspace.business_profile;
  const dueDate = method === "later" ? addDays(date, Number(terms)) : date;

  return (
    <div
      ref={root}
      data-enter-scope
      className="quick-sale facture-workspace facture-sell"
      data-preview={showPreview ? "open" : "closed"}
    >
      <PageHeader
        title={t("New sales invoice")}
        actions={
          <>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setShowPreview((v) => !v)}
              title="Alt + P"
            >
              <PanelRight />
              {t(showPreview ? "Hide facture preview" : "Live facture preview")}
            </Button>
            <Button variant="outline" disabled={busy} onClick={onClose}>
              <ArrowLeft className="directional-icon" />
              {t("Back to sales")}
            </Button>
            <Button
              variant="outline"
              disabled={busy || !filled.length}
              onClick={() => void saveDraft()}
            >
              {t("Save draft")}
            </Button>
          </>
        }
      />

      {/* Color-Coded Mode Banner */}
      <div className="facture-mode-bar" data-mode="sell">
        <div className="facture-mode-identity">
          <span className="facture-mode-pill">{t("SELL · FACTURE")}</span>
          <strong className="facture-doc-number">{upcomingNumber}</strong>
          <span className="muted">
            {selectedCustomer ? selectedCustomer.name : t("Walk-in customer")}
          </span>
        </div>
        <div className="facture-mode-shortcuts">
          <span>
            <kbd>Enter</kbd> {t("Next cell")}
          </span>
          <span>
            <kbd>Alt+P</kbd> {t("Preview")}
          </span>
          <span>
            <kbd>{t("Ctrl/⌘ + Enter")}</kbd> {t("Complete")}
          </span>
        </div>
      </div>

      {error && (
        <>
          <ErrorState error={error} />
          {partial && (
            <p className="sale-partial">
              {t(
                "The order was recorded but the invoice could not be finished. Open it from Sales to continue.",
              )}{" "}
              <Button variant="link" onClick={onClose}>
                {t("Go to Sales")}
              </Button>
            </p>
          )}
        </>
      )}

      <div className="facture-body">
        <div className="facture-main-column">
          {/* ZONE 1: Top Identity Bar */}
          <section className="panel facture-identity-bar">
            <div className="facture-identity-grid">
              <div className="facture-party-field">
                <Field
                  label={t("Customer")}
                  hint={t(
                    "Leave empty for a walk-in sale. Type a new name or phone to add one.",
                  )}
                  action={
                    <Popover open={contactOpen} onOpenChange={setContactOpen}>
                      <PopoverTrigger asChild>
                        <button
                          type="button"
                          tabIndex={-1}
                          className="facture-inline-chip"
                        >
                          <UserPlus />
                          {selectedCustomer
                            ? t("Customer info")
                            : t("+ Guest / Contact")}
                        </button>
                      </PopoverTrigger>
                      <PopoverContent align="start" className="facture-upsert-popover">
                        <div className="facture-popover-head">
                          <strong>{t("Quick customer / guest")}</strong>
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setCustomerId("");
                              setContactOpen(false);
                              toast.info(t("Using one-time walk-in guest"));
                            }}
                          >
                            {t("Use as One-Time Guest")}
                          </Button>
                        </div>
                        <div className="facture-popover-fields">
                          <Field label={t("Name")}>
                            <Input
                              value={contactDraft.name}
                              placeholder={t("Customer or company name")}
                              onChange={(e) =>
                                setContactDraft((d) => ({ ...d, name: e.target.value }))
                              }
                            />
                          </Field>
                          <Field label={t("Phone")}>
                            <Input
                              type="tel"
                              value={contactDraft.phone}
                              placeholder="0912..."
                              onChange={(e) =>
                                setContactDraft((d) => ({ ...d, phone: e.target.value }))
                              }
                            />
                          </Field>
                          <Field label={t("Tax / National ID")}>
                            <Input
                              value={contactDraft.taxId}
                              placeholder={t("Optional")}
                              onChange={(e) =>
                                setContactDraft((d) => ({ ...d, taxId: e.target.value }))
                              }
                            />
                          </Field>
                          <Field label={t("Address")}>
                            <Input
                              value={contactDraft.address}
                              placeholder={t("Optional")}
                              onChange={(e) =>
                                setContactDraft((d) => ({
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
                              disabled={!contactDraft.name.trim() && !contactDraft.phone.trim()}
                              onClick={() => {
                                const label =
                                  contactDraft.name.trim() || contactDraft.phone.trim();
                                void createCustomer(label, {
                                  phone: contactDraft.phone.trim(),
                                  taxId: contactDraft.taxId.trim(),
                                  address: contactDraft.address.trim(),
                                });
                                setContactDraft({
                                  name: "",
                                  phone: "",
                                  taxId: "",
                                  address: "",
                                });
                                setContactOpen(false);
                              }}
                            >
                              {t("Save & select")}
                            </Button>
                          </div>
                        </div>
                      </PopoverContent>
                    </Popover>
                  }
                >
                  <Combobox
                    autoFocus
                    label={t("Customer")}
                    value={customerId}
                    options={customers}
                    onChange={setCustomerId}
                    onCreate={(text) => void createCustomer(text)}
                    createLabel={(text) => t('Add "{name}" as a customer', { name: text })}
                    placeholder={t("Walk-in customer")}
                    emptyText={t("No customers yet — type a name")}
                  />
                </Field>
              </div>
              <Field label={t("Invoice #")}>
                <div className="facture-readonly-pill">{upcomingNumber}</div>
              </Field>
              <Field label={t("Date")}>
                <DatePicker value={date} onChange={setDate} />
              </Field>
            </div>
          </section>

          {/* ZONE 2: Center Line-Item Grid */}
          <section className="panel facture-lines-panel">
            <div className="table-scroll">
              <table className="quick-lines facture-grid-table">
                <thead>
                  <tr>
                    <th className="col-item">{t("Product")}</th>
                    <th className="col-qty">{t("Quantity")}</th>
                    <th className="col-price">{`${t("Price")} · ${unitName}`}</th>
                    <th className="col-discount">{`${t("Discount")} · ${unitName}`}</th>
                    <th className="col-total numeric">{t("Amount")}</th>
                    <th className="col-action" />
                  </tr>
                </thead>
                <tbody>
                  {lines.map((l, i) => {
                    const have = stock.get(l.item_id) ?? 0n;
                    const q = toThousandths(l.qty) ?? 0n;
                    const profile = profiles[l.item_id];
                    const itemObj = r?.items.find((x) => x.id === l.item_id);
                    const unitObj = r?.units.find((u) => u.id === itemObj?.base_unit_id);
                    const cost = priceRials(profile?.purchase_price);
                    const price = toIrr(l.price, unit);
                    const marginBps =
                      cost !== null && price > 0n
                        ? ((price - cost) * 10000n) / price
                        : null;
                    return (
                      <tr key={l.id} data-line={l.id}>
                        <td className="quick-product col-item">
                          <Combobox
                            label={t("Product {index}", { index: i + 1 })}
                            value={l.item_id}
                            options={products}
                            onChange={(id) => pickProduct(l.id, id)}
                            onCreate={(text) => setNewProduct({ lineId: l.id, name: text })}
                            createLabel={(text) => t('Create "{name}"', { name: text })}
                            placeholder={t("Search product or type a new one")}
                            emptyText={t("No products yet — type a name")}
                          />
                          {l.item_id && q > have && (
                            <small className="stock-warning">
                              {have > 0n
                                ? t("Only {count} in stock — the rest will be backordered", {
                                    count: formatQuantity(String(have), b.numberLocale),
                                  })
                                : t("Out of stock — will be backordered")}
                            </small>
                          )}
                        </td>
                        <td className="col-qty">
                          <div className="qty-cell-wrap">
                            <Input
                              data-field="qty"
                              aria-label={t("Quantity {index}", { index: i + 1 })}
                              inputMode="decimal"
                              value={l.qty}
                              onChange={(e) => setLine(l.id, { qty: e.target.value })}
                            />
                            {unitObj && <span className="qty-unit-badge">{unitObj.name}</span>}
                          </div>
                        </td>
                        <td className="col-price">
                          <div className="price-cell">
                            <Input
                              data-field="price"
                              aria-label={t("Price {index}", { index: i + 1 })}
                              inputMode="numeric"
                              value={l.price}
                              onChange={(e) =>
                                setLine(l.id, { price: e.target.value, touched: true })
                              }
                            />
                            {l.item_id && marginBps !== null && (
                              <span
                                className="margin-pill"
                                data-tone={marginBps >= 0n ? "positive" : "warning"}
                              >
                                {formatPercent(String(marginBps), b.numberLocale)}
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
                                  <div className="price-memory-title">
                                    <strong>{itemObj?.name}</strong>
                                    {itemObj?.sku && <span>{itemObj.sku}</span>}
                                  </div>
                                  <dl>
                                    <dt>{t("Last purchase cost")}</dt>
                                    <dd>
                                      {cost === null ? (
                                        "—"
                                      ) : (
                                        <Money
                                          value={String(cost)}
                                          unit={unit}
                                          locale={b.numberLocale}
                                        />
                                      )}
                                    </dd>
                                    <dt>{t("Listed selling price")}</dt>
                                    <dd>
                                      {profile?.sale_price ? (
                                        <Money
                                          value={profile.sale_price}
                                          unit={unit}
                                          locale={b.numberLocale}
                                        />
                                      ) : (
                                        "—"
                                      )}
                                    </dd>
                                    <dt>{t("Margin at this price")}</dt>
                                    <dd>
                                      {marginBps !== null
                                        ? formatPercent(String(marginBps), b.numberLocale)
                                        : "—"}
                                    </dd>
                                  </dl>
                                  {profile?.sale_price && (
                                    <Button
                                      type="button"
                                      size="sm"
                                      variant="outline"
                                      onClick={() =>
                                        setLine(l.id, {
                                          price: fromIrr(profile.sale_price, unit),
                                          touched: false,
                                        })
                                      }
                                    >
                                      {t("Use listed price")}
                                    </Button>
                                  )}
                                  <label className="price-memory-toggle">
                                    <input
                                      type="checkbox"
                                      checked={rememberDefaultPrice}
                                      onChange={(e) =>
                                        setRememberDefaultPrice(e.target.checked)
                                      }
                                    />
                                    <span>
                                      {t("A new price you type is remembered for next time.")}
                                    </span>
                                  </label>
                                </PopoverContent>
                              </Popover>
                            )}
                          </div>
                        </td>
                        <td className="col-discount">
                          <Input
                            data-field="discount"
                            aria-label={t("Discount {index}", { index: i + 1 })}
                            inputMode="numeric"
                            value={l.discount}
                            placeholder="0"
                            onChange={(e) => setLine(l.id, { discount: e.target.value })}
                            onKeyDown={(e) => {
                              // Enter on the last row starts the next row, like a till.
                              if (
                                e.key === "Enter" &&
                                !e.ctrlKey &&
                                !e.metaKey &&
                                i === lastIndex - 1 &&
                                l.item_id
                              ) {
                                e.preventDefault();
                                const el = root.current?.querySelector<HTMLElement>(
                                  `[data-line="${lines[lastIndex].id}"] input`,
                                );
                                el?.focus();
                              }
                            }}
                          />
                        </td>
                        <td className="col-total numeric">
                          {l.item_id ? (
                            <Money
                              value={String(net(l))}
                              unit={unit}
                              locale={b.numberLocale}
                            />
                          ) : (
                            ""
                          )}
                        </td>
                        <td className="col-action">
                          {lines.length > 1 && (
                            <Button
                              variant="ghost"
                              size="icon"
                              tabIndex={-1}
                              aria-label={t("Remove line {index}", { index: i + 1 })}
                              onClick={() =>
                                setLines((ls) => ls.filter((x) => x.id !== l.id))
                              }
                            >
                              <Trash2 />
                            </Button>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            <div className="facture-note-row">
              <Field label={t("Note (optional)")}>
                <Textarea
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  rows={1}
                  placeholder={t("Delivery remarks, serial numbers, or invoice note…")}
                />
              </Field>
            </div>
          </section>

          {/* ZONE 4: Sticky Settlement Footer */}
          <aside className="panel facture-settlement-footer">
            <div className="facture-settlement-left">
              <div className="facture-settlement-label">
                <span>{t("Summary")}</span>
                <small className="muted">
                  {short.length
                    ? t(
                        "Some items are short. The order is recorded and you can create a purchase order next.",
                      )
                    : t(
                        "Deducts stock, posts revenue and cost, records the payment and saves the invoice PDF.",
                      )}
                </small>
              </div>
              <div
                className="pay-methods"
                role="radiogroup"
                aria-label={t("How is the customer paying?")}
              >
                {(
                  [
                    ["cash", "Cash"],
                    ["bank", "Card / bank transfer"],
                    ["later", "Pay later"],
                  ] as const
                ).map(([id, label]) => (
                  <button
                    key={id}
                    type="button"
                    role="radio"
                    tabIndex={-1}
                    aria-checked={method === id}
                    data-active={method === id}
                    onClick={() => setMethod(id)}
                  >
                    {t(label)}
                  </button>
                ))}
              </div>
              <div className="facture-settlement-conditional">
                {method === "later" ? (
                  <Field label={t("Due in")}>
                    <select
                      className="quick-select"
                      value={terms}
                      onChange={(e) => setTerms(e.target.value)}
                    >
                      {["0", "7", "14", "30", "60"].map((d) => (
                        <option key={d} value={d}>
                          {d === "0"
                            ? t("Due immediately")
                            : t("{days} days", { days: d })}
                        </option>
                      ))}
                    </select>
                  </Field>
                ) : (
                  <Field
                    label={`${t("Amount received")} · ${unitName}`}
                    hint={t("Leave empty if paid in full.")}
                  >
                    <Input
                      inputMode="numeric"
                      value={received}
                      placeholder={fromIrr(total, unit)}
                      onChange={(e) => setReceived(e.target.value)}
                    />
                  </Field>
                )}
                {method !== "later" && receivedIrr < total && total > 0n && (
                  <p className="muted facture-balance-due">
                    {t("Balance on account")}:{" "}
                    <Money
                      value={String(total - receivedIrr)}
                      unit={unit}
                      locale={b.numberLocale}
                      currency
                    />
                  </p>
                )}
              </div>
            </div>

            <div className="facture-settlement-right">
              <div className="facture-totals-strip">
                <div className="summary-line">
                  <span>{t("Subtotal")}</span>
                  <Money value={String(subtotal)} unit={unit} locale={b.numberLocale} />
                </div>
                <div className="facture-discount-inline">
                  <Field label={`${t("Order discount")} · ${unitName}`}>
                    <Input
                      inputMode="numeric"
                      value={orderDiscount}
                      placeholder="0"
                      onChange={(e) => setOrderDiscount(e.target.value)}
                    />
                  </Field>
                </div>
                <div className="summary-line summary-total">
                  <span>{t("Total")}</span>
                  <Money
                    value={String(total)}
                    unit={unit}
                    locale={b.numberLocale}
                    currency
                  />
                </div>
              </div>
              <div className="facture-cta-stack">
                <Button
                  className="quick-sale-complete"
                  size="lg"
                  data-enter-submit
                  disabled={busy || !filled.length}
                  onClick={() => void complete()}
                >
                  <CheckCircle2 />
                  {busy
                    ? t("Working…")
                    : short.length
                      ? t("Record as backorder")
                      : t("Complete sale & print")}
                </Button>
                <small className="muted">{t("Ctrl/⌘ + Enter")}</small>
              </div>
            </div>
          </aside>
        </div>

        {/* ZONE 3: Collapsible Right Live Preview Drawer */}
        {showPreview && (
          <aside
            className="panel facture-preview-drawer"
            aria-label={t("Live facture preview")}
          >
            <div className="facture-paper">
              <header className="facture-paper-header">
                <div>
                  <span className="facture-paper-kicker">{t("SALES INVOICE · FACTURE")}</span>
                  <h4>{companyName}</h4>
                  {biz?.phone && <small>{biz.phone}</small>}
                  {biz?.registration_id && <small>#{biz.registration_id}</small>}
                </div>
                <div className="facture-paper-meta">
                  <strong>{upcomingNumber}</strong>
                  <span>
                    {formatDate(date, b.dateLocale, b.workspace.settings.date_display)}
                  </span>
                  <span
                    className="facture-status-stamp"
                    data-status={
                      short.length ? "backorder" : method === "later" ? "due" : "paid"
                    }
                  >
                    {short.length
                      ? t("BACKORDER")
                      : method === "later"
                        ? `${t("DUE")} ${formatDate(dueDate, b.dateLocale, b.workspace.settings.date_display)}`
                        : method === "cash"
                          ? t("PAID · CASH")
                          : t("PAID · BANK")}
                  </span>
                </div>
              </header>

              <div className="facture-paper-party">
                <small>{t("Customer")}</small>
                <strong>
                  {selectedCustomer ? selectedCustomer.name : t("Walk-in customer")}
                </strong>
                {selectedCustomerProfile?.phone && (
                  <span>{selectedCustomerProfile.phone}</span>
                )}
                {selectedCustomerProfile?.address && (
                  <span>{selectedCustomerProfile.address}</span>
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
                  {filled.length === 0 ? (
                    <tr>
                      <td colSpan={4} className="facture-paper-empty">
                        <FileText />
                        <span>{t("Items you add appear here live")}</span>
                      </td>
                    </tr>
                  ) : (
                    filled.map((l) => {
                      const item = r?.items.find((x) => x.id === l.item_id);
                      return (
                        <tr key={l.id}>
                          <td>{item?.name ?? "—"}</td>
                          <td className="numeric">{l.qty}</td>
                          <td className="numeric">
                            <Money
                              value={String(toIrr(l.price, unit))}
                              unit={unit}
                              locale={b.numberLocale}
                            />
                          </td>
                          <td className="numeric">
                            <Money
                              value={String(net(l))}
                              unit={unit}
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
                  <Money value={String(subtotal)} unit={unit} locale={b.numberLocale} />
                </div>
                {orderOff > 0n && (
                  <div className="summary-line">
                    <span>{t("Order discount")}</span>
                    <Money value={String(orderOff)} unit={unit} locale={b.numberLocale} />
                  </div>
                )}
                <div className="summary-line summary-total">
                  <span>{t("Total")}</span>
                  <Money
                    value={String(total)}
                    unit={unit}
                    locale={b.numberLocale}
                    currency
                  />
                </div>
              </div>

              {(biz?.bank_details || biz?.footer || note.trim()) && (
                <footer className="facture-paper-footer">
                  {note.trim() && <p>{note}</p>}
                  {(biz?.bank_details || biz?.footer) && (
                    <small>
                      {[biz.bank_details, biz.footer]
                        .filter(Boolean)
                        .join(" · ")}
                    </small>
                  )}
                </footer>
              )}
            </div>
          </aside>
        )}
      </div>

      {newProduct && (
        <NewProduct
          name={newProduct.name}
          onClose={() => {
            const id = newProduct.lineId;
            setNewProduct(null);
            focusLine(id, "qty");
          }}
          onCreated={(itemId, price) => {
            setLine(newProduct.lineId, { item_id: itemId, price, touched: false });
          }}
        />
      )}
    </div>
  );
}

// Inline Item Quick-Create Micro-Dialog ("The Missing Turbine Case"):
// Pre-fills Name, lets you toggle Stock vs Service, and advances Cost -> Selling Price -> Qty.
export function NewProduct({
  name: initial,
  onClose,
  onCreated,
}: {
  name: string;
  onClose: () => void;
  onCreated: (itemId: string, price: string, cost?: string) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [name, setName] = useState(initial);
  const [kind, setKind] = useState<"stock" | "service">("stock");
  const [cost, setCost] = useState("");
  const [price, setPrice] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => {
    // Typing from the sale line continues here: jump to the first empty field.
    setTimeout(
      () => form.current?.querySelector<HTMLInputElement>("input[data-cost]")?.focus(),
      50,
    );
  }, []);
  async function save() {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      if (!name.trim()) throw new Error(t("Enter a product name."));
      const key = uid();
      const costIrr = toIrr(cost, b.unit);
      const saleIrr = toIrr(price, b.unit);
      const period = b.workspace.periods.find((p) => p.id === b.periodId);
      await command("resource", {
        kind: "item",
        id: key,
        name: name.trim(),
        sku: "",
        unit_id: b.workspace.resources?.units[0]?.id ?? "",
        product_profile: {
          ...emptyProductProfile,
          company_id: b.companyId,
          category: kind === "service" ? "Service" : "",
          purchase_price: cost.trim() ? String(costIrr) : null,
          sale_price: price.trim() ? String(saleIrr) : null,
        },
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: key,
        request_key: key,
        date: period ? [period.start_date, today(), period.end_date].sort()[1] : today(),
        default_unit_name: t("Piece"),
        reason: "Register item",
      });
      await b.refresh();
      onCreated(
        key,
        price.trim() ? fromIrr(saleIrr, b.unit) : "",
        cost.trim() ? String(costIrr) : "",
      );
      toast.success(t("item created"));
      onClose();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const unitName = b.unit === "Rial" ? "IRR" : t("toman");
  const costIrr = toIrr(cost, b.unit);
  const saleIrr = toIrr(price, b.unit);
  const liveMargin =
    costIrr > 0n && saleIrr > 0n
      ? formatPercent(String(((saleIrr - costIrr) * 10000n) / saleIrr), b.numberLocale)
      : null;

  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title={t("New product")}
      description={t("Add it to your inventory and this sale at the same time.")}
    >
      <form
        ref={form}
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        {error && <ErrorState error={error} />}
        <div className="quick-create-type-bar">
          <div className="pay-methods" role="radiogroup" aria-label={t("Product type")}>
            {(
              [
                ["stock", "Stock item · Track inventory"],
                ["service", "Service / Non-stock"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                role="radio"
                tabIndex={-1}
                aria-checked={kind === id}
                data-active={kind === id}
                onClick={() => setKind(id)}
              >
                {t(label)}
              </button>
            ))}
          </div>
        </div>
        <div className="form-grid">
          <Field label={t("Name")}>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={160}
            />
          </Field>
          <Field label={`${t("Your cost")} · ${unitName}`}>
            <Input
              data-cost
              inputMode="numeric"
              value={cost}
              onChange={(e) => setCost(e.target.value)}
            />
          </Field>
          <Field
            label={`${t("Selling price")} · ${unitName}`}
            action={
              liveMargin ? (
                <span className="margin-pill" data-tone="positive">
                  {t("Margin")}: {liveMargin}
                </span>
              ) : undefined
            }
          >
            <Input
              inputMode="numeric"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
            />
          </Field>
        </div>
        <div className="form-actions">
          <Button type="button" variant="outline" onClick={onClose} disabled={busy}>
            {t("Cancel")}
          </Button>
          <Button type="submit" disabled={busy}>
            {busy ? t("Saving…") : t("Add product")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

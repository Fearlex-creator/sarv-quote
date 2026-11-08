import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import { fromIrr, toIrr, toThousandths } from "../lib/money-input";
import type { Draft, Workspace } from "../lib/types";
import { emptyPartyProfile } from "../lib/types";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { ErrorState, PageHeader } from "../components/ui/shared";
import { Money, formatQuantity } from "../components/finance/money";

interface CartLine {
  id: string;
  item_id: string;
  name: string;
  qty: string;
  price: string;
}
interface Held {
  id: string;
  label: string;
  payload: string;
}

function qtyText(thousandths: bigint) {
  const fraction = String(thousandths % 1000n).padStart(3, "0").replace(/0+$/, "");
  return `${thousandths / 1000n}${fraction ? "." + fraction : ""}`;
}

export function Pos() {
  const b = useBooks();
  const t = b.t;
  const resources = b.workspace.resources;
  const profiles = useMemo(
    () => b.workspace.product_profiles ?? {},
    [b.workspace.product_profiles],
  );
  const retail = b.workspace.retail;
  const shift = retail?.open_shift ?? null;
  const [query, setQuery] = useState("");
  const [lines, setLines] = useState<CartLine[]>([]);
  const [customerId, setCustomerId] = useState("");
  const [method, setMethod] = useState<"cash" | "bank" | "account" | "split">("cash");
  const [cashPart, setCashPart] = useState("");
  const [bankPart, setBankPart] = useState("");
  const [opening, setOpening] = useState("");
  const [counted, setCounted] = useState("");
  const [cashier, setCashier] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [online, setOnline] = useState(navigator.onLine);
  const [receipt, setReceipt] = useState<{
    number: string;
    total: string;
    paid: string;
    method: string;
    lines: CartLine[];
  } | null>(null);
  const lock = useRef(false);
  const scan = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const on = () => setOnline(true);
    const off = () => setOnline(false);
    window.addEventListener("online", on);
    window.addEventListener("offline", off);
    return () => {
      window.removeEventListener("online", on);
      window.removeEventListener("offline", off);
    };
  }, []);

  const catalog = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (resources?.items ?? [])
      .map((item) => ({
        item,
        profile: profiles[item.id],
        stock: (b.report?.reconciliation?.stock ?? [])
          .filter((row) => row.item_id === item.id)
          .reduce((sum, row) => sum + BigInt(row.quantity_thousandths), 0n),
      }))
      .filter(({ item, profile }) => {
        if (!needle) return true;
        const hay = `${item.name} ${item.sku} ${profile?.barcode ?? ""}`.toLowerCase();
        return hay.includes(needle);
      })
      .slice(0, 40);
  }, [resources, profiles, query, b.report]);

  function addItem(itemId: string, name: string) {
    setLines((current) => {
      const found = current.find((line) => line.item_id === itemId);
      if (found) {
        const next = toThousandths(found.qty) ?? 1000n;
        return current.map((line) =>
          line.id === found.id ? { ...line, qty: qtyText(next + 1000n) } : line,
        );
      }
      return [
        ...current,
        {
          id: uid(),
          item_id: itemId,
          name,
          qty: "1",
          price: fromIrr(profiles[itemId]?.sale_price ?? null, b.unit),
        },
      ];
    });
    setQuery("");
    scan.current?.focus();
  }

  function onScan(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key !== "Enter") return;
    event.preventDefault();
    const needle = query.trim();
    if (!needle) return;
    const match = (resources?.items ?? []).find((item) => {
      const profile = profiles[item.id];
      return item.sku === needle || profile?.barcode === needle || item.name === needle;
    });
    if (!match) {
      setError(t("No product matches that code"));
      return;
    }
    setError("");
    addItem(match.id, match.name);
  }

  async function openTill() {
    setBusy(true);
    setError("");
    try {
      await command("pos_shift_open", {
        company_id: b.companyId,
        request_key: uid(),
        opening_cash_rial: String(toIrr(opening || "0", b.unit)),
        cashier: cashier.trim() || t("Cashier"),
        date: today(),
      });
      setOpening("");
      await b.refresh();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function closeTill() {
    if (!shift) return;
    setBusy(true);
    setError("");
    try {
      const result = await command<{
        shift: { expected_cash_rial: string; variance_rial: string };
        notice: string;
      }>("pos_shift_close", {
        company_id: b.companyId,
        shift_id: shift.id,
        counted_cash_rial: String(toIrr(counted || "0", b.unit)),
        date: today(),
        note: "Counted at the till",
      });
      toast.success(result.notice);
      setCounted("");
      await b.refresh();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function hold() {
    if (!lines.length) return;
    await command("pos_hold", {
      company_id: b.companyId,
      id: uid(),
      label: lines.map((line) => line.name).join("، ").slice(0, 80),
      date: today(),
      payload: JSON.stringify(lines),
    });
    setLines([]);
    await b.refresh();
    toast.success(t("Sale held"));
  }

  function resume(held: Held) {
    setLines(JSON.parse(held.payload) as CartLine[]);
    void command("pos_hold_delete", { company_id: b.companyId, id: held.id }).then(() => b.refresh());
  }

  async function walkIn() {
    const label = t("Walk-in customer");
    const found = resources?.parties.find((party) => party.customer && party.name === label);
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
      reason: "Walk-in customer",
    });
    return key;
  }

  async function checkout() {
    if (lock.current || busy) return;
    if (!online) {
      setError(t("You are offline. Checkout is paused so a sale cannot look successful without being saved."));
      return;
    }
    if (!shift) {
      setError(t("Open a till before checkout."));
      return;
    }
    lock.current = true;
    setBusy(true);
    setError("");
    try {
      if (!lines.length) throw new Error(t("Add at least one product."));
      const customer = customerId || (await walkIn());
      const warehouse = resources?.warehouses[0]?.id ?? "";
      if (!warehouse) throw new Error(t("Add a warehouse before checkout."));
      const draft: Draft = {
        id: uid(),
        revision: "0",
        company_id: b.companyId,
        period_id: b.periodId,
        setup_id: b.workspace.setups[0]?.id ?? "",
        customer_id: customer,
        reference: `ORD-${uid().slice(0, 8)}`,
        document_date: today(),
        financial_date: today(),
        date_display: "Iso",
        display_unit: "Rial",
        document_discount: "0",
        supplied_total: "0",
        reason: "Point of sale",
        quotation: null,
        lines: lines.map((line) => ({
          id: uid(),
          item_id: line.item_id,
          warehouse_id: warehouse,
          unit_id: resources?.items.find((item) => item.id === line.item_id)?.base_unit_id ?? "",
          quantity: qtyText(toThousandths(line.qty) ?? 0n),
          unit_price: String(toIrr(line.price, b.unit)),
          line_discount: "0",
          price_reference: "Till price",
          price_date: today(),
          override_reason: "",
        })),
      };
      const preview = await command<{ supplied_total: string }>("draft_preview", { draft });
      const saveable = { ...draft, supplied_total: preview.supplied_total };
      await command("draft_save", { draft: saveable, expected_revision: null });
      const stored = (await command<Workspace>("workspace", { company_id: b.companyId })).drafts.find(
        (row) => row.id === draft.id,
      );
      if (!stored) throw new Error(t("Draft could not be saved."));
      const reviewed = await command<{ _generation: string }>("draft_preview", { draft: stored });
      const posted = await command<{ record_id: string }>("draft_post", {
        id: stored.id,
        revision: stored.revision,
        request_key: uid(),
        expected_generation: reviewed._generation,
      });
      const options = await command<{ lines: { id: string; remaining_quantity: string }[] }>(
        "operation_options",
        { company_id: b.companyId, record_id: posted.record_id, operation: "fulfill" },
      );
      const key = uid();
      const invoice = `POS-${key.slice(0, 8)}`;
      const fulfill = {
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: key,
        request_key: key,
        date: today(),
        due_date: today(),
        reference: invoice,
        reason: "Till sale",
        record_id: posted.record_id,
        order_id: posted.record_id,
        lines: options.lines.map((line) => ({
          id: line.id,
          quantity: formatQuantity(line.remaining_quantity),
        })),
      };
      const review = await command<{ _generation: string }>("fulfill_preview", fulfill);
      await command("fulfill_post", { ...fulfill, expected_generation: review._generation });
      const invoiceId = `${key}-invoice`;
      const open = await command<{ items: { id: string; remaining: string }[] }>("payment_options", {
        company_id: b.companyId,
        side: "receive",
        party_id: customer,
      });
      const owed = BigInt(open.items.find((item) => item.id === invoiceId)?.remaining ?? "0");
      const pay = async (amount: bigint, funds: "cash" | "bank") => {
        if (amount <= 0n) return 0n;
        const part = amount > owed ? owed : amount;
        const paymentKey = uid();
        const body = {
          company_id: b.companyId,
          period_id: b.periodId,
          source_id: paymentKey,
          request_key: paymentKey,
          date: today(),
          reason: "Till receipt",
          side: "receive",
          party_id: customer,
          funds_role: funds,
          reference: `POS:${shift.id}:${invoice}`,
          advance_amount: "0",
          allocations: [{ open_item_id: invoiceId, amount: String(part) }],
        };
        const paymentReview = await command<{ _generation: string }>("payment_preview", body);
        await command("payment_post", { ...body, expected_generation: paymentReview._generation });
        return part;
      };
      let paid = 0n;
      if (method === "cash") paid = await pay(owed, "cash");
      else if (method === "bank") paid = await pay(owed, "bank");
      else if (method === "split") {
        paid += await pay(toIrr(cashPart || "0", b.unit), "cash");
        const left = owed - paid;
        const bank = toIrr(bankPart || "0", b.unit);
        paid += await pay(bank > left ? left : bank, "bank");
      }
      setReceipt({
        number: invoice,
        total: String(owed),
        paid: String(paid),
        method,
        lines: lines.map((line) => ({ ...line })),
      });
      setLines([]);
      await b.refresh();
      toast.success(t("Sale recorded"));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      lock.current = false;
      setBusy(false);
    }
  }

  const unitLabel = b.unit === "Toman" ? t("Toman (10 rials)") : t("Rial");

  return (
    <div className="flex flex-col gap-4">
      <PageHeader title={t("Point of sale")} />
      {!online && (
        <p role="status" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm">
          {t("You are offline. Checkout is paused so a sale cannot look successful without being saved.")}
        </p>
      )}
      <p className="text-sm text-muted-foreground">
        {t("Negative stock is rejected. Card and bank amounts here are manual records, not a live collection.")}{" "}
        {unitLabel}
      </p>
      <section className="grid gap-3 rounded-xl border border-border bg-card p-4 md:grid-cols-[1fr_auto]">
        {shift ? (
          <>
            <div className="text-sm">
              <div className="font-medium">{t("Till open")}</div>
              <div dir="ltr">
                {shift.cashier} · {t("Opening")} <Money value={shift.opening_cash_rial} />
              </div>
            </div>
            <div className="flex flex-wrap items-end gap-2">
              <label className="text-sm">
                {t("Counted cash")}
                <Input inputMode="decimal" value={counted} onChange={(e) => setCounted(e.target.value)} />
              </label>
              <Button type="button" variant="outline" disabled={busy} onClick={() => void closeTill()}>
                {t("Close till")}
              </Button>
            </div>
          </>
        ) : (
          <>
            <p className="text-sm">{t("Open a till before checkout. Opening cash is the float you counted, not new revenue.")}</p>
            <div className="flex flex-wrap items-end gap-2">
              <label className="text-sm">
                {t("Cashier")}
                <Input value={cashier} onChange={(e) => setCashier(e.target.value)} />
              </label>
              <label className="text-sm">
                {t("Opening cash")}
                <Input inputMode="decimal" value={opening} onChange={(e) => setOpening(e.target.value)} />
              </label>
              <Button type="button" disabled={busy} onClick={() => void openTill()}>
                {t("Open till")}
              </Button>
            </div>
          </>
        )}
      </section>
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <section className="flex flex-col gap-3">
          <label className="text-sm font-medium" htmlFor="pos-scan">
            {t("Scan or search")}
          </label>
          <Input
            id="pos-scan"
            ref={scan}
            autoFocus
            value={query}
            placeholder={t("Barcode, SKU, or name")}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onScan}
            autoComplete="off"
          />
          <div className="grid grid-cols-3 gap-2" role="group" aria-label={t("Keypad")} dir="ltr">
            {["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"].map((digit) => (
              <Button
                key={digit}
                type="button"
                variant="outline"
                className="h-12 text-lg tabular-nums"
                aria-label={t("Digit {digit}", { digit })}
                onClick={() => {
                  setQuery((current) => `${current}${digit}`);
                  scan.current?.focus();
                }}
              >
                {digit}
              </Button>
            ))}
            <Button
              type="button"
              variant="outline"
              className="h-12"
              onClick={() => setQuery((current) => current.slice(0, -1))}
            >
              {t("Backspace")}
            </Button>
            <Button type="button" variant="outline" className="h-12" onClick={() => setQuery("")}>
              {t("Clear code")}
            </Button>
          </div>
          {catalog.length === 0 && (
            <p className="text-sm text-muted-foreground">{t("No matching product")}</p>
          )}
          <ul className="grid gap-2 sm:grid-cols-2">
            {catalog.map(({ item, stock }) => (
              <li key={item.id}>
                <button
                  type="button"
                  className="flex min-h-14 w-full flex-col justify-center rounded-lg border border-border px-3 py-3 text-start hover:bg-accent focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                  onClick={() => addItem(item.id, item.name)}
                >
                  <span className="font-medium">{item.name}</span>
                  <span className="text-xs text-muted-foreground" dir="ltr">
                    {item.sku} · {formatQuantity(String(stock))}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </section>
        <aside className="flex flex-col gap-3 rounded-xl border border-border bg-card p-4">
          <h2 className="font-medium">{t("Cart")}</h2>
          {lines.length === 0 && <p className="text-sm text-muted-foreground">{t("No records yet.")}</p>}
          <ul className="flex flex-col gap-2">
            {lines.map((line) => (
              <li key={line.id} className="grid grid-cols-1 items-center gap-2 text-sm sm:grid-cols-[minmax(0,1fr)_auto_5.5rem_auto]">
                <span className="truncate">{line.name}</span>
                <div className="flex items-center gap-1">
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className="h-11 w-11"
                    aria-label={t("Decrease quantity")}
                    onClick={() =>
                      setLines((current) =>
                        current.map((row) => {
                          if (row.id !== line.id) return row;
                          const next = (toThousandths(row.qty) ?? 0n) - 1000n;
                          return { ...row, qty: qtyText(next > 0n ? next : 0n) };
                        }),
                      )
                    }
                  >
                    −
                  </Button>
                  <Input
                  aria-label={t("Quantity")}
                  inputMode="decimal"
                  value={line.qty}
                  onChange={(e) =>
                    setLines((current) =>
                      current.map((row) => (row.id === line.id ? { ...row, qty: e.target.value } : row)),
                    )
                  }
                />
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    className="h-11 w-11"
                    aria-label={t("Increase quantity")}
                    onClick={() =>
                      setLines((current) =>
                        current.map((row) =>
                          row.id === line.id
                            ? { ...row, qty: qtyText((toThousandths(row.qty) ?? 0n) + 1000n) }
                            : row,
                        ),
                      )
                    }
                  >
                    +
                  </Button>
                </div>
                <Input
                  aria-label={t("Price")}
                  inputMode="decimal"
                  value={line.price}
                  onChange={(e) =>
                    setLines((current) =>
                      current.map((row) => (row.id === line.id ? { ...row, price: e.target.value } : row)),
                    )
                  }
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  aria-label={t("Remove")}
                  onClick={() => setLines((current) => current.filter((row) => row.id !== line.id))}
                >
                  ×
                </Button>
              </li>
            ))}
          </ul>
          <label className="text-sm">
            {t("Customer")}
            <select
              className="mt-1 h-9 w-full rounded-md border border-input bg-background px-2"
              value={customerId}
              onChange={(e) => setCustomerId(e.target.value)}
            >
              <option value="">{t("Walk-in customer")}</option>
              {(resources?.parties ?? [])
                .filter((party) => party.customer)
                .map((party) => (
                  <option key={party.id} value={party.id}>
                    {party.name}
                  </option>
                ))}
            </select>
          </label>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2" role="group" aria-label={t("How is the customer paying?")}>
            {(
              [
                ["cash", "Cash recorded in the till. This is not a card payment."],
                ["bank", "Bank receipt recorded. Sarv did not contact a bank or a card terminal."],
                ["account", "Left on the customer account. Nothing was charged to a card."],
                ["split", "Split the receipt between cash and bank. Both are manual records."],
              ] as const
            ).map(([id, label]) => (
              <Button
                key={id}
                type="button"
                variant={method === id ? "default" : "outline"}
                className="h-auto whitespace-normal px-2 py-2 text-start text-xs"
                onClick={() => setMethod(id)}
              >
                {t(label)}
              </Button>
            ))}
          </div>
          {method === "split" && (
            <div className="grid grid-cols-2 gap-2">
              <label className="text-sm">
                {t("Cash")}
                <Input inputMode="decimal" value={cashPart} onChange={(e) => setCashPart(e.target.value)} />
              </label>
              <label className="text-sm">
                {t("Bank")}
                <Input inputMode="decimal" value={bankPart} onChange={(e) => setBankPart(e.target.value)} />
              </label>
            </div>
          )}
          {error && <ErrorState error={error} />}
          <div className="flex gap-2">
            <Button type="button" variant="outline" disabled={!lines.length || busy} onClick={() => void hold()}>
              {t("Hold sale")}
            </Button>
            <Button type="button" className="h-12 flex-1 text-base" disabled={busy || !online} onClick={() => void checkout()}>
              {t("Complete sale")}
            </Button>
          </div>
          {(retail?.held.length ?? 0) > 0 && (
            <ul className="flex flex-col gap-1 text-sm">
              {retail?.held.map((held) => (
                <li key={held.id}>
                  <button type="button" className="underline" onClick={() => resume(held)}>
                    {t("Resume")}: {held.label}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </aside>
      </div>
      {receipt && (
        <section
          className="pos-receipt rounded-xl border border-border p-4 text-sm"
          dir={b.locale === "fa" ? "rtl" : "ltr"}
        >
          <h2 className="font-semibold">{t("Receipt")}</h2>
          <p dir="ltr">{receipt.number}</p>
          <ul className="flex flex-col gap-1">
            {receipt.lines.map((line) => (
              <li key={line.id} className="flex justify-between gap-3">
                <span>
                  {line.name} <span dir="ltr">× {line.qty}</span>
                </span>
                <span dir="ltr">{line.price}</span>
              </li>
            ))}
          </ul>
          <p>
            {t("Total")} <Money value={receipt.total} />
          </p>
          <p>
            {t("Paid")} <Money value={receipt.paid} />
          </p>
          <p>{t("Manual payment")}</p>
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              document.body.classList.add("pos-printing");
              window.print();
              document.body.classList.remove("pos-printing");
            }}
          >
            {t("Print")}
          </Button>
        </section>
      )}
    </div>
  );
}

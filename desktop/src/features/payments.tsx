import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type {
  PaymentOpenItem,
  PaymentPreviewData,
  PaymentRecordEntry,
  PaymentSide,
} from "../lib/types";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { FieldGroup } from "../components/ui/field";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import {
  PageHeader,
  Field,
  ErrorState,
  Loading,
  Note,
} from "../components/ui/shared";
import { DatePicker } from "../components/ui/date-picker";
import { DataGrid } from "../components/ui/data-grid";
import { Money, formatDate } from "../components/finance/money";
import { Modal } from "../components/layout/modal";

export function Payments({ onJournal }: { onJournal: (id: string) => void }) {
  const b = useBooks(),
    t = b.t;
  const [items, setItems] = useState<Record<PaymentSide, PaymentOpenItem[]> | null>(
      null,
    ),
    [error, setError] = useState("");
  const [dialog, setDialog] = useState<{
    side: PaymentSide;
    partyId?: string;
    openItemId?: string;
  } | null>(null);
  const [reverse, setReverse] = useState<PaymentRecordEntry | null>(null);
  const load = useCallback(async () => {
    const [receive, make] = await Promise.all([
      command<{ items: PaymentOpenItem[] }>("payment_options", {
        company_id: b.companyId,
        side: "receive",
      }),
      command<{ items: PaymentOpenItem[] }>("payment_options", {
        company_id: b.companyId,
        side: "make",
      }),
    ]);
    setItems({ receive: receive.items, make: make.items });
    setError("");
  }, [b.companyId]);
  useEffect(() => {
    let active = true;
    load().catch((e) => {
      if (active) setError((e as Error).message);
    });
    return () => {
      active = false;
    };
  }, [load, b.workspace.journal_count]);
  const money = (v: string) => (
    <Money value={v} unit={b.unit} locale={b.numberLocale} />
  );
  const parties = b.workspace.resources?.parties ?? [];
  const partyName = (id: string) =>
    parties.find((p) => p.id === id)?.name ?? id;
  const payments = b.workspace.payments ?? [];
  const receivableTotal = useMemo(
    () =>
      (items?.receive ?? []).reduce((sum, i) => sum + BigInt(i.remaining), 0n),
    [items],
  );
  const payableTotal = useMemo(
    () => (items?.make ?? []).reduce((sum, i) => sum + BigInt(i.remaining), 0n),
    [items],
  );
  if (!b.workspace.initialized)
    return (
      <>
        <PageHeader title={t("Payments")} />
        <Note>{t("Confirm starting balances to settle invoices and bills.")}</Note>
        <Button onClick={b.openSetup}>{t("Record opening balances")}</Button>
      </>
    );
  return (
    <>
      <PageHeader
        title={t("Payments")}
        actions={
          <div className="toolbar-actions">
            <Button
              disabled={!items}
              onClick={() => setDialog({ side: "receive" })}
            >
              {t("Receive money")}
            </Button>
            <Button
              variant="outline"
              disabled={!items}
              onClick={() => setDialog({ side: "make" })}
            >
              {t("Pay bill")}
            </Button>
          </div>
        }
      />
      {error && (
        <ErrorState error={error} retry={() => void load().catch((e) => setError(e.message))} />
      )}
      {!items ? (
        <Loading />
      ) : (
        <>
          <div className="overview-metrics finance-metrics">
            <section className="metric">
              <div className="metric-label">{t("Receivables outstanding")}</div>
              <div className="metric-value">{money(receivableTotal.toString())}</div>
              <small>{items.receive.length} {t("open invoices")}</small>
            </section>
            <section className="metric">
              <div className="metric-label">{t("Payables outstanding")}</div>
              <div className="metric-value">{money(payableTotal.toString())}</div>
              <small>{items.make.length} {t("open bills")}</small>
            </section>
          </div>
          <h2>{t("Customer receivables")}</h2>
          <DataGrid
            data={items.receive}
            id={(i) => i.id}
            empty={t("No open customer invoices")}
            searchText={(i) => partyName(i.party_id) + " " + i.reference}
            columns={[
              {
                header: t("Customer"),
                cell: ({ row }) => partyName(row.original.party_id),
              },
              { header: t("Reference"), cell: ({ row }) => row.original.reference },
              {
                header: t("Document date"),
                cell: ({ row }) =>
                  formatDate(
                    row.original.document_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("Due date"),
                cell: ({ row }) =>
                  formatDate(
                    row.original.due_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("Remaining"),
                meta: { numeric: true },
                cell: ({ row }) => money(row.original.remaining),
              },
              {
                header: "",
                cell: ({ row }) => (
                  <Button
                    variant="outline"
                    onClick={() =>
                      setDialog({
                        side: "receive",
                        partyId: row.original.party_id,
                        openItemId: row.original.id,
                      })
                    }
                  >
                    {t("Settle")}
                  </Button>
                ),
              },
            ]}
          />
          <h2>{t("Supplier payables")}</h2>
          <DataGrid
            data={items.make}
            id={(i) => i.id}
            empty={t("No open supplier bills")}
            searchText={(i) => partyName(i.party_id) + " " + i.reference}
            columns={[
              {
                header: t("Vendor"),
                cell: ({ row }) => partyName(row.original.party_id),
              },
              { header: t("Reference"), cell: ({ row }) => row.original.reference },
              {
                header: t("Document date"),
                cell: ({ row }) =>
                  formatDate(
                    row.original.document_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("Due date"),
                cell: ({ row }) =>
                  formatDate(
                    row.original.due_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("Remaining"),
                meta: { numeric: true },
                cell: ({ row }) => money(row.original.remaining),
              },
              {
                header: "",
                cell: ({ row }) => (
                  <Button
                    variant="outline"
                    onClick={() =>
                      setDialog({
                        side: "make",
                        partyId: row.original.party_id,
                        openItemId: row.original.id,
                      })
                    }
                  >
                    {t("Settle")}
                  </Button>
                ),
              },
            ]}
          />
          <h2>{t("Payment history")}</h2>
          <DataGrid
            data={payments}
            id={(p) => p.id}
            empty={t("No payments recorded yet")}
            searchText={(p) => JSON.stringify(p.data)}
            columns={[
              {
                header: t("Date"),
                cell: ({ row }) => {
                  const ctx = Object.values(row.original.command)[0]?.context;
                  return ctx
                    ? formatDate(
                        ctx.date,
                        b.dateLocale,
                        b.workspace.settings.date_display,
                      )
                    : "";
                },
              },
              {
                header: t("Type"),
                cell: ({ row }) => {
                  const key = Object.keys(row.original.data)[0];
                  const data = Object.values(row.original.data)[0];
                  if (key === "Reversal") return t("Reversal");
                  return data?.side === "Make" ? t("Pay bill") : t("Receive money");
                },
              },
              {
                header: t("Party"),
                cell: ({ row }) => {
                  const data = Object.values(row.original.data)[0];
                  const id =
                    data?.party?.id ??
                    Object.values(row.original.command)[0]?.party_id ??
                    "";
                  return id ? partyName(id) : "";
                },
              },
              {
                header: t("Reference"),
                cell: ({ row }) =>
                  Object.values(row.original.data)[0]?.reference ?? "",
              },
              {
                header: t("Amount"),
                cell: ({ row }) => {
                  const v = Object.values(row.original.data)[0]?.total;
                  return v ? money(v) : "";
                },
              },
              {
                header: t("Journal"),
                cell: ({ row }) => (
                  <Button
                    variant="link"
                    onClick={() => onJournal(row.original.journal_id)}
                  >
                    {t("View")}
                  </Button>
                ),
              },
              {
                header: "",
                cell: ({ row }) => {
                  const key = Object.keys(row.original.data)[0];
                  if (key !== "Payment") return null;
                  const reversed = payments.some((p) => {
                    const k = Object.keys(p.data)[0];
                    return (
                      k === "Reversal" &&
                      Object.values(p.data)[0]?.payment_id === row.original.id
                    );
                  });
                  return reversed ? (
                    <span>{t("Reversed")}</span>
                  ) : (
                    <Button
                      variant="outline"
                      onClick={() => setReverse(row.original)}
                    >
                      {t("Reverse")}
                    </Button>
                  );
                },
              },
            ]}
          />
        </>
      )}
      {dialog && (
        <PaymentDialog
          side={dialog.side}
          presetPartyId={dialog.partyId}
          presetOpenItemId={dialog.openItemId}
          onClose={() => setDialog(null)}
          onPosted={async () => {
            await b.refresh();
            await load();
            setDialog(null);
          }}
        />
      )}
      {reverse && (
        <ReverseDialog
          payment={reverse}
          onClose={() => setReverse(null)}
          onPosted={async () => {
            await b.refresh();
            await load();
            setReverse(null);
          }}
        />
      )}
    </>
  );
}

export function PaymentDialog({
  side,
  presetPartyId,
  presetOpenItemId,
  onClose,
  onPosted,
}: {
  side: PaymentSide;
  presetPartyId?: string;
  presetOpenItemId?: string;
  onClose: () => void;
  onPosted: () => Promise<void>;
}) {
  const b = useBooks(),
    t = b.t;
  const [key] = useState(uid),
    [date, setDate] = useState(today),
    [partyId, setPartyId] = useState(presetPartyId ?? ""),
    [fundsRole, setFundsRole] = useState<"cash" | "bank">("bank"),
    [reference, setReference] = useState(""),
    [reason, setReason] = useState(""),
    [advance, setAdvance] = useState(""),
    [amounts, setAmounts] = useState<Record<string, string>>(() =>
      presetOpenItemId ? { [presetOpenItemId]: "" } : {},
    );
  const [items, setItems] = useState<PaymentOpenItem[] | null>(null),
    [review, setReview] = useState<(PaymentPreviewData & { _generation: string }) | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [discard, setDiscard] = useState(false);
  const parties = (b.workspace.resources?.parties ?? []).filter((p) =>
    side === "receive" ? p.customer : p.supplier,
  );
  useEffect(() => {
    if (!partyId) {
      setItems(null);
      return;
    }
    let active = true;
    command<{ items: PaymentOpenItem[] }>("payment_options", {
      company_id: b.companyId,
      side,
      party_id: partyId,
    })
      .then((r) => {
        if (active) setItems(r.items);
      })
      .catch((e) => {
        if (active) setError((e as Error).message);
      });
    return () => {
      active = false;
    };
  }, [partyId, side, b.companyId]);
  const rows = (items ?? []).filter((i) => i.remaining !== "0");
  const allocated = rows.reduce(
    (sum, i) => sum + (BigInt(amounts[i.id] ?? "0") || 0n),
    0n,
  );
  const advanceValue = BigInt(advance || "0") || 0n;
  const total = allocated + advanceValue;
  const payload = {
    company_id: b.companyId,
    period_id: b.periodId,
    source_id: key,
    request_key: key,
    date,
    reason: reason || t("Invoice settlement"),
    side,
    party_id: partyId,
    funds_role: fundsRole,
    reference,
    advance_amount: advance || "0",
    allocations: rows
      .filter((i) => BigInt(amounts[i.id] ?? "0") > 0n)
      .map((i) => ({ open_item_id: i.id, amount: amounts[i.id] })),
  };
  async function act(post = false) {
    if (busy || (post && !review)) return;
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command("payment_post", {
          ...payload,
          expected_generation: review?._generation,
        });
        await onPosted();
        toast.success(t("Payment recorded"));
      } else {
        setReview(
          await command<PaymentPreviewData & { _generation: string }>(
            "payment_preview",
            payload,
          ),
        );
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const close = () => {
    if (busy) return;
    if (reference || advance || Object.keys(amounts).length) setDiscard(true);
    else onClose();
  };
  const accountName = (id: string) =>
    b.workspace.accounts.find((a) => a.id === id)?.name ?? id;
  return (
    <>
      <Modal
        open
        onOpenChange={(v) => {
          if (!v) close();
        }}
        title={t(side === "receive" ? "Receive money" : "Pay bill")}
        wide
        description={t(
          "Allocate the exact amount to open documents; any surplus becomes an advance.",
        )}
      >
        {error && <ErrorState error={error} />}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void act();
          }}
          onChange={() => setReview(null)}
        >
          <FieldGroup className="form-grid">
            <Field label={t(side === "receive" ? "Customer" : "Vendor")}>
              <NativeSelect
                required
                value={partyId}
                onChange={(e) => {
                  setPartyId(e.target.value);
                  setAmounts({});
                  setReview(null);
                }}
              >
                <NativeSelectOption value="">
                  {t("Select contact")}
                </NativeSelectOption>
                {parties.map((p) => (
                  <NativeSelectOption key={p.id} value={p.id}>
                    {p.name}
                  </NativeSelectOption>
                ))}
              </NativeSelect>
            </Field>
            <Field label={t("Cash / bank account")}>
              <NativeSelect
                value={fundsRole}
                onChange={(e) => {
                  setFundsRole(e.target.value as "cash" | "bank");
                  setReview(null);
                }}
              >
                <NativeSelectOption value="bank">{t("Bank")}</NativeSelectOption>
                <NativeSelectOption value="cash">{t("Cash")}</NativeSelectOption>
              </NativeSelect>
            </Field>
            <Field label={t("Financial date")}>
              <DatePicker
                value={date}
                onChange={(v) => {
                  setDate(v);
                  setReview(null);
                }}
              />
            </Field>
            <Field label={t("Reference")}>
              <Input
                required
                maxLength={160}
                value={reference}
                onChange={(e) => setReference(e.target.value)}
                placeholder={t("Receipt or transfer number")}
              />
            </Field>
            <Field label={t("Description")}>
              <Input
                maxLength={160}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
                placeholder={t("Invoice settlement")}
              />
            </Field>
            <Field label={t("Unallocated advance · IRR")}>
              <Input
                inputMode="decimal"
                value={advance}
                onChange={(e) => setAdvance(e.target.value)}
              />
            </Field>
          </FieldGroup>
          {partyId && (
            <>
              <h3>{t("Open documents")}</h3>
              {!items ? (
                <Loading />
              ) : (
                <DataGrid
                  data={rows}
                  id={(i) => i.id}
                  empty={t("No open documents for this contact")}
                  columns={[
                    {
                      header: t("Reference"),
                      cell: ({ row }) => row.original.reference,
                    },
                    {
                      header: t("Due date"),
                      cell: ({ row }) =>
                        formatDate(
                          row.original.due_date,
                          b.dateLocale,
                          b.workspace.settings.date_display,
                        ),
                    },
                    {
                      header: t("Remaining"),
                      cell: ({ row }) => row.original.remaining,
                    },
                    {
                      header: t("Allocate · IRR"),
                      cell: ({ row }) => (
                        <Input
                          inputMode="decimal"
                          value={amounts[row.original.id] ?? ""}
                          max={row.original.remaining}
                          onChange={(e) =>
                            setAmounts({
                              ...amounts,
                              [row.original.id]: e.target.value,
                            })
                          }
                        />
                      ),
                    },
                    {
                      header: "",
                      cell: ({ row }) => (
                        <Button
                          type="button"
                          variant="ghost"
                          onClick={() =>
                            setAmounts({
                              ...amounts,
                              [row.original.id]: row.original.remaining,
                            })
                          }
                        >
                          {t("Full")}
                        </Button>
                      ),
                    },
                  ]}
                />
              )}
            </>
          )}
          <div className="form-actions">
            <span className="total-row">
              {t("Allocated")}:{" "}
              <Money value={allocated.toString()} unit={b.unit} locale={b.numberLocale} />
              {" · "}
              {t("Total")}:{" "}
              <Money value={total.toString()} unit={b.unit} locale={b.numberLocale} />
            </span>
            <Button type="button" variant="outline" disabled={busy} onClick={close}>
              {t("Cancel")}
            </Button>
            <Button
              type="submit"
              disabled={busy || !partyId || total <= 0n || !reference.trim()}
            >
              {t("Review payment")}
            </Button>
            <Button
              type="button"
              disabled={busy || !review}
              onClick={() => void act(true)}
            >
              {t("Confirm and post")}
            </Button>
          </div>
        </form>
        {review && (
          <>
            <h3>{t("Financial effect")}</h3>
            <DataGrid
              data={review.journal_lines}
              id={(l) => l.account_id + l.debit + l.credit}
              empty=""
              columns={[
                {
                  header: t("Account"),
                  cell: ({ row }) => accountName(row.original.account_id),
                },
                {
                  header: t("Debit · IRR"),
                  cell: ({ row }) => <Money value={row.original.debit} />,
                },
                {
                  header: t("Credit · IRR"),
                  cell: ({ row }) => <Money value={row.original.credit} />,
                },
              ]}
            />
          </>
        )}
      </Modal>
      <Modal
        open={discard}
        onOpenChange={setDiscard}
        title={t("Leave unsaved changes?")}
        description={t("Discard unsaved payment?")}
        confirmation
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

function ReverseDialog({
  payment,
  onClose,
  onPosted,
}: {
  payment: PaymentRecordEntry;
  onClose: () => void;
  onPosted: () => Promise<void>;
}) {
  const b = useBooks(),
    t = b.t;
  const [key] = useState(uid),
    [date, setDate] = useState(today),
    [reason, setReason] = useState("");
  const [review, setReview] = useState<(PaymentPreviewData & { _generation: string }) | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const payload = {
    company_id: b.companyId,
    period_id: b.periodId,
    source_id: key,
    request_key: key,
    date,
    reason: reason || t("Payment reversal"),
    payment_id: payment.id,
  };
  async function act(post = false) {
    if (busy || (post && !review)) return;
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command("payment_post", {
          ...payload,
          expected_generation: review?._generation,
        });
        await onPosted();
        toast.success(t("Payment reversed"));
      } else {
        setReview(
          await command<PaymentPreviewData & { _generation: string }>(
            "payment_reverse_preview",
            { company_id: b.companyId, payment_id: payment.id },
          ),
        );
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const data = Object.values(payment.data)[0];
  const accountName = (id: string) =>
    b.workspace.accounts.find((a) => a.id === id)?.name ?? id;
  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title={t("Reverse payment")}
      wide
      description={t(
        "The mirrored journal restores open documents and returns the money to cash/bank.",
      )}
    >
      {error && <ErrorState error={error} />}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void act();
        }}
        onChange={() => setReview(null)}
      >
        <Note>
          {data?.reference} · <Money value={data?.total ?? "0"} unit={b.unit} locale={b.numberLocale} />
        </Note>
        <FieldGroup className="form-grid">
          <Field label={t("Financial date")}>
            <DatePicker
              value={date}
              onChange={(v) => {
                setDate(v);
                setReview(null);
              }}
            />
          </Field>
          <Field label={t("Description")}>
            <Input
              maxLength={160}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder={t("Payment reversal")}
            />
          </Field>
        </FieldGroup>
        <div className="form-actions">
          <Button type="button" variant="outline" disabled={busy} onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button type="submit" disabled={busy}>
            {t("Review reversal")}
          </Button>
          <Button type="button" disabled={busy || !review} onClick={() => void act(true)}>
            {t("Confirm and post")}
          </Button>
        </div>
      </form>
      {review && (
        <>
          <h3>{t("Financial effect")}</h3>
          <DataGrid
            data={review.journal_lines}
            id={(l) => l.account_id + l.debit + l.credit}
            empty=""
            columns={[
              {
                header: t("Account"),
                cell: ({ row }) => accountName(row.original.account_id),
              },
              {
                header: t("Debit · IRR"),
                cell: ({ row }) => <Money value={row.original.debit} />,
              },
              {
                header: t("Credit · IRR"),
                cell: ({ row }) => <Money value={row.original.credit} />,
              },
            ]}
          />
        </>
      )}
    </Modal>
  );
}

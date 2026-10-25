import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { BankingData, Journal } from "../lib/types";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Textarea } from "../components/ui/textarea";
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
  DateRange,
} from "../components/ui/shared";
import { DatePicker } from "../components/ui/date-picker";
import { DataGrid } from "../components/ui/data-grid";
import { Money, formatDate } from "../components/finance/money";
import { Modal } from "../components/layout/modal";
export type MovementKind = "deposit" | "withdrawal" | "transfer";

export function Banking({ onJournal }: { onJournal: (id: string) => void }) {
  const b = useBooks(),
    t = b.t;
  const [data, setData] = useState<BankingData | null>(null),
    [error, setError] = useState("");
  const [offset, setOffset] = useState(0),
    [kind, setKind] = useState<MovementKind | null>(null);
  const [reconcile, setReconcile] = useState<BankingData["accounts"][number] | null>(
    null,
  );
  const load = useCallback(async () => {
    const result = await command<BankingData>("banking", {
      company_id: b.companyId,
      from: b.from,
      to: b.to,
      offset: String(offset),
    });
    setData(result);
    setError("");
  }, [b.companyId, b.from, b.to, offset]);
  useEffect(() => {
    let active = true;
    command<BankingData>("banking", {
      company_id: b.companyId,
      from: b.from,
      to: b.to,
      offset: String(offset),
    })
      .then((d) => {
        if (active) {
          setData(d);
          setError("");
        }
      })
      .catch((e) => {
        if (active) setError(e.message);
      });
    return () => {
      active = false;
    };
  }, [b.companyId, b.from, b.to, offset, b.workspace.journal_count]);
  const money = (v: string) => (
    <Money value={v} unit={b.unit} locale={b.numberLocale} />
  );
  if (!b.workspace.initialized)
    return (
      <>
        <PageHeader title={t("Banking")} />
        <Note>{t("Confirm starting balances to use cash and banking.")}</Note>
        <Button onClick={b.openSetup}>{t("Record opening balances")}</Button>
      </>
    );
  return (
    <>
      <PageHeader
        title={t("Banking")}
        actions={
          <div className="toolbar-actions">
            <Button disabled={!data} onClick={() => setKind("deposit")}>
              {t("Money in")}
            </Button>
            <Button
              variant="outline"
              disabled={!data}
              onClick={() => setKind("withdrawal")}
            >
              {t("Money out")}
            </Button>
            <Button
              variant="outline"
              disabled={!data}
              onClick={() => setKind("transfer")}
            >
              {t("Transfer money")}
            </Button>
          </div>
        }
      />
      <DateRange
        from={b.from}
        to={b.to}
        onChange={(from, to) => {
          setOffset(0);
          b.setRange(from, to);
        }}
      />
      {error && (
        <ErrorState
          error={error}
          retry={() => void load().catch((e) => setError(e.message))}
        />
      )}
      {!data ? (
        <Loading />
      ) : (
        <>
          <div className="stat-grid">
            {data.accounts.map((a) => (
              <section className="stat-card" key={a.account.id}>
                <div className="stat-card-head">
                  <span>{t(a.role === "Cash" ? "Cash" : "Bank")}</span>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setReconcile(a)}
                    aria-label={t("Reconcile account")}
                  >
                    {t("Reconcile")}
                  </Button>
                </div>
                <div className="stat-card-value">
                  {money(a.balance)}
                  <small>{b.unit === "Rial" ? "IRR" : t("toman")}</small>
                </div>
                <p dir="auto">{a.account.name}</p>
              </section>
            ))}
          </div>
          <h2>{t("Cash and bank statement")}</h2>
          <DataGrid
            data={data.accounts}
            id={(a) => a.account.id}
            empty={t("No accounts")}
            columns={[
              {
                header: t("Account"),
                cell: ({ row }) => row.original.account.name,
              },
              {
                header: t("Opening balance"),
                meta: { numeric: true },
                cell: ({ row }) => money(row.original.opening),
              },
              {
                header: t("Money in"),
                meta: { numeric: true },
                cell: ({ row }) => money(row.original.incoming),
              },
              {
                header: t("Money out"),
                meta: { numeric: true },
                cell: ({ row }) => money(row.original.outgoing),
              },
              {
                header: t("Closing balance"),
                meta: { numeric: true },
                cell: ({ row }) => money(row.original.balance),
              },
            ]}
          />
          <h2>{t("Movement history")}</h2>
          <DataGrid
            data={data.movements}
            id={(j) => j.id}
            searchText={(j) =>
              j.reason + " " + j.lines.map((l) => l.description).join(" ")
            }
            empty={t("No cash or bank movements in this period")}
            columns={[
              {
                header: t("Date"),
                cell: ({ row }) =>
                  formatDate(
                    row.original.date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("Reference"),
                cell: ({ row }) => (
                  <Button
                    variant="link"
                    onClick={() => onJournal(row.original.id)}
                  >
                    {row.original.lines[0]?.description ||
                      row.original.source_id}
                  </Button>
                ),
              },
              { header: t("Description"), accessorKey: "reason" },
              {
                header: t("Money in"),
                cell: ({ row }) =>
                  money(
                    row.original.lines
                      .filter((l) =>
                        data.accounts.some(
                          (a) => a.account.id === l.account.id,
                        ),
                      )
                      .reduce((sum, l) => sum + BigInt(l.debit), 0n)
                      .toString(),
                  ),
              },
              {
                header: t("Money out"),
                cell: ({ row }) =>
                  money(
                    row.original.lines
                      .filter((l) =>
                        data.accounts.some(
                          (a) => a.account.id === l.account.id,
                        ),
                      )
                      .reduce((sum, l) => sum + BigInt(l.credit), 0n)
                      .toString(),
                  ),
              },
            ]}
          />
          <div className="form-actions">
            <Button
              variant="outline"
              disabled={!offset}
              onClick={() => setOffset(Math.max(0, offset - 100))}
            >
              {t("Previous")}
            </Button>
            <Button
              variant="outline"
              disabled={offset + 100 >= Number(data.movement_count)}
              onClick={() => setOffset(offset + 100)}
            >
              {t("Next")}
            </Button>
          </div>
          {kind && (
            <MovementDialog
              kind={kind}
              data={data}
              onClose={() => setKind(null)}
              onPosted={async () => {
                await b.refresh();
                await load();
                setKind(null);
              }}
            />
          )}
          {reconcile && (
            <ReconcileDialog
              account={reconcile}
              data={data}
              onClose={() => setReconcile(null)}
            />
          )}
        </>
      )}
    </>
  );
}

function ReconcileDialog({
  account,
  data,
  onClose,
}: {
  account: BankingData["accounts"][number];
  data: BankingData;
  onClose: () => void;
}) {
  const b = useBooks(),
    t = b.t;
  const [statement, setStatement] = useState("");
  const [checked, setChecked] = useState<Record<string, boolean>>({});
  const movements = data.movements.filter((j) =>
    j.lines.some((l) => l.account.id === account.account.id),
  );
  const signed = (j: Journal) =>
    j.lines
      .filter((l) => l.account.id === account.account.id)
      .reduce((sum, l) => sum + BigInt(l.debit) - BigInt(l.credit), 0n);
  const book = BigInt(account.balance);
  const entered = statement.trim() ? BigInt(statement) : null;
  const difference = entered === null ? 0n : entered - book;
  const marked = movements
    .filter((j) => checked[j.id])
    .reduce((sum, j) => sum + signed(j), 0n);
  const unmarked = movements
    .filter((j) => !checked[j.id])
    .reduce((sum, j) => sum + signed(j), 0n);
  const money = (v: bigint) => (
    <Money value={v.toString()} unit={b.unit} locale={b.numberLocale} />
  );
  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v) onClose();
      }}
      title={t("Reconcile account")}
      wide
      description={t(
        "Compare your bank statement with the book balance and mark the entries you found on the statement.",
      )}
    >
      <Note>
        {account.account.name} · {t("Through")}{" "}
        {formatDate(b.to, b.dateLocale, b.workspace.settings.date_display)}
      </Note>
      <FieldGroup className="form-grid">
        <Field label={t("Book balance")}>
          <Input value={account.balance} readOnly />
        </Field>
        <Field label={t("Statement closing balance · IRR")}>
          <Input
            inputMode="decimal"
            value={statement}
            onChange={(e) => setStatement(e.target.value)}
            placeholder={account.balance}
          />
        </Field>
      </FieldGroup>
      {(statement.trim() !== "") && (
        <div className="overview-metrics finance-metrics">
          <section className="metric">
            <div className="metric-label">{t("Difference")}</div>
            <div className="metric-value">{money(difference)}</div>
          </section>
          <section className="metric">
            <div className="metric-label">{t("Unmarked entries")}</div>
            <div className="metric-value">{money(unmarked)}</div>
            <small>{t("Explains the difference when entries are missing")}</small>
          </section>
          <section className="metric">
            <div className="metric-label">{t("Marked on statement")}</div>
            <div className="metric-value">{money(marked)}</div>
          </section>
        </div>
      )}
      <h3>{t("Mark entries found on the statement")}</h3>
      <DataGrid
        data={movements}
        id={(j) => j.id}
        empty={t("No cash or bank movements in this period")}
        columns={[
          {
            header: "",
            cell: ({ row }) => (
              <input
                type="checkbox"
                checked={!!checked[row.original.id]}
                onChange={(e) =>
                  setChecked({
                    ...checked,
                    [row.original.id]: e.target.checked,
                  })
                }
              />
            ),
          },
          {
            header: t("Date"),
            cell: ({ row }) =>
              formatDate(
                row.original.date,
                b.dateLocale,
                b.workspace.settings.date_display,
              ),
          },
          { header: t("Description"), accessorKey: "reason" },
          {
            header: t("Change"),
            cell: ({ row }) => money(signed(row.original)),
          },
        ]}
      />
      <div className="form-actions">
        <Button variant="outline" onClick={onClose}>
          {t("Close")}
        </Button>
        <Button
          onClick={() => setChecked(Object.fromEntries(movements.map((j) => [j.id, true])))}
        >
          {t("Mark all")}
        </Button>
      </div>
    </Modal>
  );
}
function MovementDialog({
  kind,
  data,
  onClose,
  onPosted,
}: {
  kind: MovementKind;
  data: BankingData;
  onClose: () => void;
  onPosted: () => Promise<void>;
}) {
  const b = useBooks(),
    t = b.t;
  const [key] = useState(uid),
    [date, setDate] = useState(today),
    [amount, setAmount] = useState("");
  const [account, setAccount] = useState(
    data.accounts.find((a) => a.role === "Bank")?.account.id ??
      data.accounts[0]?.account.id ??
      "",
  );
  const [counterpart, setCounterpart] = useState(""),
    [reference, setReference] = useState(""),
    [reason, setReason] = useState("");
  const [review, setReview] = useState<
      (Journal & { _generation: string }) | null
    >(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const title = t(
    kind === "deposit"
      ? "Money in"
      : kind === "withdrawal"
        ? "Money out"
        : "Transfer money",
  );
  const payload = {
    kind,
    company_id: b.companyId,
    period_id: b.periodId,
    source_id: key,
    request_key: key,
    date,
    account_id: account,
    counterpart_id: counterpart,
    reference,
    reason,
    amount,
  };
  const [discard, setDiscard] = useState(false);
  const close = () => {
    if (busy) return;
    if (amount || reference || reason || counterpart) setDiscard(true);
    else onClose();
  };
  async function act(post = false) {
    if (busy || (post && !review)) return;
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command("banking_post", {
          ...payload,
          expected_generation: review?._generation,
        });
        await onPosted();
        toast.success(t("Banking movement recorded"));
      } else setReview(await command("banking_preview", payload));
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
          if (!v) close();
        }}
        title={title}
        wide
        description={t("Record a reviewed cash/bank movement in the ledger.")}
      >
        <Note>
          {t(
            "Use these movements for funding, operating income or expenses, and cash/bank transfers. They do not settle customer or supplier invoices.",
          )}
        </Note>
        {error && <ErrorState error={error} />}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void act();
          }}
          onChange={() => setReview(null)}
        >
          <FieldGroup className="form-grid">
            <Field
              label={t(
                kind === "transfer" ? "From account" : "Cash / bank account",
              )}
            >
              <NativeSelect
                required
                value={account}
                onChange={(e) => {
                  setAccount(e.target.value);
                  setCounterpart("");
                  setReview(null);
                }}
              >
                {data.accounts.map((a) => (
                  <NativeSelectOption key={a.account.id} value={a.account.id}>
                    {a.account.name}
                  </NativeSelectOption>
                ))}
              </NativeSelect>
            </Field>
            <Field
              label={t(
                kind === "transfer"
                  ? "To account"
                  : "Category / counterpart account",
              )}
            >
              <NativeSelect
                required
                value={counterpart}
                onChange={(e) => setCounterpart(e.target.value)}
              >
                <NativeSelectOption value="">
                  {t("Select account")}
                </NativeSelectOption>
                {(kind === "transfer"
                  ? data.accounts
                      .map((a) => a.account)
                      .filter((a) => a.id !== account)
                  : data.counterparts
                ).map((a) => (
                  <NativeSelectOption key={a.id} value={a.id}>
                    {a.code} · {a.name}
                  </NativeSelectOption>
                ))}
              </NativeSelect>
            </Field>
            <Field label={t("Amount · IRR")}>
              <Input
                required
                inputMode="decimal"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
              />
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
              />
            </Field>
            <Field label={t("Description")}>
              <Textarea
                required
                maxLength={160}
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
            </Field>
          </FieldGroup>
          <div className="form-actions">
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={close}
            >
              {t("Cancel")}
            </Button>
            <Button type="submit" disabled={busy}>
              {t("Review movement")}
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
              data={review.lines}
              id={(l) => l.account.id}
              empty=""
              columns={[
                {
                  header: t("Account"),
                  cell: ({ row }) => row.original.account.name,
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
        description={t("Discard unsaved banking movement?")}
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

import {
  Table,
  TableHeader,
  TableBody,
  TableFooter,
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
import { Plus, LockKeyhole, LockKeyholeOpen } from "lucide-react";
import { toast } from "sonner";
import { tabKeyboard } from "../lib/tab-keyboard";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { Journal, Period } from "../lib/types";
import { PageHeader, Field, ErrorState, Note } from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { Modal } from "../components/ui/dialog";
import { DataGrid } from "../components/ui/data-grid";
import { Money, Status, formatDate } from "../components/finance/money";
export function Accounting({ onDetail }: { onDetail: (j: Journal) => void }) {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Journals");
  const [period, setPeriod] = useState<Period | null>(null);
  const [reason, setReason] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [offset, setOffset] = useState(0);
  const [rows, setRows] = useState<Journal[] | null>(null);
  const [rowsSyncedAt, setRowsSyncedAt] = useState<Date | null>(null);
  async function page(n: number) {
    try {
      const w = await command<{ journals: Journal[] }>("workspace", {
        company_id: b.companyId,
        offset: String(n),
      });
      setRows(w.journals);
      setRowsSyncedAt(new Date());
      setOffset(n);
    } catch (e) {
      setError((e as Error).message);
    }
  }
  return (
    <>
      <PageHeader
        title={b.t("Accounting")}
        syncedAt={tab === "Journals" && rows ? rowsSyncedAt : undefined}
        actions={
          <Button disabled={!b.workspace.initialized} onClick={b.openJournal}>
            <Plus />
            {b.t("New journal")}
          </Button>
        }
      />
      <div className="tabs" role="tablist" onKeyDown={tabKeyboard}>
        {["Journals", "Chart of accounts", "Fiscal periods"].map((x) => (
          <button
            className="tab"
            role="tab"
            aria-selected={tab === x}
            tabIndex={tab === x ? 0 : -1}
            key={x}
            onClick={() => setTab(x)}
          >
            {b.t(x)}
          </button>
        ))}
      </div>
      {error && <ErrorState error={error} />}
      <section className="panel">
        {tab === "Journals" ? (
          <>
            <DataGrid
              data={rows ?? b.workspace.journals}
              id={(x) => x.id}
              searchText={(x) => x.reason + x.source_id}
              empty={t("No journals")}
              columns={[
                {
                  header: t("Entry"),
                  cell: ({ row }) => (
                    <button
                      className="text-link"
                      onClick={() => onDetail(row.original)}
                    >
                      #{row.original.id.slice(0, 8)}
                    </button>
                  ),
                },
                {
                  header: t("Date"),
                  accessorKey: "date",
                  cell: ({ row }) =>
                    formatDate(
                      row.original.date,
                      b.dateLocale,
                      b.workspace.settings.date_display,
                    ),
                },
                { header: t("Reason"), accessorKey: "reason" },
                {
                  header: t("Kind"),
                  cell: ({ row }) =>
                    typeof row.original.kind === "string"
                      ? t(row.original.kind)
                      : t("Reversal"),
                },
                {
                  header: `${t("Debit")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                  meta: { numeric: true },
                  cell: ({ row }) => (
                    <Money
                      value={row.original.total_debit}
                      unit={b.unit}
                      locale={b.numberLocale}
                    />
                  ),
                },
                {
                  header: `${t("Credit")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                  meta: { numeric: true },
                  cell: ({ row }) => (
                    <Money
                      value={row.original.total_credit}
                      unit={b.unit}
                      locale={b.numberLocale}
                    />
                  ),
                },
                {
                  header: t("State"),
                  cell: () => <Status kind="positive">{t("Posted")}</Status>,
                },
              ]}
            />
            <div className="pagination">
              <span>
                {t("{count} posted journals", {
                  count: new Intl.NumberFormat(
                    b.numberLocale === "fa" ? "fa-IR" : "en-US",
                  ).format(BigInt(b.workspace.journal_count)),
                })}
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
                  disabled={offset + 100 >= Number(b.workspace.journal_count)}
                  onClick={() => page(offset + 100)}
                >
                  {t("Next")}
                </Button>
              </div>
            </div>
          </>
        ) : tab === "Chart of accounts" ? (
          <DataGrid
            data={b.workspace.accounts}
            id={(x) => x.id}
            searchText={(x) => x.code + x.name}
            empty={t("No accounts")}
            columns={[
              { header: t("Code"), accessorKey: "code" },
              { header: t("Account"), accessorKey: "name" },
              { header: t("Classification"), accessorKey: "classification" },
              { header: t("Normal side"), accessorKey: "normal_balance" },
              {
                header: t("Control"),
                cell: ({ row }) =>
                  row.original.control
                    ? t("Subledger required")
                    : t("Manual posting allowed"),
              },
              {
                header: t("State"),
                cell: ({ row }) =>
                  row.original.archived ? t("Archived") : t("Active"),
              },
            ]}
          />
        ) : (
          <DataGrid
            data={b.workspace.periods}
            id={(x) => x.id}
            empty={t("No fiscal periods")}
            columns={[
              { header: t("Period"), accessorKey: "name" },
              {
                header: t("Start"),
                accessorKey: "start_date",
                cell: ({ row }) =>
                  formatDate(
                    row.original.start_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("End"),
                accessorKey: "end_date",
                cell: ({ row }) =>
                  formatDate(
                    row.original.end_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  ),
              },
              {
                header: t("State"),
                cell: ({ row }) => (
                  <Status kind={row.original.locked ? "warning" : "positive"}>
                    {row.original.locked ? t("Locked") : t("Open")}
                  </Status>
                ),
              },
              {
                header: "",
                cell: ({ row }) => (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setPeriod(row.original);
                      setReason("");
                    }}
                  >
                    {row.original.locked ? (
                      <LockKeyholeOpen />
                    ) : (
                      <LockKeyhole />
                    )}
                    {row.original.locked ? t("Reopen") : t("Lock")}
                  </Button>
                ),
              },
            ]}
          />
        )}
      </section>
      <Modal
        open={!!period}
        onOpenChange={(v) => {
          if (!v && !busy) setPeriod(null);
        }}
        title={
          period?.locked ? t("Reopen fiscal period") : t("Lock fiscal period")
        }
        confirmation
        description={t(
          period?.locked
            ? "Reopen {period}. Allows posting in this period and records an audit event."
            : "Lock {period}. Prevents posting in this period and records an audit event.",
          { period: period?.name ?? t("fiscal period") },
        )}
      >
        <Field label={t("Reason")}>
          <Textarea
            required
            value={reason}
            onChange={(e) => setReason(e.target.value)}
          />
        </Field>
        <div className="form-actions">
          <Button variant="outline" onClick={() => setPeriod(null)}>
            {t("Cancel")}
          </Button>
          <Button
            disabled={busy || !reason.trim()}
            onClick={async () => {
              setBusy(true);
              setError("");
              try {
                await command("period_lock", {
                  company_id: b.companyId,
                  period_id: period!.id,
                  locked: !period!.locked,
                  reason,
                });
                await b.refresh();
                setPeriod(null);
                toast.success(t("Period state updated"));
              } catch (e) {
                setError((e as Error).message);
              } finally {
                setBusy(false);
              }
            }}
          >
            {t("Confirm")}
            {period?.locked ? t("reopen") : t("lock")}
          </Button>
        </div>
      </Modal>
    </>
  );
}
export function JournalEditor({ onClose }: { onClose: () => void }) {
  const t = useTranslation();
  const b = useBooks();
  const [date, setDate] = useState(today);
  const [reason, setReason] = useState("");
  const [lines, setLines] = useState([
    { id: uid(), account_id: "", debit: "0", credit: "0", description: "" },
    { id: uid(), account_id: "", debit: "0", credit: "0", description: "" },
  ]);
  const [key] = useState(uid);
  const [preview, setPreview] = useState<Journal | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const payload = {
    company_id: b.companyId,
    period_id: b.periodId,
    date,
    reason,
    source_id: key,
    request_key: key,
    lines: lines.map(({ id: _, ...l }) => l),
  };
  function update(id: string, k: string, v: string) {
    setPreview(null);
    setLines((ls) => ls.map((l) => (l.id === id ? { ...l, [k]: v } : l)));
  }
  async function run(post = false) {
    setBusy(true);
    setError("");
    try {
      if (post) {
        await command("journal_post", {
          ...payload,
          expected_generation: preview?._generation,
        });
        await b.refresh();
        toast.success(t("Journal posted"));
        onClose();
      } else setPreview(await command("journal_preview", payload));
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
      title={t("New manual journal")}
      confirmation={!!preview}
      description={t(
        "Post journal “{value1}” dated {value2}. Creates an immutable journal entry.",
        {
          value1: reason,
          value2: formatDate(
            date,
            b.dateLocale,
            b.workspace.settings.date_display,
          ),
        },
      )}
      wide
    >
      {error && <ErrorState error={error} />}
      <div className="form-grid">
        <Field label={t("Financial date")}>
          <Input
            type="date"
            value={date}
            onChange={(e) => {
              setDate(e.target.value);
              setPreview(null);
            }}
          />
        </Field>
        <Field label={t("Reason")}>
          <Input
            value={reason}
            onChange={(e) => {
              setReason(e.target.value);
              setPreview(null);
            }}
          />
        </Field>
      </div>
      <div className="table-scroll section-gap">
        <Table className="editor-lines">
          <TableHeader>
            <TableRow>
              <TableHead>{t("Account")}</TableHead>
              <TableHead className="numeric">{t("Debit · IRR")}</TableHead>
              <TableHead className="numeric">{t("Credit · IRR")}</TableHead>
              <TableHead>{t("Description")}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            {lines.map((l, i) => (
              <TableRow key={l.id}>
                <TableCell>
                  <NativeSelect
                    aria-label={t("Journal account {index}", { index: i + 1 })}
                    value={l.account_id}
                    onChange={(e) => update(l.id, "account_id", e.target.value)}
                  >
                    <NativeSelectOption value="">
                      {t("Select account")}
                    </NativeSelectOption>
                    {b.workspace.accounts
                      .filter((a) => a.postable && !a.archived && !a.control)
                      .map((a) => (
                        <NativeSelectOption value={a.id} key={a.id}>
                          {a.code} · {a.name}
                        </NativeSelectOption>
                      ))}
                  </NativeSelect>
                </TableCell>
                <TableCell>
                  <Input
                    aria-label={t("Journal debit {index}", { index: i + 1 })}
                    inputMode="numeric"
                    value={l.debit}
                    onChange={(e) => update(l.id, "debit", e.target.value)}
                  />
                </TableCell>
                <TableCell>
                  <Input
                    aria-label={t("Journal credit {index}", { index: i + 1 })}
                    inputMode="numeric"
                    value={l.credit}
                    onChange={(e) => update(l.id, "credit", e.target.value)}
                  />
                </TableCell>
                <TableCell>
                  <Input
                    aria-label={t("Journal description {index}", {
                      index: i + 1,
                    })}
                    value={l.description}
                    onChange={(e) =>
                      update(l.id, "description", e.target.value)
                    }
                  />
                </TableCell>
                <TableCell>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setLines(lines.filter((x) => x.id !== l.id));
                      setPreview(null);
                    }}
                    aria-label={t("Remove journal line {index}", {
                      index: i + 1,
                    })}
                  >
                    {t("Remove")}
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
          setLines([
            ...lines,
            {
              id: uid(),
              account_id: "",
              debit: "0",
              credit: "0",
              description: "",
            },
          ]);
          setPreview(null);
        }}
      >
        <Plus />
        {t("Add line")}
      </Button>
      {preview && (
        <Note>
          {t("Debit / credit:")} <Money value={preview.total_debit} currency />
        </Note>
      )}
      <div className="form-actions">
        <Button variant="outline" disabled={busy} onClick={onClose}>
          {t("Cancel")}
        </Button>
        <Button variant="outline" disabled={busy} onClick={() => run()}>
          {t("Validate journal")}
        </Button>
        <Button disabled={busy || !preview} onClick={() => run(true)}>
          {busy ? t("Working…") : t("Confirm and post journal")}
        </Button>
      </div>
    </Modal>
  );
}
export function JournalDetail({
  journal,
  onClose,
}: {
  journal: Journal;
  onClose: () => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [reverse, setReverse] = useState(false);
  const [date, setDate] = useState(today);
  const [reason, setReason] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [key] = useState(uid);
  const typed =
    journal.lines.some((l) => l.account.control) ||
    typeof journal.kind !== "string" ||
    journal.kind !== "Manual";
  return (
    <Modal
      open
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title={t("Journal #{value1}", { value1: journal.id.slice(0, 8) })}
      confirmation={reverse}
      description={
        reverse
          ? t(
              "Reverse Journal #{reference}. Creates an offsetting journal and preserves the original.",
              { reference: journal.id.slice(0, 8) },
            )
          : journal.reason
      }
      wide
    >
      <div className="toolbar-actions">
        <Status kind="positive">{t("Posted · immutable")}</Status>
        <span className="muted">
          {formatDate(
            journal.date,
            b.dateLocale,
            b.workspace.settings.date_display,
          )}{" "}
          · {journal.actor}
        </span>
      </div>
      <div className="grouped-list contact-details section-gap">
        <div className="summary-line">
          <span>{t("Reason")}</span>
          <span>{journal.reason}</span>
        </div>
        <div className="summary-line">
          <span>{t("Source")}</span>
          <span>{journal.source_id}</span>
        </div>
        <div className="summary-line">
          <span>{t("Recorded")}</span>
          <span>
            {formatDate(
              journal.recorded_at,
              b.dateLocale,
              b.workspace.settings.date_display,
            )}
          </span>
        </div>
      </div>
      <div className="panel table-scroll section-gap">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("Account")}</TableHead>
              <TableHead>{t("Description")}</TableHead>
              <TableHead className="numeric">{t("Debit · IRR")}</TableHead>
              <TableHead className="numeric">{t("Credit · IRR")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {journal.lines.map((l, i) => (
              <TableRow key={i}>
                <TableCell>
                  {l.account.code} · {l.account.name}
                </TableCell>
                <TableCell>{l.description}</TableCell>
                <TableCell>
                  <Money value={l.debit} />
                </TableCell>
                <TableCell>
                  <Money value={l.credit} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
          <TableFooter>
            <TableRow>
              <TableCell colSpan={2}>{t("Total")}</TableCell>
              <TableCell>
                <Money value={journal.total_debit} />
              </TableCell>
              <TableCell>
                <Money value={journal.total_credit} />
              </TableCell>
            </TableRow>
          </TableFooter>
        </Table>
      </div>
      {error && <ErrorState error={error} />}{" "}
      {!typed && !reverse && (
        <Button variant="outline" onClick={() => setReverse(true)}>
          {t("Create linked reversal")}
        </Button>
      )}
      {reverse && (
        <>
          <div className="form-grid">
            <Field label={t("Reversal date")}>
              <Input
                type="date"
                value={date}
                onChange={(e) => setDate(e.target.value)}
              />
            </Field>
            <Field label={t("Reason")}>
              <Input
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
            </Field>
          </div>
          <div className="form-actions">
            <Button variant="outline" onClick={() => setReverse(false)}>
              {t("Cancel")}
            </Button>
            <Button
              disabled={busy || !reason.trim()}
              onClick={async () => {
                setBusy(true);
                setError("");
                try {
                  await command("journal_reverse", {
                    company_id: b.companyId,
                    journal_id: journal.id,
                    period_id: b.periodId,
                    date,
                    reason,
                    request_key: key,
                  });
                  await b.refresh();
                  toast.success(t("Linked reversal posted"));
                  onClose();
                } catch (e) {
                  setError((e as Error).message);
                } finally {
                  setBusy(false);
                }
              }}
            >
              {t("Confirm reversal")}
            </Button>
          </div>
        </>
      )}
    </Modal>
  );
}

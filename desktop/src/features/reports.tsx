import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { DownloadSimpleIcon as Download } from "@phosphor-icons/react/dist/csr/DownloadSimple";
import { toast } from "sonner";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import { useBooks } from "../app/books";
import { command, exportReport } from "../lib/client";
import {
  PageHeader,
  Field,
  ErrorState,
  Loading,
  Note,
} from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { Modal } from "../components/layout/modal";
import { DataGrid } from "../components/ui/data-grid";
import { Money, Status, formatDate } from "../components/finance/money";
interface LedgerRow {
  journal_id: string;
  date: string;
  reference: string;
  description: string;
  debit: string;
  credit: string;
}
export function Reports({ onJournal }: { onJournal: (id: string) => void }) {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Profit & Loss");
  const [error, setError] = useState("");
  const [exporting, setExporting] = useState(false);
  const [ledger, setLedger] = useState<{
    name: string;
    account_id: string;
    rows: LedgerRow[];
    count: string;
    offset: number;
    loadedAt: Date;
  } | null>(null);
  async function drill(aid: string, name: string, offset = 0) {
    setError("");
    try {
      const result = await command<{ rows: LedgerRow[]; count: string }>(
        "ledger",
        {
          company_id: b.companyId,
          account_id: aid,
          from: b.from,
          to: b.to,
          offset: String(offset),
        },
      );
      setLedger({
        name,
        account_id: aid,
        ...result,
        offset,
        loadedAt: new Date(),
      });
    } catch (e) {
      setError((e as Error).message);
    }
  }
  const r = b.report;
  return (
    <Tabs
      value={tab}
      onValueChange={setTab}
      dir={b.locale === "fa" ? "rtl" : "ltr"}
    >
      <PageHeader
        title={b.t("Reports")}
        actions={
          tab !== "Control reconciliation" && (
            <Button
              variant="default"
              disabled={exporting || !r}
              onClick={async () => {
                setExporting(true);
                try {
                  if (
                    await exportReport(
                      tab === "Profit & Loss" ? "profit_csv" : "report_csv",
                      { company_id: b.companyId, from: b.from, to: b.to },
                    )
                  )
                    toast.success(t("Report exported"));
                } catch (e) {
                  setError((e as Error).message);
                } finally {
                  setExporting(false);
                }
              }}
            >
              <Download />
              {b.t("Export CSV")}
            </Button>
          )
        }
      />
      <TabsList>
        {["Profit & Loss", "Trial balance", "Control reconciliation"].map(
          (x) => (
            <TabsTrigger key={x} value={x}>
              {b.t(x)}
            </TabsTrigger>
          ),
        )}
      </TabsList>
      <TabsContent value={tab}>
        <div className="period-context">
          <Field label={t("From")}>
            <Input
              type="date"
              value={b.from}
              onChange={(e) => b.setRange(e.target.value, b.to)}
            />
          </Field>
          <Field label={t("Through")}>
            <Input
              type="date"
              value={b.to}
              onChange={(e) => b.setRange(b.from, e.target.value)}
            />
          </Field>
          <span>
            · {b.unit === "Rial" ? "IRR" : t("toman")} ·{" "}
            {tab === "Trial balance"
              ? t("Cumulative through cutoff")
              : t("Posted entries")}
          </span>
        </div>
        {(error || b.reportError) && (
          <ErrorState error={error || b.reportError} />
        )}{" "}
        {!b.companyId ? (
          <Note>{t("Company required")}</Note>
        ) : !r && !b.reportError ? (
          <Loading />
        ) : (
          r && (
            <section className="panel">
              {tab === "Profit & Loss" ? (
                <>
                  <div className="panel-heading">
                    <h2>{b.t("Profit & Loss")}</h2>
                    <span className="muted" style={{ fontSize: 14 }}>
                      {formatDate(
                        b.from,
                        b.dateLocale,
                        b.workspace.settings.date_display,
                      )}{" "}
                      —{" "}
                      {formatDate(
                        b.to,
                        b.dateLocale,
                        b.workspace.settings.date_display,
                      )}
                    </span>
                  </div>
                  <DataGrid
                    data={r.profit_loss.rows.filter((x) => x.amount !== "0")}
                    totals={[
                      b.t("Net profit"),
                      "",
                      <Money
                        value={r.profit_loss.profit}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />,
                    ]}
                    id={(x) => x.account_id}
                    empty={t("No entries in this period")}
                    columns={[
                      {
                        header: t("Account"),
                        cell: ({ row }) => (
                          <Button
                            variant="link"
                            className="text-link h-auto justify-start p-0"
                            onClick={() =>
                              drill(row.original.account_id, row.original.name)
                            }
                          >
                            {row.original.code} · {row.original.name}
                          </Button>
                        ),
                      },
                      {
                        header: t("Classification"),
                        accessorKey: "classification",
                      },
                      {
                        header: `${t("Amount")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                        meta: { numeric: true },
                        cell: ({ row }) => (
                          <Money
                            value={row.original.amount}
                            unit={b.unit}
                            locale={b.numberLocale}
                          />
                        ),
                      },
                    ]}
                  />

                  <div className="detail-summary">
                    <div>
                      <small>{b.t("Revenue")}</small>
                      <Money
                        value={r.profit_loss.revenue}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />
                    </div>
                    <div>
                      <small>{b.t("Expenses")}</small>
                      <Money
                        value={r.profit_loss.expenses}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />
                    </div>
                  </div>
                </>
              ) : tab === "Trial balance" ? (
                <>
                  <div className="panel-heading">
                    <h2>{b.t("Trial balance")}</h2>
                    <Status
                      kind={
                        r.trial.total_debit === r.trial.total_credit
                          ? "positive"
                          : "danger"
                      }
                    >
                      {r.trial.total_debit === r.trial.total_credit
                        ? t("Balanced")
                        : t("Review difference")}
                    </Status>
                  </div>
                  <DataGrid
                    data={r.trial.rows}
                    totals={[
                      "Total",
                      "",
                      <Money
                        value={r.trial.total_debit}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />,
                      <Money
                        value={r.trial.total_credit}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />,
                    ]}
                    id={(x) => x.account_id}
                    empty={t("No balances through this date")}
                    columns={[
                      { header: t("Code"), accessorKey: "code" },
                      {
                        header: t("Account"),
                        cell: ({ row }) => (
                          <Button
                            variant="link"
                            className="text-link h-auto justify-start p-0"
                            onClick={() =>
                              drill(row.original.account_id, row.original.name)
                            }
                          >
                            {row.original.name}
                          </Button>
                        ),
                      },
                      {
                        header: `${t("Debit")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                        meta: { numeric: true },
                        cell: ({ row }) => (
                          <Money
                            value={row.original.debit}
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
                            value={row.original.credit}
                            unit={b.unit}
                            locale={b.numberLocale}
                          />
                        ),
                      },
                    ]}
                  />
                </>
              ) : r.reconciliation ? (
                <>
                  <div className="panel-heading">
                    <h2>{t("Subledger control reconciliation")}</h2>
                    <Status
                      kind={r.reconciliation.reconciled ? "positive" : "danger"}
                    >
                      {r.reconciliation.reconciled
                        ? t("All controls agree")
                        : t("Investigate differences")}
                    </Status>
                  </div>
                  <DataGrid
                    data={r.reconciliation.controls}
                    id={(x) => x.account_id}
                    empty={t("No controls.")}
                    columns={[
                      { header: t("Control"), accessorKey: "role" },
                      {
                        header: `${t("Ledger")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                        meta: { numeric: true },
                        cell: ({ row }) => (
                          <Money
                            value={row.original.ledger}
                            unit={b.unit}
                            locale={b.numberLocale}
                          />
                        ),
                      },
                      {
                        header: `${t("Subledger")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                        meta: { numeric: true },
                        cell: ({ row }) => (
                          <Money
                            value={row.original.detail}
                            unit={b.unit}
                            locale={b.numberLocale}
                          />
                        ),
                      },
                      {
                        header: `${t("Difference")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                        meta: { numeric: true },
                        cell: ({ row }) => (
                          <Money
                            value={row.original.difference}
                            unit={b.unit}
                            locale={b.numberLocale}
                          />
                        ),
                      },
                    ]}
                  />
                </>
              ) : (
                <Note>{t("Opening balances required")}</Note>
              )}
            </section>
          )
        )}
        <Modal
          open={!!ledger}
          onOpenChange={(v) => {
            if (!v) setLedger(null);
          }}
          title={ledger?.name ?? t("General ledger")}
          description={t(
            "Posted account activity · {value1} through {value2}",
            {
              value1: b.from,
              value2: b.to,
            },
          )}
          wide
        >
          {ledger && (
            <>
              <div className="sync-time">
                {t("Last synced")}{" "}
                <time dateTime={ledger.loadedAt.toISOString()}>
                  {ledger.loadedAt
                    ? new Intl.DateTimeFormat(b.dateLocale, {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                      }).format(ledger.loadedAt)
                    : "—"}
                </time>
              </div>
              <div className="panel">
                <DataGrid
                  data={ledger.rows}
                  id={(x) => x.journal_id + x.description + x.debit + x.credit}
                  empty={t("No entries in this period")}
                  columns={[
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
                    {
                      header: t("Source"),
                      cell: ({ row }) => (
                        <Button
                          variant="link"
                          className="text-link h-auto justify-start p-0"
                          onClick={() => {
                            onJournal(row.original.journal_id);
                            setLedger(null);
                          }}
                        >
                          {row.original.reference}
                        </Button>
                      ),
                    },
                    { header: t("Description"), accessorKey: "description" },
                    {
                      header: `${t("Debit")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                      meta: { numeric: true },
                      cell: ({ row }) => (
                        <Money
                          value={row.original.debit}
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
                          value={row.original.credit}
                          unit={b.unit}
                          locale={b.numberLocale}
                        />
                      ),
                    },
                  ]}
                />
                <div className="pagination">
                  <span>
                    {t("{count} account entries", {
                      count: new Intl.NumberFormat(
                        b.numberLocale === "fa" ? "fa-IR" : "en-US",
                      ).format(BigInt(ledger.count)),
                    })}
                  </span>
                  <div className="toolbar-actions">
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={!ledger.offset}
                      onClick={() =>
                        drill(
                          ledger.account_id,
                          ledger.name,
                          Math.max(0, ledger.offset - 100),
                        )
                      }
                    >
                      {t("Previous")}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={ledger.offset + 100 >= Number(ledger.count)}
                      onClick={() =>
                        drill(
                          ledger.account_id,
                          ledger.name,
                          ledger.offset + 100,
                        )
                      }
                    >
                      {t("Next")}
                    </Button>
                  </div>
                </div>
              </div>
            </>
          )}
        </Modal>
      </TabsContent>
    </Tabs>
  );
}

import { Input } from "../components/ui/input";
import {
  ArrowUpRight,
  Banknote,
  FileText,
  ShoppingBag,
  Plus,
} from "lucide-react";
import { useBooks } from "../app/books";
import { Button } from "../components/ui/button";
import { PageHeader, ErrorState, Loading } from "../components/ui/shared";
import { Money, Status, formatDate } from "../components/finance/money";
import { DataGrid } from "../components/ui/data-grid";
import type { Journal } from "../lib/types";
export function Overview({ onDetail }: { onDetail: (j: Journal) => void }) {
  const b = useBooks();
  const { workspace: w, report: r, t } = b;
  const metrics = [
    { key: "cash" as const, label: "Cash and bank", icon: Banknote },
    { key: "receivables" as const, label: "Receivables", icon: FileText },
    { key: "payables" as const, label: "Payables", icon: ShoppingBag },
  ];
  return (
    <>
      <PageHeader
        title={t("Overview")}
        actions={
          !w.initialized ? (
            <Button onClick={b.openSetup}>
              {b.companyId ? t("Record opening balances") : t("Set up company")}
            </Button>
          ) : (
            <Button onClick={b.openJournal}>
              <Plus />
              {t("New journal")}
            </Button>
          )
        }
      />
      <div className="period-context">
        {t("As of")}{" "}
        <Input
          aria-label={t("Overview cutoff date")}
          type="date"
          value={b.to}
          onChange={(e) => b.setRange(b.from, e.target.value)}
        />
        <span>{b.unit === "Rial" ? "IRR" : t("toman")}</span>
      </div>
      {b.reportError && <ErrorState error={b.reportError} />}
      <div className="overview-metrics">
        {metrics.map(({ key, label, icon: Icon }) => (
          <div className="metric" key={key}>
            <div className="metric-label">
              <Icon />
              {t(label)}
            </div>
            <div className="metric-value">
              {!b.companyId ? (
                "—"
              ) : r ? (
                <Money
                  value={r.position[key]}
                  unit={b.unit}
                  locale={b.numberLocale}
                />
              ) : b.reportError ? (
                "—"
              ) : (
                <span className="muted">…</span>
              )}
            </div>
          </div>
        ))}
      </div>
      {w.initialized && (
        <div className="overview-summary">
          <button onClick={() => b.navigate("sales")}>
            {t("Drafts")}
            <strong>{w.drafts.length}</strong>
          </button>
          <button onClick={() => b.navigate("accounting")}>
            {t("Journals")}
            <strong>{w.journal_count}</strong>
          </button>
          <button onClick={() => b.navigate("reports")}>
            {t("Controls")}{" "}
            <Status
              kind={r?.reconciliation?.reconciled ? "positive" : "neutral"}
            >
              {r?.reconciliation
                ? r.reconciliation.reconciled
                  ? t("Balanced")
                  : t("Difference")
                : "—"}
            </Status>
          </button>
        </div>
      )}
      <section className="panel section-gap">
        <div className="panel-heading">
          <h2>{t("Recent entries")}</h2>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => b.navigate("accounting")}
          >
            {t("View all")}
            <ArrowUpRight />
          </Button>
        </div>
        <DataGrid
          data={w.journals.slice(0, 5)}
          id={(j) => j.id}
          empty={t("No entries")}
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
              header: t("Reference"),
              cell: ({ row }) => (
                <button
                  className="text-link"
                  onClick={() => onDetail(row.original)}
                >
                  #{row.original.id.slice(0, 8)}
                </button>
              ),
            },
            { header: t("Description"), accessorKey: "reason" },
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
          ]}
        />
      </section>
      {b.companyId && !r && !b.reportError && <Loading />}
    </>
  );
}

import { useBooks } from "../../app/books";
import { DataGrid } from "../ui/data-grid";
import { Note } from "../ui/shared";
import { Money, Status } from "./money";
import type { Aging, AgingRow, BalanceSheet, StatementRow } from "../../lib/types";

export function BalanceSheetPanel({ sheet }: { sheet?: BalanceSheet | null }) {
  const b = useBooks(),
    t = b.t;
  if (!sheet) return <Note>{t("Confirm opening balances to see the balance sheet.")}</Note>;
  const money = (value: string) => (
    <Money value={value} unit={b.unit} locale={b.numberLocale} />
  );
  const section = (title: string, rows: StatementRow[], total: string, extra?: StatementRow) => {
    const data = extra ? [...rows, extra] : rows;
    return (
      <>
        <h3>{t(title)}</h3>
        <DataGrid
          data={data}
          id={(r) => r.account_id}
          empty={t("Nothing to show")}
          columns={[
            { header: t("Code"), cell: ({ row }) => row.original.code },
            { header: t("Account"), cell: ({ row }) => row.original.name },
            {
              header: `${t("Amount")} · ${b.unit === "Rial" ? "IRR" : t("toman")}`,
              meta: { numeric: true },
              cell: ({ row }) => money(row.original.amount),
            },
          ]}
        />
        <p className="total-row">
          {t("Total")}: {money(total)}
        </p>
      </>
    );
  };
  return (
    <>
      <div className="panel-heading">
        <h2>{t("Balance sheet")}</h2>
        <Status kind={sheet.balanced ? "positive" : "danger"}>
          {sheet.balanced ? t("Balanced") : t("Out of balance")}
        </Status>
      </div>
      {section("Assets", sheet.assets, sheet.total_assets)}
      {section("Liabilities", sheet.liabilities, sheet.total_liabilities)}
      {section(
        "Equity",
        sheet.equity,
        sheet.total_equity,
        sheet.current_earnings === "0"
          ? undefined
          : {
              account_id: "current-earnings",
              code: "",
              name: t("Current earnings (not yet closed)"),
              amount: sheet.current_earnings,
            },
      )}
    </>
  );
}

export function AgingPanel({
  aging,
}: {
  aging?: { receivable: Aging; payable: Aging } | null;
}) {
  const b = useBooks(),
    t = b.t;
  if (!aging) return <Note>{t("Confirm opening balances to see aging.")}</Note>;
  const partyName = (id: string) =>
    b.workspace.resources?.parties.find((p) => p.id === id)?.name ?? id;
  const money = (value: string) => (
    <Money value={value} unit={b.unit} locale={b.numberLocale} />
  );
  const table = (title: string, who: string, a: Aging) => {
    const data: AgingRow[] = a.rows.length ? [...a.rows, a.totals] : [];
    const num = (key: keyof AgingRow, label: string) => ({
      header: t(label),
      meta: { numeric: true },
      cell: ({ row }: { row: { original: AgingRow } }) => money(row.original[key]),
    });
    return (
      <>
        <h3>{t(title)}</h3>
        <DataGrid
          data={data}
          id={(r) => r.party_id}
          empty={t("Nothing is open")}
          columns={[
            {
              header: t(who),
              cell: ({ row }) =>
                row.original.party_id === "total"
                  ? t("Total")
                  : partyName(row.original.party_id),
            },
            num("not_due", "Not due"),
            num("days_1_30", "1–30 days"),
            num("days_31_60", "31–60 days"),
            num("days_61_90", "61–90 days"),
            num("days_over_90", "Over 90 days"),
            num("total", "Total"),
          ]}
        />
      </>
    );
  };
  return (
    <>
      <div className="panel-heading">
        <h2>{t("Aging")}</h2>
      </div>
      {table("Receivables by days overdue", "Customer", aging.receivable)}
      {table("Payables by days overdue", "Vendor", aging.payable)}
    </>
  );
}

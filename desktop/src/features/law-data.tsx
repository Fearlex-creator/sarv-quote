import { useEffect, useState } from "react";
import { useBooks } from "../app/books";
import { command } from "../lib/client";
import { Badge } from "../components/ui/badge";
import { DataGrid } from "../components/ui/data-grid";
import { ErrorState, Loading } from "../components/ui/shared";
import { SettingsPage } from "../components/ui/settings";
import { LawNotice } from "../components/finance/law-notice";

interface LawEntry {
  key: string;
  title_en: string;
  title_fa: string;
  value: string;
  from_year: number | string;
  to_year: number | string;
  verify: boolean;
  reviewed: boolean;
  note: string;
}

/** Read-only list of every legal value in this build, with its review state. */
export function LawData() {
  const b = useBooks(),
    t = b.t;
  const [entries, setEntries] = useState<LawEntry[] | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    command<{ entries: LawEntry[] }>("law_data")
      .then((r) => {
        if (active) setEntries(r.entries);
      })
      .catch((e) => {
        if (active) setError((e as Error).message);
      });
    return () => {
      active = false;
    };
  }, []);
  return (
    <SettingsPage>
      <LawNotice />
      {error && <ErrorState error={error} />}
      {!entries ? (
        <Loading />
      ) : (
        <DataGrid
          data={entries}
          id={(e) => e.key}
          empty={t("No legal values")}
          columns={[
            {
              header: t("Value"),
              cell: ({ row }) =>
                b.locale === "fa" ? row.original.title_fa : row.original.title_en,
            },
            {
              header: t("Current setting"),
              cell: ({ row }) => <bdi dir="ltr">{row.original.value}</bdi>,
            },
            {
              header: t("Jalali years"),
              cell: ({ row }) => `${row.original.from_year}–${row.original.to_year}`,
            },
            {
              header: t("Status"),
              cell: ({ row }) => (
                <span className="row-actions">
                  <Badge variant="destructive">{t("Not reviewed")}</Badge>
                  {row.original.verify && (
                    <Badge variant="outline">{t("Needs verification")}</Badge>
                  )}
                </span>
              ),
            },
            { header: t("Note"), cell: ({ row }) => row.original.note },
          ]}
        />
      )}
    </SettingsPage>
  );
}

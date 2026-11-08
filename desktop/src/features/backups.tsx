import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command } from "../lib/client";
import { Button } from "../components/ui/button";
import { DataGrid } from "../components/ui/data-grid";
import { ErrorState, Loading, Note } from "../components/ui/shared";
import {
  SettingsGroup,
  SettingsPage,
} from "../components/ui/settings";
import { Modal } from "../components/layout/modal";
import { formatDate } from "../components/finance/money";
import { invoke, isTauri } from "@tauri-apps/api/core";

interface BackupRow {
  filename: string;
  bytes: number;
  modified: number;
  companies: number;
  journals: number;
  valid: boolean;
}
interface RestoreSummary {
  companies: string[];
  journals: number;
  sales: number;
  purchases: number;
  payments: number;
  quotes: number;
  drafts: number;
}

export function Backups() {
  const b = useBooks(),
    t = b.t;
  const [rows, setRows] = useState<BackupRow[] | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const [restore, setRestore] = useState<{
    filename: string;
    backup: RestoreSummary;
    current: RestoreSummary;
    generation: string;
  } | null>(null);
  const load = useCallback(async () => {
    const result = await command<{ backups: BackupRow[] }>("backup_list", {});
    setRows(result.backups);
    setError("");
  }, []);
  useEffect(() => {
    load().catch((e) => setError((e as Error).message));
  }, [load]);
  async function create() {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      const r = await command<{ filename: string; bytes: number }>(
        "backup_create",
        {},
      );
      toast.success(
        t("Backup created: {name}", {
          name: `${r.filename} (${Math.max(1, Math.round(r.bytes / 1024))} KB)`,
        }),
      );
      await load();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function exportToFile() {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      // Native save dialog only: paths never come from web content.
      await invoke<boolean>("save_report", {
        action: "backup_export",
        payload: {},
      });
      toast.success(t("Backup exported"));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function openRestore(filename: string) {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      const r = await command<{
        backup: RestoreSummary;
        current: RestoreSummary;
        _generation: string;
      }>("backup_restore_preview", { filename });
      setRestore({
        filename,
        backup: r.backup,
        current: r.current,
        generation: r._generation,
      });
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  async function confirmRestore() {
    if (!restore || busy) return;
    setBusy(true);
    setError("");
    try {
      await command("backup_restore", {
        filename: restore.filename,
        confirmed: true,
        expected_generation: restore.generation,
      });
      toast.success(t("Books restored from backup"));
      setRestore(null);
      await b.refresh();
      await load();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const kb = (n: number) => `${Math.max(1, Math.round(n / 1024))} KB`;
  const summaryLine = (s: RestoreSummary) =>
    `${s.companies.length} ${t("Businesses")} · ${s.journals} ${t("Journals")} · ${s.sales} ${t("Sales")} · ${s.payments} ${t("Payments")}`;
  return (
    <SettingsPage>
      <SettingsGroup
        title={t("Backups")}
        description={t(
          "A backup captures the complete books with a checksum. Restoring replaces everything and keeps an automatic safety copy of the current books first.",
        )}
        action={
          <div className="flex gap-2">
            {isTauri() && (
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void exportToFile()}
              >
                {t("Export backup to file…")}
              </Button>
            )}
            <Button disabled={busy} onClick={() => void create()}>
              {t("Create backup")}
            </Button>
          </div>
        }
      >
        <div className="[&_[data-slot=table-container]]:rounded-none">
          {!rows ? (
            <Loading />
          ) : (
            <DataGrid
              data={rows}
              id={(r) => r.filename}
              searchText={(r) => r.filename}
              empty={t("No backups yet. Create one before your first closing.")}
              columns={[
                {
                  header: t("Backup"),
                  cell: ({ row }) => (
                    <span dir="ltr">{row.original.filename.slice(12, -5)}</span>
                  ),
                },
                {
                  header: t("Date"),
                  cell: ({ row }) =>
                    formatDate(
                      new Date(row.original.modified * 1000)
                        .toISOString()
                        .slice(0, 10),
                      b.dateLocale,
                      b.workspace.settings.date_display,
                    ),
                },
                { header: t("Size"), cell: ({ row }) => kb(row.original.bytes) },
                {
                  header: t("Contents"),
                  cell: ({ row }) =>
                    row.original.valid
                      ? `${row.original.companies} ${t("Businesses")} · ${row.original.journals} ${t("Journals")}`
                      : t("Unreadable"),
                },
                {
                  header: "",
                  cell: ({ row }) => (
                    <Button
                      variant="outline"
                      disabled={busy || !row.original.valid}
                      onClick={() => void openRestore(row.original.filename)}
                    >
                      {t("Restore…")}
                    </Button>
                  ),
                },
              ]}
            />
          )}
        </div>
      </SettingsGroup>
      {error && <ErrorState error={error} retry={() => void load()} />}
      <Note>
        {t(
          "Backups also capture quotations, drafts and profiles. Keep a copy outside this device for real books.",
        )}
      </Note>
      {restore && (
        <Modal
          open
          onOpenChange={(v) => {
            if (!v && !busy) setRestore(null);
          }}
          title={t("Restore books from backup?")}
          confirmation
          description={t(
            "The current books are replaced by the backup contents. An automatic safety backup of the current books is written first.",
          )}
        >
          <div className="restore-compare">
            <div>
              <h3>{t("Backup contains")}</h3>
              <p>{summaryLine(restore.backup)}</p>
              <ul dir="auto">
                {restore.backup.companies.map((c) => (
                  <li key={c}>{c}</li>
                ))}
              </ul>
            </div>
            <div>
              <h3>{t("Current books")}</h3>
              <p>{summaryLine(restore.current)}</p>
              <ul dir="auto">
                {restore.current.companies.map((c) => (
                  <li key={c}>{c}</li>
                ))}
              </ul>
            </div>
          </div>
          {error && (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          )}
          <div className="form-actions">
            <Button
              variant="outline"
              disabled={busy}
              onClick={() => setRestore(null)}
            >
              {t("Keep current books")}
            </Button>
            <Button
              variant="destructive"
              disabled={busy}
              onClick={() => void confirmRestore()}
            >
              {t("Restore backup")}
            </Button>
          </div>
        </Modal>
      )}
    </SettingsPage>
  );
}

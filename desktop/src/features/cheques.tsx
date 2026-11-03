import { useMemo, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import type { ChequeAction, ChequeRow, ChequeStatus } from "../lib/types";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { FieldGroup } from "../components/ui/field";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { ErrorState, Field } from "../components/ui/shared";
import { DatePicker } from "../components/ui/date-picker";
import { DataGrid } from "../components/ui/data-grid";
import { Money, formatDate } from "../components/finance/money";
import { Modal } from "../components/layout/modal";

const statusLabel: Record<ChequeStatus, string> = {
  in_hand: "In hand",
  in_collection: "In collection",
  cleared: "Cleared",
  bounced: "Bounced",
  spent: "Spent",
  returned: "Returned to customer",
  issued: "Issued",
  voided: "Voided",
};
const statusVariant: Record<
  ChequeStatus,
  "outline" | "secondary" | "destructive"
> = {
  in_hand: "outline",
  in_collection: "outline",
  issued: "outline",
  cleared: "secondary",
  spent: "secondary",
  bounced: "destructive",
  returned: "destructive",
  voided: "destructive",
};
// `spend` is done from the payment dialog (it needs a supplier and open bills).
const rowActions: ChequeAction[] = [
  "deposit",
  "clear",
  "bounce",
  "return",
  "void",
];
const actionLabel: Record<string, string> = {
  deposit: "Deposit for collection",
  clear: "Mark as cleared",
  bounce: "Mark as bounced",
  return: "Return to customer",
  void: "Void cheque",
};
const actionHelp: Record<string, string> = {
  deposit:
    "Hand the cheque to the bank for collection. No accounting entry is made.",
  clear:
    "The bank paid the cheque. This moves the amount between the cheque account and your bank account.",
  bounce:
    "The bank refused the cheque. The original payment is reversed, so the invoice becomes open again.",
  return:
    "Give the cheque back to the customer. The original payment is reversed, so the invoice becomes open again.",
  void: "Cancel the cheque you issued. The original payment is reversed, so the bill becomes open again.",
};

/** 1234 5678 9012 3456: the Sayad ID is always read in groups of four. */
export function formatSayad(id: string): string {
  return id.replace(/(\d{4})(?=\d)/g, "$1 ").trim();
}

export function ChequesPanel() {
  const b = useBooks(),
    t = b.t;
  const rows = b.workspace.cheques ?? [];
  const [action, setAction] = useState<{
    row: ChequeRow;
    kind: ChequeAction;
  } | null>(null);
  const partyName = (id: string) =>
    b.workspace.resources?.parties.find((p) => p.id === id)?.name ?? id;
  if (!rows.length) return null;
  return (
    <>
      <h2>{t("Cheques")}</h2>
      <DataGrid
        data={rows}
        id={(r) => r.cheque.id}
        empty={t("No cheques yet")}
        searchText={(r) =>
          `${partyName(r.cheque.party_id)} ${r.cheque.details.sayad_id} ${r.cheque.details.number}`
        }
        columns={[
          {
            header: t("Sayad ID"),
            cell: ({ row }) => (
              <bdi dir="ltr">
                {formatSayad(row.original.cheque.details.sayad_id) || "—"}
              </bdi>
            ),
          },
          {
            header: t("Contact"),
            cell: ({ row }) => partyName(row.original.cheque.party_id),
          },
          {
            header: t("Type"),
            cell: ({ row }) =>
              t(
                row.original.cheque.direction === "Received"
                  ? "Received"
                  : "Issued",
              ),
          },
          {
            header: t("Due date"),
            cell: ({ row }) =>
              formatDate(
                row.original.cheque.details.due_date,
                b.dateLocale,
                b.workspace.settings.date_display,
              ),
          },
          {
            header: t("Amount"),
            meta: { numeric: true },
            cell: ({ row }) => (
              <Money
                value={row.original.cheque.amount}
                unit={b.unit}
                locale={b.numberLocale}
              />
            ),
          },
          {
            header: t("Status"),
            cell: ({ row }) => (
              <Badge variant={statusVariant[row.original.status]}>
                {t(statusLabel[row.original.status])}
              </Badge>
            ),
          },
          {
            header: "",
            cell: ({ row }) => (
              <div className="row-actions">
                {rowActions
                  .filter((a) => row.original.allowed.includes(a))
                  .map((a) => (
                    <Button
                      key={a}
                      variant="outline"
                      onClick={() => setAction({ row: row.original, kind: a })}
                    >
                      {t(actionLabel[a])}
                    </Button>
                  ))}
              </div>
            ),
          },
        ]}
      />
      {action && (
        <ChequeActionDialog {...action} onClose={() => setAction(null)} />
      )}
    </>
  );
}

function ChequeActionDialog({
  row,
  kind,
  onClose,
}: {
  row: ChequeRow;
  kind: ChequeAction;
  onClose: () => void;
}) {
  const b = useBooks(),
    t = b.t;
  const [key] = useState(uid);
  const [date, setDate] = useState(today);
  const [reason, setReason] = useState("");
  const [fund, setFund] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  // Cheques clear into the cash/bank accounts mapped for this company's chart.
  const funds = useMemo(() => {
    const setup = b.workspace.setups.find(
      (s) => s.command.company_id === b.companyId,
    );
    const ids = new Set(
      (setup?.command.mappings ?? [])
        // Roles serialise in PascalCase ("Bank", "Cash").
        .filter((m) => m.role === "Bank" || m.role === "Cash")
        .map((m) => m.account_id),
    );
    return b.workspace.accounts.filter((a) => ids.has(a.id));
  }, [b.workspace.setups, b.workspace.accounts, b.companyId]);
  const needsFund = kind === "clear";
  async function submit() {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await command(`cheque_${kind}`, {
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: key,
        request_key: key,
        date,
        reason: reason || t(actionLabel[kind]),
        cheque_id: row.cheque.id,
        ...(needsFund ? { fund_account_id: fund } : {}),
      });
      await b.refresh();
      toast.success(t("Cheque updated"));
      onClose();
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
      title={t(actionLabel[kind])}
      description={t(actionHelp[kind])}
    >
      {error && <ErrorState error={error} />}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <FieldGroup className="form-grid">
          <Field label={t("Financial date")}>
            <DatePicker value={date} onChange={setDate} />
          </Field>
          {needsFund && (
            <Field label={t("Bank account")}>
              <NativeSelect
                required
                value={fund}
                onChange={(e) => setFund(e.target.value)}
              >
                <NativeSelectOption value="">
                  {t("Select account")}
                </NativeSelectOption>
                {funds.map((a) => (
                  <NativeSelectOption key={a.id} value={a.id}>
                    {a.name}
                  </NativeSelectOption>
                ))}
              </NativeSelect>
            </Field>
          )}
          <Field label={t("Description")}>
            <Input
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
            onClick={onClose}
          >
            {t("Cancel")}
          </Button>
          <Button type="submit" disabled={busy || (needsFund && !fund)}>
            {t("Confirm")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

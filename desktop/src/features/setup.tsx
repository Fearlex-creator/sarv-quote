import { FieldGroup } from "../components/ui/field";
import { Checkbox } from "../components/ui/checkbox";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "../components/ui/table";
import { Textarea } from "../components/ui/textarea";
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { CheckIcon as Check } from "@phosphor-icons/react/dist/csr/Check";
import { toast } from "sonner";
import { command, uid, today } from "../lib/client";
import { useBooks } from "../app/books";
import { Modal } from "../components/layout/modal";
import { Button } from "../components/ui/button";
import { Field, ErrorState, Note } from "../components/ui/shared";
import type { Integer } from "../lib/types";
import { Money } from "../components/finance/money";
interface OpeningReview {
  _generation: string;
  total_debit: Integer;
  total_credit: Integer;
  balanced: boolean;
  reconciled: boolean;
  controls: {
    role: string;
    ledger: Integer;
    detail: Integer;
    difference: Integer;
  }[];
}
export function SetupDialog({
  open,
  onClose,
  onCreated,
  newCompany,
  firstRun = false,
}: {
  open: boolean;
  newCompany: boolean;
  firstRun?: boolean;
  onClose: () => void;
  onCreated: (id: string) => void;
}) {
  const t = useTranslation();
  const { workspace, companyId, refresh, unit, numberLocale } = useBooks();
  const year = today().slice(0, 4);
  const [name, setName] = useState("");
  const [start, setStart] = useState(`${year}-01-01`);
  const [end, setEnd] = useState(`${year}-12-31`);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const [csv, setCsv] = useState("");
  const [preview, setPreview] = useState<OpeningReview | null>(null);
  const [step, setStep] = useState(0);
  const [key] = useState(uid);
  const needsOpening = !newCompany && !!companyId && !workspace.initialized;
  async function act(action: string, payload: unknown) {
    setBusy(true);
    setError("");
    try {
      const r = await command<{ company_id: string }>(action, payload);
      await refresh();
      if (action === "setup_company") {
        onCreated(r.company_id);
        toast.success(t("Company and fiscal period created"));
      } else {
        toast.success(t("Opening recorded"));
        onClose();
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal
      open={open}
      onOpenChange={(v) => {
        if (!v && !busy) onClose();
      }}
      title={
        needsOpening ? t("Record opening balances") : t("Set up your company")
      }
      confirmation={needsOpening}
      description={
        needsOpening
          ? t(
              "Record {balances} opening balances for {company}. Establishes the starting balances and stock for these books.",
              {
                balances: t(csv ? "the validated" : "zero"),
                company:
                  workspace.companies.find((c) => c.id === companyId)?.name ??
                  t("this company"),
              },
            )
          : t(
              "Create a separate company, fiscal period and core chart. Existing books are preserved.",
            )
      }
    >
      {firstRun && (
        <ol className="setup-progress" aria-label={t("Setup progress")}>
          {["Company", "Fiscal period", "Opening balances"].map(
            (label, index) => {
              const current = needsOpening ? 2 : step;
              return (
                <li
                  key={label}
                  aria-current={index === current ? "step" : undefined}
                  data-complete={index < current}
                >
                  <span className="step-mark" aria-hidden="true">
                    {index < current ? (
                      <Check />
                    ) : (
                      new Intl.NumberFormat(
                        numberLocale === "fa" ? "fa-IR" : "en-US",
                      ).format(index + 1)
                    )}
                  </span>
                  <span>{t(label)}</span>
                </li>
              );
            },
          )}
        </ol>
      )}
      <div className="setup-stage" key={needsOpening ? "opening" : step}>
        {error && <ErrorState error={error} />}
        {needsOpening ? (
          <>
            <Field
              label={t("Opening CSV")}
              hint={t(
                "Strict core opening CSV; amounts in integer IRR. Opening import documentation applies.",
              )}
            >
              <Textarea
                rows={7}
                value={csv}
                onChange={(e) => {
                  setCsv(e.target.value);
                  setPreview(null);
                }}
                placeholder={t("Paste opening CSV…")}
              />
            </Field>
            {csv ? (
              <div className="form-actions">
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={async () => {
                    setBusy(true);
                    setError("");
                    try {
                      setPreview(
                        await command("opening_csv_preview", {
                          company_id: companyId,
                          csv,
                          request_key: key,
                        }),
                      );
                    } catch (e) {
                      setError((e as Error).message);
                    } finally {
                      setBusy(false);
                    }
                  }}
                >
                  {t("Validate opening")}
                </Button>
                <Button
                  disabled={busy || !preview?.balanced || !preview?.reconciled}
                  onClick={() =>
                    act("opening_csv_post", {
                      company_id: companyId,
                      csv,
                      request_key: key,
                      expected_generation: preview?._generation,
                    })
                  }
                >
                  {t("Record validated opening")}
                </Button>
              </div>
            ) : (
              <>
                <label className="checkbox-label">
                  <Checkbox
                    checked={confirmed}
                    onCheckedChange={(checked) =>
                      setConfirmed(checked === true)
                    }
                  />
                  {t(
                    "I confirm this is a fresh company with zero balances, no stock, and no outstanding receivables or payables.",
                  )}
                </label>
                <div className="form-actions">
                  <Button variant="outline" onClick={onClose}>
                    {t("Cancel")}
                  </Button>
                  <Button
                    disabled={!confirmed || busy}
                    onClick={() =>
                      act("initialize_zero", {
                        company_id: companyId,
                        source_id: key,
                        request_key: key,
                        confirmed: true,
                      })
                    }
                  >
                    {busy ? t("Recording…") : t("Start with zero balances")}
                  </Button>
                </div>
              </>
            )}
            {preview && (
              <>
                <Note>
                  {t("Debit / credit:")}{" "}
                  <Money
                    value={preview.total_debit}
                    unit={unit}
                    locale={numberLocale}
                    currency
                  />
                </Note>
                <div className="table-scroll panel">
                  <Table aria-label={t("Opening control review")}>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t("Control")}</TableHead>
                        <TableHead className="numeric">
                          {t("Ledger ·")}
                          {unit === "Rial" ? "IRR" : t("toman")}
                        </TableHead>
                        <TableHead className="numeric">
                          {t("Registers ·")}
                          {unit === "Rial" ? "IRR" : t("toman")}
                        </TableHead>
                        <TableHead className="numeric">
                          {t("Difference ·")}
                          {unit === "Rial" ? "IRR" : t("toman")}
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {preview.controls.map((c) => (
                        <TableRow key={c.role}>
                          <TableCell>{t(c.role)}</TableCell>
                          <TableCell>
                            <Money
                              value={c.ledger}
                              unit={unit}
                              locale={numberLocale}
                            />
                          </TableCell>
                          <TableCell>
                            <Money
                              value={c.detail}
                              unit={unit}
                              locale={numberLocale}
                            />
                          </TableCell>
                          <TableCell>
                            <Money
                              value={c.difference}
                              unit={unit}
                              locale={numberLocale}
                            />
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              </>
            )}
          </>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (firstRun && step === 0) {
                setStep(1);
                return;
              }
              void act("setup_company", {
                name,
                fiscal_name: year,
                start_date: start,
                end_date: end,
                timezone: "AsiaTehran",
              });
            }}
          >
            <FieldGroup className="form-grid">
              {(!firstRun || step === 0) && (
                <Field label={t("Company name")}>
                  <Input
                    autoFocus
                    required
                    value={name}
                    maxLength={160}
                    onChange={(e) => setName(e.target.value)}
                    placeholder={t("Your business name")}
                  />
                </Field>
              )}
              {(!firstRun || step === 1) && (
                <>
                  <Field label={t("Fiscal period")}>
                    <Input value={year} readOnly />
                  </Field>
                  <Field label={t("Start date")}>
                    <Input
                      type="date"
                      autoFocus={firstRun}
                      required
                      value={start}
                      onChange={(e) => setStart(e.target.value)}
                    />
                  </Field>
                  <Field label={t("End date")}>
                    <Input
                      type="date"
                      required
                      value={end}
                      min={start}
                      onChange={(e) => setEnd(e.target.value)}
                    />
                  </Field>
                </>
              )}
            </FieldGroup>
            <div className="form-actions">
              <Button
                type="button"
                variant="outline"
                onClick={firstRun && step === 1 ? () => setStep(0) : onClose}
              >
                {t(firstRun && step === 1 ? "Back" : "Cancel")}
              </Button>
              <Button
                disabled={busy || (firstRun && step === 0 && !name.trim())}
              >
                {firstRun && step === 0
                  ? t("Continue")
                  : busy
                    ? t("Creating…")
                    : t("Create company")}
              </Button>
            </div>
          </form>
        )}
      </div>
    </Modal>
  );
}

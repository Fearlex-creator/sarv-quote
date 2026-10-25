import { BusinessFields } from "../components/finance/business-fields";
import { emptyBusinessProfile, type BusinessProfile } from "../lib/types";
import { DatePicker } from "../components/ui/date-picker";
import { fiscalYear, parseDate, formatCalendarDate } from "../lib/calendar";
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
import { StorefrontIcon as Storefront } from "@phosphor-icons/react/dist/csr/Storefront";
import { TruckIcon as Truck } from "@phosphor-icons/react/dist/csr/Truck";
import { BriefcaseIcon as Briefcase } from "@phosphor-icons/react/dist/csr/Briefcase";
import { FactoryIcon as Factory } from "@phosphor-icons/react/dist/csr/Factory";
import { SparkleIcon as Sparkle } from "@phosphor-icons/react/dist/csr/Sparkle";
import { FileCsvIcon as FileCsv } from "@phosphor-icons/react/dist/csr/FileCsv";
import { CalendarDotsIcon as CalendarDays } from "@phosphor-icons/react/dist/csr/CalendarDots";
import { PencilSimpleIcon as Pencil } from "@phosphor-icons/react/dist/csr/PencilSimple";
import { toast } from "sonner";
import { command, uid, today } from "../lib/client";
import { useBooks } from "../app/books";
import { toIrr } from "../lib/money-input";
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

function buildSampleOpeningCsv(companyId: string, asOfDate: string): string {
  const p = companyId.slice(0, 8) || "CO";
  return [
    "version,kind,id,company_id,name,role,amount,credit,unit,quantity,numerator,denominator,party_id,item_id,warehouse_id,unit_id,base_unit_id,customer,supplier,indivisible,reference,settlement_reference,bank_reference,holder,state,document_date,due_date,as_of,statement_amount",
    `sarv-opening-v1,opening,${p}-OPEN,${companyId},Sample Opening Balance,,,,,,,,,,,,,,,,,,,,,,,${asOfDate},`,
    `sarv-opening-v1,ledger,${p}-GL-cash,${companyId},,cash,500,0,IRR,,,,,,,,,,,,${p}-OPEN-cash,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-bank,${companyId},,bank,1000,0,IRR,,,,,,,,,,,,${p}-OPEN-bank,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-receivables,${companyId},,receivables,0,0,IRR,,,,,,,,,,,,${p}-OPEN-ar,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-inventory,${companyId},,inventory,0,0,IRR,,,,,,,,,,,,${p}-OPEN-inv,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-cheques_receivable,${companyId},,cheques_receivable,0,0,IRR,,,,,,,,,,,,${p}-OPEN-cr,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-payables,${companyId},,payables,0,0,IRR,,,,,,,,,,,,${p}-OPEN-ap,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-cheques_payable,${companyId},,cheques_payable,0,0,IRR,,,,,,,,,,,,${p}-OPEN-cp,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-grni,${companyId},,grni,0,0,IRR,,,,,,,,,,,,${p}-OPEN-grni,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-supplier_advances,${companyId},,supplier_advances,0,0,IRR,,,,,,,,,,,,${p}-OPEN-sa,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-customer_advances,${companyId},,customer_advances,0,0,IRR,,,,,,,,,,,,${p}-OPEN-ca,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-cogs,${companyId},,cogs,0,0,IRR,,,,,,,,,,,,${p}-OPEN-cogs,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-inventory_loss,${companyId},,inventory_loss,0,0,IRR,,,,,,,,,,,,${p}-OPEN-loss,,,,,,,,`,
    `sarv-opening-v1,ledger,${p}-GL-owner_capital,${companyId},,owner_capital,0,1500,IRR,,,,,,,,,,,,${p}-OPEN-cap,,,,,,,,`,
    `sarv-opening-v1,cash,${p}-C1,${companyId},Main Cash,,500,,IRR,,,,,,,,,,,,${p}-CASH-1,,,,,,,,`,
    `sarv-opening-v1,bank,${p}-B1,${companyId},Operating Bank,,1000,,IRR,,,,,,,,,,,,${p}-BANK-1,,BANK-REF-001,,,,,,1000`,
  ].join("\n");
}


const OPENING_COLUMNS =
  "version,kind,id,company_id,name,role,amount,credit,unit,quantity,numerator,denominator,party_id,item_id,warehouse_id,unit_id,base_unit_id,customer,supplier,indivisible,reference,settlement_reference,bank_reference,holder,state,document_date,due_date,as_of,statement_amount".split(
    ",",
  );

function csvCell(value: string): string {
  return /[",\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}

function buildOpeningCsv(
  companyId: string,
  asOfDate: string,
  m: { cash: bigint; bank: bigint; bankName: string; bankRef: string },
): string {
  const p = companyId.slice(0, 8) || "CO";
  const row = (fields: Record<string, string>) =>
    OPENING_COLUMNS.map((c) => csvCell(fields[c] ?? "")).join(",");
  const ledger = (role: string, debit: bigint, credit = 0n) =>
    row({
      version: "sarv-opening-v1",
      kind: "ledger",
      id: `${p}-GL-${role}`,
      company_id: companyId,
      role,
      amount: String(debit),
      credit: String(credit),
      unit: "IRR",
      reference: `${p}-OPEN-${role}`,
    });
  const rows = [
    OPENING_COLUMNS.join(","),
    row({
      version: "sarv-opening-v1",
      kind: "opening",
      id: `${p}-OPEN`,
      company_id: companyId,
      name: "Opening balances",
      as_of: asOfDate,
    }),
    ledger("cash", m.cash),
    ledger("bank", m.bank),
    ...[
      "receivables",
      "inventory",
      "cheques_receivable",
      "payables",
      "cheques_payable",
      "grni",
      "supplier_advances",
      "customer_advances",
      "cogs",
      "inventory_loss",
    ].map((role) => ledger(role, 0n)),
    ledger("owner_capital", 0n, m.cash + m.bank),
  ];
  if (m.cash > 0n)
    rows.push(
      row({
        version: "sarv-opening-v1",
        kind: "cash",
        id: `${p}-C1`,
        company_id: companyId,
        name: "Main Cash",
        amount: String(m.cash),
        unit: "IRR",
        reference: `${p}-CASH-1`,
      }),
    );
  if (m.bank > 0n)
    rows.push(
      row({
        version: "sarv-opening-v1",
        kind: "bank",
        id: `${p}-B1`,
        company_id: companyId,
        name: m.bankName.trim() || "Main Bank",
        amount: String(m.bank),
        unit: "IRR",
        reference: `${p}-BANK-1`,
        bank_reference: m.bankRef.trim() || "MAIN-BANK",
        statement_amount: String(m.bank),
      }),
    );
  return rows.join("\n");
}

function daysBetween(startIso: string, endIso: string): number {
  const s = Date.parse(startIso);
  const e = Date.parse(endIso);
  if (Number.isNaN(s) || Number.isNaN(e) || e < s) return 0;
  return Math.round((e - s) / 86_400_000) + 1;
}

export function BusinessProfileDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [profile, setProfile] = useState<BusinessProfile>(
    () => b.workspace.business_profile ?? emptyBusinessProfile,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const company = b.workspace.companies.find((c) => c.id === b.companyId);

  async function save() {
    if (!b.companyId || busy) return;
    setBusy(true);
    setError("");
    try {
      await command("business_profile", {
        company_id: b.companyId,
        profile,
      });
      await b.refresh();
      toast.success(t("Business profile updated"));
      onClose();
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
      title={t("Business and invoice details")}
      description={
        company?.name ??
        t(
          "Optional details appear on your printed documents. You can update them in Settings.",
        )
      }
      wide
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        {error && <ErrorState error={error} />}
        <BusinessFields value={profile} onChange={setProfile} />
        <div className="form-actions">
          <Button
            type="button"
            variant="outline"
            disabled={busy}
            onClick={onClose}
          >
            {t("Cancel")}
          </Button>
          <Button disabled={busy}>
            {busy ? t("Saving…") : t("Save business profile")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

export function SetupDialog({
  open,
  onClose,
  onCreated,
  newCompany,
  firstRun = false,
  onInitialized,
}: {
  open: boolean;
  newCompany: boolean;
  firstRun?: boolean;
  onClose: () => void;
  onCreated: (id: string) => void;
  onInitialized?: () => void;
}) {
  const t = useTranslation();
  const { workspace, companyId, refresh, unit, numberLocale, locale } =
    useBooks();
  const [defaults] = useState(() =>
    fiscalYear(
      !workspace.companies.length
        ? locale
        : workspace.settings.date_display === "Jalali"
          ? "fa"
          : "en",
      parseDate(today()),
    ),
  );
  const [year, setYear] = useState(defaults.name);
  const [name, setName] = useState("");
  const [preset, setPreset] = useState<string>("retail");
  const [profile, setProfile] = useState(emptyBusinessProfile);
  const [start, setStart] = useState(defaults.start);
  const [end, setEnd] = useState(defaults.end);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const [csv, setCsv] = useState("");
  const [preview, setPreview] = useState<OpeningReview | null>(null);
  const [step, setStep] = useState(0);
  const [bankName, setBankName] = useState("");
  const [bankRef, setBankRef] = useState("");
  const [bankBalance, setBankBalance] = useState("");
  const [cashBalance, setCashBalance] = useState("");
  const [key] = useState(uid);
  const needsOpening = !newCompany && !!companyId && !workspace.initialized;

  const businessPresets = [
    {
      id: "retail",
      icon: Storefront,
      label: "Retail & Shop",
      desc: "Sales, inventory, point-of-sale invoices",
      prefix: "INV",
    },
    {
      id: "wholesale",
      icon: Truck,
      label: "Wholesale & Distribution",
      desc: "Bulk orders, warehouses, customer credit",
      prefix: "WHS",
    },
    {
      id: "services",
      icon: Briefcase,
      label: "Services & Agency",
      desc: "Quotations, project billing, expenses",
      prefix: "SRV",
    },
    {
      id: "manufacturing",
      icon: Factory,
      label: "Manufacturing & Workshop",
      desc: "Raw materials, stock transfers, production",
      prefix: "MFG",
    },
  ] as const;

  const periodDays = daysBetween(start, end);

  const bankIrr = toIrr(bankBalance, unit);
  const cashIrr = toIrr(cashBalance, unit);
  const hasMoney = bankIrr + cashIrr > 0n;

  async function recordMoney() {
    setBusy(true);
    setError("");
    try {
      const asOf =
        workspace.periods.find((p) => p.company_id === companyId)
          ?.start_date ?? today();
      const text = buildOpeningCsv(companyId, asOf, {
        cash: cashIrr,
        bank: bankIrr,
        bankName,
        bankRef,
      });
      const review = await command<OpeningReview>("opening_csv_preview", {
        company_id: companyId,
        csv: text,
        request_key: key,
      });
      if (!review.balanced || !review.reconciled)
        throw new Error(t("Opening balances did not reconcile"));
      await command("opening_csv_post", {
        company_id: companyId,
        csv: text,
        request_key: key,
        expected_generation: review._generation,
      });
      const details = [bankName.trim(), bankRef.trim()]
        .filter(Boolean)
        .join(" · ");
      if (details && !workspace.business_profile?.bank_details)
        await command("business_profile", {
          company_id: companyId,
          profile: {
            ...(workspace.business_profile ?? emptyBusinessProfile),
            bank_details: details,
          },
        });
      await refresh();
      toast.success(t("Opening recorded"));
      if (onInitialized) onInitialized();
      else onClose();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

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
        if (onInitialized) onInitialized();
        else onClose();
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
      wide
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
          {[
            "Company",
            "Business details",
            "Fiscal period",
            "Review",
            "Opening balances",
          ].map((label, index) => {
            const current = needsOpening ? 4 : step;
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
          })}
        </ol>
      )}
      <div className="setup-stage" key={needsOpening ? "opening" : step}>
        {firstRun && !needsOpening && (
          <div className="setup-step-heading">
            <h2>
              {t(
                step === 0
                  ? "What’s your business called?"
                  : step === 1
                    ? "Business and invoice details"
                    : step === 2
                      ? "Choose your financial year"
                      : "Ready to create your books?",
              )}
            </h2>
            <p>
              {t(
                step === 0
                  ? "This name identifies your company in Sarv."
                  : step === 1
                    ? "Optional details appear on your printed documents. You can update them in Settings."
                    : step === 2
                      ? "We’ve selected the current year for your language. Adjust the dates if your business uses a different period."
                      : "Check your details. You’ll choose your opening balances next.",
              )}
            </p>
          </div>
        )}
        {error && <ErrorState error={error} />}
        {needsOpening ? (
          <div className="opening-wizard">
            <div className="opening-paths">
              <div
                data-enter-scope
                className={`opening-path-card ${!csv ? "opening-path-active" : ""}`}
              >
                <div className="opening-path-head">
                  <div className="opening-path-icon">
                    <Sparkle aria-hidden="true" />
                  </div>
                  <div>
                    <div className="opening-path-title-row">
                      <h3>{t("Your starting position")}</h3>
                      <span className="setup-badge-positive">
                        {t("Recommended")}
                      </span>
                    </div>
                    <p>
                      {t(
                        "Start with a clean ledger. You can add customers, products, purchases, and bank movements right away.",
                      )}
                    </p>
                  </div>
                </div>
                {!csv && (
                  <>
                    <div className="money-today">
                      <h4>{t("Where is your money today?")}</h4>
                      <p>
                        {t(
                          "Add your main bank account and any cash in the till. Leave both empty if you are starting from nothing.",
                        )}
                      </p>
                      <div className="money-today-grid">
                        <Field label={t("Main bank account name")}>
                          <Input
                            value={bankName}
                            placeholder={t("e.g. Mellat business account")}
                            onChange={(e) => setBankName(e.target.value)}
                          />
                        </Field>
                        <Field label={t("Account or card number (optional)")}>
                          <Input
                            dir="ltr"
                            inputMode="numeric"
                            value={bankRef}
                            onChange={(e) => setBankRef(e.target.value)}
                          />
                        </Field>
                        <Field
                          label={`${t("Bank balance")} · ${unit === "Rial" ? "IRR" : t("toman")}`}
                        >
                          <Input
                            dir="ltr"
                            inputMode="numeric"
                            value={bankBalance}
                            placeholder="0"
                            onChange={(e) => setBankBalance(e.target.value)}
                          />
                        </Field>
                        <Field
                          label={`${t("Cash on hand")} · ${unit === "Rial" ? "IRR" : t("toman")}`}
                        >
                          <Input
                            dir="ltr"
                            inputMode="numeric"
                            value={cashBalance}
                            placeholder="0"
                            onChange={(e) => setCashBalance(e.target.value)}
                          />
                        </Field>
                      </div>
                    </div>
                    {!hasMoney && (
                      <label className="checkbox-label opening-confirm-box">
                        <Checkbox
                          checked={confirmed}
                          onCheckedChange={(checked) =>
                            setConfirmed(checked === true)
                          }
                        />
                        <span>
                          {t(
                            "I confirm this is a fresh company with zero balances, no stock, and no outstanding receivables or payables.",
                          )}
                        </span>
                      </label>
                    )}
                    <div className="form-actions">
                      <Button variant="outline" onClick={onClose}>
                        {t("Cancel")}
                      </Button>
                      <Button
                        data-enter-submit
                        disabled={(!hasMoney && !confirmed) || busy}
                        onClick={() =>
                          hasMoney
                            ? void recordMoney()
                            : act("initialize_zero", {
                                company_id: companyId,
                                source_id: key,
                                request_key: key,
                                confirmed: true,
                              })
                        }
                      >
                        {busy
                          ? t("Recording…")
                          : hasMoney
                            ? t("Record my opening balances")
                            : t("Start with zero balances")}
                      </Button>
                    </div>
                  </>
                )}
              </div>

              <div
                className={`opening-path-card ${csv ? "opening-path-active" : ""}`}
              >
                <div className="opening-path-head">
                  <div className="opening-path-icon opening-path-icon-muted">
                    <FileCsv aria-hidden="true" />
                  </div>
                  <div className="flex-1">
                    <div className="opening-path-title-row">
                      <h3>{t("Import opening CSV")}</h3>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          const periodStart =
                            workspace.periods.find(
                              (p) => p.company_id === companyId,
                            )?.start_date ?? today();
                          setCsv(buildSampleOpeningCsv(companyId, periodStart));
                          setPreview(null);
                          toast.info(
                            t(
                              "Sample opening CSV loaded. Review the rows and click Validate opening.",
                            ),
                          );
                        }}
                      >
                        {t("Load sample CSV")}
                      </Button>
                    </div>
                    <p>{t("For migrating existing books")}</p>
                  </div>
                </div>
                <Field
                  label={t("Opening CSV")}
                  hint={t(
                    "Strict core opening CSV; amounts in integer IRR. Opening import documentation applies.",
                  )}
                >
                  <Textarea
                    rows={5}
                    value={csv}
                    onChange={(e) => {
                      setCsv(e.target.value);
                      setPreview(null);
                    }}
                    placeholder={t("Paste opening CSV…")}
                  />
                </Field>
                {csv && (
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
                      disabled={
                        busy || !preview?.balanced || !preview?.reconciled
                      }
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
                )}
              </div>
            </div>

            {preview && (
              <div className="opening-preview-block">
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
              </div>
            )}
          </div>
        ) : (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (firstRun && step < 3) {
                setStep(step + 1);
                return;
              }
              void act("setup_company", {
                name,
                fiscal_name: year,
                business_profile: profile,
                start_date: start,
                end_date: end,
                timezone: "AsiaTehran",
                ...(firstRun ? { locale } : {}),
              });
            }}
          >
            {firstRun && step === 3 ? (
              <div className="setup-review-container">
                <dl className="setup-review">
                  {[
                    ["Company name", name, 0],
                    ["Business phone", profile.phone || "—", 1],
                    ["Business address", profile.address || "—", 1],
                    [
                      "Registration / national ID",
                      profile.registration_id || "—",
                      1,
                    ],
                    ["Tax / economic ID", profile.tax_id || "—", 1],
                    [
                      "Invoice reference prefix",
                      profile.invoice_prefix || "INV",
                      1,
                    ],
                    ["Fiscal period", year, 2],
                    ["Start date", formatCalendarDate(start, locale), 2],
                    ["End date", formatCalendarDate(end, locale), 2],
                  ].map(([label, value, targetStep]) => (
                    <div key={String(label)} className="setup-review-row">
                      <div>
                        <dt>{t(String(label))}</dt>
                        <dd>{String(value)}</dd>
                      </div>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => setStep(Number(targetStep))}
                      >
                        <Pencil aria-hidden="true" />
                        {t("Edit")}
                      </Button>
                    </div>
                  ))}
                </dl>
                <div className="setup-provision-box">
                  <h4>{t("Included in your books")}</h4>
                  <ul className="setup-provision-list">
                    {[
                      "Standard 5-class chart of accounts",
                      "Receivables, payables & inventory controls",
                      "Cash & bank statement reconciliation",
                      "Immutable journals & audit trail",
                    ].map((item) => (
                      <li key={item}>
                        <Check aria-hidden="true" />
                        <span>{t(item)}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            ) : (
              <>
                <FieldGroup className="form-grid">
                  {(!firstRun || step === 0) && (
                    <div className={firstRun ? "form-full" : ""}>
                      <Field label={t("Company name")}>
                        <Input
                          data-autofocus="true"
                          autoFocus
                          required
                          value={name}
                          maxLength={160}
                          onChange={(e) => setName(e.target.value)}
                          placeholder={t("Your business name")}
                        />
                      </Field>
                    </div>
                  )}
                  {(!firstRun || step === 2) && (
                    <>
                      {firstRun && step === 2 && (
                        <div className="form-full setup-calendar-presets">
                          <span className="setup-preset-label">
                            <CalendarDays aria-hidden="true" />
                            {t("Calendar presets")}
                          </span>
                          <div className="setup-preset-actions">
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              onClick={() => {
                                const fy = fiscalYear("fa", parseDate(today()));
                                setYear(fy.name);
                                setStart(fy.start);
                                setEnd(fy.end);
                              }}
                            >
                              {t("Current Jalali year")}
                            </Button>
                            <Button
                              type="button"
                              variant="outline"
                              size="sm"
                              onClick={() => {
                                const fy = fiscalYear("en", parseDate(today()));
                                setYear(fy.name);
                                setStart(fy.start);
                                setEnd(fy.end);
                              }}
                            >
                              {t("Current Gregorian year")}
                            </Button>
                          </div>
                        </div>
                      )}
                      <Field label={t("Fiscal period")}>
                        <Input
                          required
                          value={year}
                          onChange={(e) => setYear(e.target.value)}
                        />
                      </Field>
                      <Field label={t("Start date")}>
                        <DatePicker
                          locale={locale}
                          calendar={
                            !workspace.companies.length
                              ? locale === "fa"
                                ? "persian"
                                : "gregory"
                              : undefined
                          }
                          value={start}
                          onChange={(value) => {
                            setStart(value);
                            if (end < value) setEnd(value);
                          }}
                        />
                      </Field>
                      <Field label={t("End date")}>
                        <DatePicker
                          locale={locale}
                          calendar={
                            !workspace.companies.length
                              ? locale === "fa"
                                ? "persian"
                                : "gregory"
                              : undefined
                          }
                          value={end}
                          min={start}
                          onChange={setEnd}
                        />
                      </Field>
                      {periodDays > 0 && (
                        <div className="form-full setup-period-summary">
                          {t(
                            "{days} days in fiscal period · Timezone: Asia/Tehran",
                            { days: periodDays },
                          )}
                        </div>
                      )}
                    </>
                  )}
                </FieldGroup>

                {firstRun && step === 0 && (
                  <div className="setup-presets-section">
                    <div className="setup-presets-header">
                      <strong>{t("Business type (optional preset)")}</strong>
                      <p>
                        {t(
                          "Select a profile to pre-fill invoice defaults, or customize everything yourself.",
                        )}
                      </p>
                    </div>
                    <div className="setup-presets-grid">
                      {businessPresets.map((item) => {
                        const Icon = item.icon;
                        const active = preset === item.id;
                        return (
                          <button
                            key={item.id}
                            type="button"
                            className={`setup-preset-card ${active ? "setup-preset-active" : ""}`}
                            onClick={() => {
                              setPreset(item.id);
                              setProfile((prev) => ({
                                ...prev,
                                invoice_prefix:
                                  !prev.invoice_prefix ||
                                  ["INV", "WHS", "SRV", "MFG"].includes(
                                    prev.invoice_prefix,
                                  )
                                    ? item.prefix
                                    : prev.invoice_prefix,
                              }));
                            }}
                          >
                            <Icon aria-hidden="true" />
                            <div>
                              <strong>{t(item.label)}</strong>
                              <small>{t(item.desc)}</small>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                    <div className="setup-provision-box">
                      <h4>{t("Included in your books")}</h4>
                      <ul className="setup-provision-list">
                        {[
                          "Standard 5-class chart of accounts",
                          "Receivables, payables & inventory controls",
                          "Cash & bank statement reconciliation",
                          "Immutable journals & audit trail",
                        ].map((item) => (
                          <li key={item}>
                            <Check aria-hidden="true" />
                            <span>{t(item)}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  </div>
                )}
              </>
            )}
            {(!firstRun || step === 1) && (
              <div className={!firstRun ? "mt-5 pt-5 border-t border-border/60" : ""}>
                <BusinessFields value={profile} onChange={setProfile} />
              </div>
            )}
            <div className="form-actions">
              <Button
                type="button"
                variant="outline"
                onClick={
                  firstRun && step > 0 ? () => setStep(step - 1) : onClose
                }
              >
                {t(firstRun && step > 0 ? "Back" : "Cancel")}
              </Button>
              <Button
                disabled={
                  busy ||
                  end < start ||
                  (firstRun && step === 0 && !name.trim())
                }
              >
                {firstRun && step < 3
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

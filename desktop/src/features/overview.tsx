import { useState } from "react";
import { DatePicker } from "../components/ui/date-picker";
import { ArrowUpRightIcon as ArrowUpRight } from "@phosphor-icons/react/dist/csr/ArrowUpRight";
import { MoneyIcon as Banknote } from "@phosphor-icons/react/dist/csr/Money";
import { FileTextIcon as FileText } from "@phosphor-icons/react/dist/csr/FileText";
import { ShoppingBagIcon as ShoppingBag } from "@phosphor-icons/react/dist/csr/ShoppingBag";
import { TrendUpIcon as TrendUp } from "@phosphor-icons/react/dist/csr/TrendUp";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { CheckIcon as Check } from "@phosphor-icons/react/dist/csr/Check";
import { BuildingsIcon as Building2 } from "@phosphor-icons/react/dist/csr/Buildings";
import { BookOpenIcon as BookOpen } from "@phosphor-icons/react/dist/csr/BookOpen";
import { PackageIcon as Package } from "@phosphor-icons/react/dist/csr/Package";
import { UsersIcon as Users } from "@phosphor-icons/react/dist/csr/Users";
import { WarehouseIcon as Warehouse } from "@phosphor-icons/react/dist/csr/Warehouse";
import { SparkleIcon as Sparkle } from "@phosphor-icons/react/dist/csr/Sparkle";
import { CaretDownIcon as CaretDown } from "@phosphor-icons/react/dist/csr/CaretDown";
import { CaretUpIcon as CaretUp } from "@phosphor-icons/react/dist/csr/CaretUp";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command, today, uid } from "../lib/client";
import { Button } from "../components/ui/button";
import { Checkbox } from "../components/ui/checkbox";
import { Modal } from "../components/layout/modal";
import { PageHeader, ErrorState, Loading } from "../components/ui/shared";
import { Money, Status, formatDate } from "../components/finance/money";
import { DataGrid } from "../components/ui/data-grid";
import { BusinessProfileDialog } from "./setup";
import type { Journal } from "../lib/types";

function StarterEssentialsDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const b = useBooks();
  const { t, workspace: w } = b;
  const existingUnits = new Set(
    (w.resources?.units ?? []).map((u) => u.name.toLowerCase()),
  );
  const existingWarehouses = new Set(
    (w.resources?.warehouses ?? []).map((wh) => wh.name.toLowerCase()),
  );

  const unitCatalog = ["Piece", "Box", "Kilogram", "Meter", "Pack"] as const;
  const warehouseCatalog = ["Main warehouse", "Storefront / Shop"] as const;

  const [selectedUnits, setSelectedUnits] = useState<Record<string, boolean>>(
    () =>
      Object.fromEntries(
        unitCatalog.map((u, i) => [
          u,
          i < 3 && !existingUnits.has(t(u).toLowerCase()),
        ]),
      ),
  );
  const [selectedWarehouses, setSelectedWarehouses] = useState<
    Record<string, boolean>
  >(() =>
    Object.fromEntries(
      warehouseCatalog.map((wh, i) => [
        wh,
        i === 0 && !existingWarehouses.has(t(wh).toLowerCase()),
      ]),
    ),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const effectiveDate = (() => {
    const period = w.periods.find((p) => p.id === b.periodId);
    return period
      ? [period.start_date, today(), period.end_date].sort()[1]
      : today();
  })();

  const totalSelected =
    Object.entries(selectedUnits).filter(
      ([u, checked]) => checked && !existingUnits.has(t(u).toLowerCase()),
    ).length +
    Object.entries(selectedWarehouses).filter(
      ([wh, checked]) =>
        checked && !existingWarehouses.has(t(wh).toLowerCase()),
    ).length;

  async function createSelected() {
    if (busy || totalSelected === 0) return;
    setBusy(true);
    setError("");
    try {
      for (const u of unitCatalog) {
        const localized = t(u);
        if (selectedUnits[u] && !existingUnits.has(localized.toLowerCase())) {
          const id = uid();
          await command("resource", {
            kind: "unit",
            id,
            name: localized,
            sku: "",
            unit_id: "",
            company_id: b.companyId,
            period_id: b.periodId,
            source_id: id,
            request_key: id,
            date: effectiveDate,
            default_unit_name: t("Piece"),
            reason: "Register starter unit",
          });
        }
      }
      for (const wh of warehouseCatalog) {
        const localized = t(wh);
        if (
          selectedWarehouses[wh] &&
          !existingWarehouses.has(localized.toLowerCase())
        ) {
          const id = uid();
          await command("resource", {
            kind: "warehouse",
            id,
            name: localized,
            sku: "",
            unit_id: "",
            company_id: b.companyId,
            period_id: b.periodId,
            source_id: id,
            request_key: id,
            date: effectiveDate,
            default_unit_name: t("Piece"),
            reason: "Register starter warehouse",
          });
        }
      }
      await b.refresh();
      toast.success(t("Essentials added to your business"));
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
      title={t("Starter kit: Warehouses & units")}
      description={t(
        "Select standard units and warehouses to add to this business in one click.",
      )}
    >
      {error && <ErrorState error={error} />}
      <div className="starter-kit-sections">
        <div className="starter-kit-group">
          <h3>{t("Default warehouses")}</h3>
          <div className="starter-kit-grid">
            {warehouseCatalog.map((wh) => {
              const localized = t(wh);
              const exists = existingWarehouses.has(localized.toLowerCase());
              return (
                <label
                  key={wh}
                  className={`starter-kit-item ${exists ? "starter-kit-exists" : ""}`}
                >
                  <Checkbox
                    disabled={exists || busy}
                    checked={exists || !!selectedWarehouses[wh]}
                    onCheckedChange={(checked) =>
                      setSelectedWarehouses((prev) => ({
                        ...prev,
                        [wh]: checked === true,
                      }))
                    }
                  />
                  <span>{localized}</span>
                  {exists && <small>{t("Already exists")}</small>}
                </label>
              );
            })}
          </div>
        </div>
        <div className="starter-kit-group">
          <h3>{t("Standard measurement units")}</h3>
          <div className="starter-kit-grid">
            {unitCatalog.map((u) => {
              const localized = t(u);
              const exists = existingUnits.has(localized.toLowerCase());
              return (
                <label
                  key={u}
                  className={`starter-kit-item ${exists ? "starter-kit-exists" : ""}`}
                >
                  <Checkbox
                    disabled={exists || busy}
                    checked={exists || !!selectedUnits[u]}
                    onCheckedChange={(checked) =>
                      setSelectedUnits((prev) => ({
                        ...prev,
                        [u]: checked === true,
                      }))
                    }
                  />
                  <span>{localized}</span>
                  {exists && <small>{t("Already exists")}</small>}
                </label>
              );
            })}
          </div>
        </div>
      </div>
      <div className="form-actions">
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          onClick={onClose}
        >
          {t("Cancel")}
        </Button>
        <Button
          disabled={busy || totalSelected === 0}
          onClick={() => void createSelected()}
        >
          {busy ? t("Saving…") : t("Create selected essentials")}
        </Button>
      </div>
    </Modal>
  );
}

export function Overview({ onDetail }: { onDetail: (j: Journal) => void }) {
  const b = useBooks();
  const { workspace: w, report: r, t } = b;
  const [profileDialog, setProfileDialog] = useState(false);
  const [starterDialog, setStarterDialog] = useState(false);
  const storageKey = `sarv.setupGuide.${b.companyId || "default"}`;
  const [guidePreference, setGuidePreference] = useState<
    "auto" | "open" | "closed"
  >(() => {
    const saved = localStorage.getItem(storageKey);
    return saved === "open" || saved === "closed" ? saved : "auto";
  });

  const metrics = [
    { key: "cash" as const, label: "Cash and bank", icon: Banknote },
    { key: "receivables" as const, label: "Receivables", icon: FileText },
    { key: "payables" as const, label: "Payables", icon: ShoppingBag },
    { key: "profit" as const, label: "Profit this period", icon: TrendUp },
  ];
  const metricValue = (key: (typeof metrics)[number]["key"]) =>
    r ? (key === "profit" ? r.profit_loss.profit : r.position[key]) : null;
  const quickActions = [
    { action: "invoice" as const, label: "New sales invoice", icon: FileText },
    { action: "purchase" as const, label: "New purchase order", icon: ShoppingBag },
    { action: "customer" as const, label: "New customer", icon: Users },
    { action: "item" as const, label: "New product", icon: Package },
  ];

  const hasProfileDetails = Boolean(
    w.business_profile?.phone ||
      w.business_profile?.registration_id ||
      w.business_profile?.tax_id ||
      w.business_profile?.address,
  );
  const warehousesCount = w.resources?.warehouses.length ?? 0;
  const unitsCount = w.resources?.units.length ?? 0;
  const itemsCount = w.resources?.items.length ?? 0;
  const partiesCount = w.resources?.parties.length ?? 0;
  const txCount =
    Number(w.sales_count || 0) +
    Number(w.purchase_count || 0) +
    Number(w.payment_count || 0);

  const setupSteps = [
    {
      id: "profile",
      icon: Building2,
      title: "Business identity & invoice profile",
      description:
        "Phone, registration/tax IDs, address, and default invoice prefix for printed PDFs.",
      done: Boolean(b.companyId) && (hasProfileDetails || w.initialized),
      badge:
        w.business_profile?.invoice_prefix ||
        w.companies.find((c) => c.id === b.companyId)?.name ||
        t("Ready"),
      actions: [
        {
          label: "Edit business profile",
          primary: false,
          onClick: () => setProfileDialog(true),
        },
      ],
    },
    {
      id: "opening",
      icon: BookOpen,
      title: "Opening balances",
      description:
        "Confirm zero starting balances or validate an opening CSV import to unlock posting.",
      done: w.initialized,
      badge: w.initialized ? t("Balanced") : t("Action needed"),
      actions: !w.initialized
        ? [
            {
              label: "Configure opening balances",
              primary: true,
              onClick: b.openSetup,
            },
          ]
        : [],
    },
    {
      id: "inventory-base",
      icon: Warehouse,
      title: "Warehouses & measurement units",
      description:
        "Define where stock is held and how items are counted (e.g., Piece, Box, Kg).",
      done: warehousesCount > 0 && unitsCount > 0,
      badge: `${new Intl.NumberFormat(b.numberLocale === "fa" ? "fa-IR" : "en-US").format(warehousesCount)} · ${new Intl.NumberFormat(b.numberLocale === "fa" ? "fa-IR" : "en-US").format(unitsCount)}`,
      actions: [
        {
          label: "Quick-add essentials",
          primary: warehousesCount === 0 && unitsCount === 0 && w.initialized,
          onClick: () => {
            if (!w.initialized) {
              toast.info(
                t(
                  "Confirm your starting balances first. New shops can start with zero balances.",
                ),
              );
              b.openSetup();
              return;
            }
            setStarterDialog(true);
          },
        },
        {
          label: "Add warehouse",
          primary: false,
          onClick: () => b.openResource("warehouse"),
        },
        {
          label: "Add unit",
          primary: false,
          onClick: () => b.openResource("unit"),
        },
      ],
    },
    {
      id: "catalog",
      icon: Package,
      title: "Products & price catalog",
      description:
        "Register your goods with default purchase/selling prices and low-stock alerts.",
      done: itemsCount > 0,
      badge: `${new Intl.NumberFormat(b.numberLocale === "fa" ? "fa-IR" : "en-US").format(itemsCount)} ${t("Products")}`,
      actions: [
        {
          label: "Add product",
          primary:
            w.initialized &&
            warehousesCount > 0 &&
            unitsCount > 0 &&
            itemsCount === 0,
          onClick: () => b.openResource("item"),
        },
      ],
    },
    {
      id: "transactions",
      icon: Users,
      title: "Contacts & first transaction",
      description:
        "Add your first customer or supplier and issue a quotation, sales invoice, or purchase order.",
      done: partiesCount > 0 && txCount > 0,
      badge: `${new Intl.NumberFormat(b.numberLocale === "fa" ? "fa-IR" : "en-US").format(partiesCount)} ${t("Contacts")}`,
      actions: [
        {
          label: "Add customer",
          primary: w.initialized && itemsCount > 0 && partiesCount === 0,
          onClick: () => b.openResource("customer"),
        },
        {
          label: "Add supplier",
          primary: false,
          onClick: () => b.openResource("vendor"),
        },
        {
          label: "Create sales invoice",
          primary: w.initialized && partiesCount > 0 && txCount === 0,
          onClick: () => b.runAction("invoice"),
        },
        {
          label: "Create purchase order",
          primary: false,
          onClick: () => b.runAction("purchase"),
        },
      ],
    },
  ];

  const completedSteps = setupSteps.filter((s) => s.done).length;
  const allComplete = completedSteps === setupSteps.length;
  const guideExpanded =
    guidePreference === "open" ||
    (guidePreference === "auto" && !allComplete);

  function toggleGuide() {
    const next = guideExpanded ? "closed" : "open";
    setGuidePreference(next);
    localStorage.setItem(storageKey, next);
  }

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
        <DatePicker
          aria-label={t("Overview cutoff date")}
          value={b.to}
          onChange={(value) => b.setRange(b.from, value)}
        />
        <span>{b.unit === "Rial" ? "IRR" : t("toman")}</span>
      </div>
      {b.reportError && <ErrorState error={b.reportError} />}
      {w.initialized && b.companyId && (
        <div className="quick-actions" aria-label={t("Quick actions")}>
          {quickActions.map(({ action, label, icon: Icon }) => (
            <Button
              key={action}
              variant="outline"
              className="quick-action"
              onClick={() => b.runAction(action)}
            >
              <Icon />
              {t(label)}
            </Button>
          ))}
          <Button
            variant="outline"
            className="quick-action"
            onClick={() => b.navigate("payments")}
          >
            <Banknote />
            {t("Record a payment")}
          </Button>
        </div>
      )}
      <div className="overview-metrics">
        {metrics.map(({ key, label, icon: Icon }) => (
          <div className="metric" key={key} data-metric={key}>
            <div className="metric-label">
              <span className="metric-icon">
                <Icon />
              </span>
              {t(label)}
            </div>
            <div className="metric-value">
              {!b.companyId ? (
                "—"
              ) : r ? (
                <Money
                  value={metricValue(key) ?? "0"}
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

      {b.companyId && (
        <section
          className={`setup-launchpad ${guideExpanded ? "setup-launchpad-open" : "setup-launchpad-collapsed"}`}
          aria-label={t("Business setup guide")}
        >
          <div className="setup-launchpad-header">
            <div className="setup-launchpad-title-group">
              <div className="setup-launchpad-icon" aria-hidden="true">
                <Sparkle />
              </div>
              <div>
                <div className="setup-launchpad-heading-row">
                  <h2>{t("Business setup guide")}</h2>
                  <span className="setup-launchpad-counter">
                    {t("{done} of {total} completed", {
                      done: completedSteps,
                      total: setupSteps.length,
                    })}
                  </span>
                </div>
                {guideExpanded && (
                  <p>
                    {t(
                      "Complete these steps to get your accounting workspace, catalog, and invoicing ready.",
                    )}
                  </p>
                )}
              </div>
            </div>
            <div className="setup-launchpad-controls">
              <div
                className="setup-launchpad-bar"
                role="progressbar"
                aria-valuenow={completedSteps}
                aria-valuemin={0}
                aria-valuemax={setupSteps.length}
              >
                <span
                  style={{
                    width: `${Math.round((completedSteps / setupSteps.length) * 100)}%`,
                  }}
                />
              </div>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={toggleGuide}
              >
                {guideExpanded ? (
                  <>
                    {t("Hide guide")}
                    <CaretUp />
                  </>
                ) : (
                  <>
                    {t("Setup guide")}
                    <CaretDown />
                  </>
                )}
              </Button>
            </div>
          </div>

          {guideExpanded && (
            <ol className="setup-launchpad-list">
              {setupSteps.map((step, index) => {
                const StepIcon = step.icon;
                return (
                  <li
                    key={step.id}
                    className={`setup-launchpad-item ${step.done ? "setup-launchpad-done" : ""}`}
                  >
                    <div className="setup-launchpad-step-mark" aria-hidden="true">
                      {step.done ? (
                        <Check />
                      ) : (
                        new Intl.NumberFormat(
                          b.numberLocale === "fa" ? "fa-IR" : "en-US",
                        ).format(index + 1)
                      )}
                    </div>
                    <div className="setup-launchpad-body">
                      <div className="setup-launchpad-row-top">
                        <div className="setup-launchpad-step-title">
                          <StepIcon aria-hidden="true" />
                          <strong>{t(step.title)}</strong>
                        </div>
                        <span
                          className={`setup-launchpad-badge ${step.done ? "setup-badge-positive" : ""}`}
                        >
                          {step.badge}
                        </span>
                      </div>
                      <p>{t(step.description)}</p>
                      {step.actions.length > 0 && (
                        <div className="setup-launchpad-actions">
                          {step.actions.map((act) => (
                            <Button
                              key={act.label}
                              type="button"
                              size="sm"
                              variant={act.primary ? "default" : "outline"}
                              onClick={act.onClick}
                            >
                              {t(act.label)}
                            </Button>
                          ))}
                        </div>
                      )}
                    </div>
                  </li>
                );
              })}
            </ol>
          )}
        </section>
      )}

      {w.initialized && (
        <div className="overview-summary">
          <Button variant="ghost" onClick={() => b.navigate("sales")}>
            {t("Drafts")}
            <strong>{w.drafts.length}</strong>
          </Button>
          <Button variant="ghost" onClick={() => b.navigate("accounting")}>
            {t("Journals")}
            <strong>{w.journal_count}</strong>
          </Button>
          <Button variant="ghost" onClick={() => b.navigate("reports")}>
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
          </Button>
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
                <Button
                  variant="link"
                  className="text-link h-auto justify-start p-0"
                  onClick={() => onDetail(row.original)}
                >
                  #{row.original.id.slice(0, 8)}
                </Button>
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
      {profileDialog && (
        <BusinessProfileDialog
          open={profileDialog}
          onClose={() => setProfileDialog(false)}
        />
      )}
      {starterDialog && (
        <StarterEssentialsDialog
          open={starterDialog}
          onClose={() => setStarterDialog(false)}
        />
      )}
      {b.companyId && !r && !b.reportError && <Loading />}
    </>
  );
}

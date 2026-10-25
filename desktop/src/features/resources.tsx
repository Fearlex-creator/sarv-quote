import { TabHint } from "../components/ui/tab-hint";
import { FieldGroup } from "../components/ui/field";
import { ContactFields } from "../components/finance/contact-fields";
import { ProductFields } from "../components/finance/product-fields";
import { estimatedPricing, formatPercent } from "../lib/product-pricing";
import { emptyProductProfile, emptyPartyProfile } from "../lib/types";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { toast } from "sonner";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import { useBooks } from "../app/books";
import { command, uid, today } from "../lib/client";
import {
  PageHeader,
  Field,
  ErrorState,
  Note,
  Loading,
} from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { Modal } from "../components/layout/modal";
import { DataGrid } from "../components/ui/data-grid";
import { Money, formatQuantity } from "../components/finance/money";
import type { ResourceKind } from "../lib/shortcuts";
export function ResourceDialog({
  kind,
  onClose,
  onCreated,
}: {
  kind: ResourceKind;
  onClose: () => void;
  onCreated?: (id: string) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [name, setName] = useState("");
  const [productProfile, setProductProfile] = useState(emptyProductProfile);
  const [profile, setProfile] = useState(emptyPartyProfile);
  const contact = kind === "customer" || kind === "vendor";
  const [sku, setSku] = useState("");
  const [unit, setUnit] = useState(b.workspace.resources?.units[0]?.id ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [key] = useState(uid);
  async function save() {
    setBusy(true);
    setError("");
    try {
      if (
        contact &&
        ![
          name,
          profile.first_name,
          profile.last_name,
          profile.organization,
        ].some((v) => v.trim())
      )
        throw Error(t("Enter a name or company name for this contact."));
      await command("resource", {
        kind,
        id: key,
        name: contact
          ? name.trim() ||
            [profile.first_name, profile.last_name].filter(Boolean).join(" ") ||
            profile.organization
          : name,
        ...(contact ? { profile } : {}),
        ...(kind === "item" ? { product_profile: productProfile } : {}),
        sku,
        unit_id: unit,
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: key,
        request_key: key,
        date: (() => {
          const period = b.workspace.periods.find((p) => p.id === b.periodId);
          return period
            ? [period.start_date, today(), period.end_date].sort()[1]
            : today();
        })(),
        default_unit_name: t("Piece"),
        reason: `Register ${kind}`,
      });
      await b.refresh();
      onCreated?.(key);
      toast.success(t(`${kind} created`));
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
      title={t(kind === "item" ? "New product" : `New ${kind}`)}
      description={t(
        "Add a record to this business. You can use it immediately in sales and purchases.",
      )}
      wide={contact || kind === "item"}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
      >
        {error && <ErrorState error={error} />}
        <FieldGroup className="form-grid">
          <Field label={b.t("Name")}>
            <Input
              required={!contact}
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={160}
            />
          </Field>
          {kind === "item" && (
            <>
              <Field
                label={t("Product code (optional)")}
                hint={t("Leave blank to generate a unique product code.")}
              >
                <Input value={sku} onChange={(e) => setSku(e.target.value)} />
              </Field>
              <Field label={t("Base unit")}>
                <NativeSelect
                  value={unit}
                  onChange={(e) => setUnit(e.target.value)}
                >
                  <NativeSelectOption value="">
                    {t(
                      b.workspace.resources?.units.length
                        ? "Use default unit"
                        : "Piece (create automatically)",
                    )}
                  </NativeSelectOption>
                  {b.workspace.resources?.units.map((x) => (
                    <NativeSelectOption key={x.id} value={x.id}>
                      {x.name}
                    </NativeSelectOption>
                  ))}
                </NativeSelect>
              </Field>
            </>
          )}
        </FieldGroup>
        {contact && <ContactFields value={profile} onChange={setProfile} />}
        {kind === "item" && (
          <ProductFields value={productProfile} onChange={setProductProfile} />
        )}
        <div className="form-actions">
          <Button variant="outline" type="button" onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button disabled={busy}>
            {busy ? t("Saving…") : t("Create record")}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
export function Contacts() {
  const b = useBooks();
  const [tab, setTab] = useState("Customers");
  return (
    <Tabs value={tab} onValueChange={setTab} dir={b.locale === "fa" ? "rtl" : "ltr"}>
      <TabsList>
        {["Customers", "Suppliers"].map((x) => (
          <TabsTrigger key={x} value={x}>
            {b.t(x)}
          </TabsTrigger>
        ))}
      </TabsList>
      <TabHint>
        {tab === "Customers"
          ? "People and companies you sell to."
          : "People and companies you buy from."}
      </TabHint>
      <Customers vendor={tab === "Suppliers"} />
    </Tabs>
  );
}
export function Customers({ vendor = false }: { vendor?: boolean }) {
  const t = useTranslation();
  const b = useBooks();
  const parties =
    b.workspace.resources?.parties.filter((x) =>
      vendor ? x.supplier : x.customer,
    ) ?? [];
  const balance = (id: string) =>
    b.report?.reconciliation
      ? (b.report.party_balances[`${vendor ? "vendor" : "customer"}:${id}`] ??
        "0")
      : undefined;
  return (
    <>
      <PageHeader
        title={b.t(vendor ? "Vendors" : "Customers")}
        actions={
          <Button
            onClick={() => b.openResource(vendor ? "vendor" : "customer")}
          >
            <Plus />
            {vendor ? t("New vendor") : b.t("New customer")}
          </Button>
        }
      />
      {!b.workspace.initialized && (
        <Note>
          {t(
            "Confirm your starting balances first. New shops can start with zero balances.",
          )}
        </Note>
      )}
      <section className="panel">
        <DataGrid
          data={parties}
          id={(x) => x.id}
          searchText={(x) =>
            x.name +
            " " +
            Object.values(b.workspace.party_profiles?.[x.id] ?? {}).join(" ")
          }
          empty={t("No contacts")}
          columns={[
            {
              header: b.t("Name"),
              accessorKey: "name",
              cell: ({ row }) => (
                <Button
                  variant="link"
                  className="text-link h-auto justify-start p-0"
                  onClick={() => b.openContact(row.original.id)}
                >
                  {row.original.name}
                </Button>
              ),
            },
            {
              header: t("Phone"),
              cell: ({ row }) =>
                b.workspace.party_profiles?.[row.original.id]?.mobile ||
                b.workspace.party_profiles?.[row.original.id]?.phone ||
                "—",
            },
            {
              header: t("Email"),
              cell: ({ row }) =>
                b.workspace.party_profiles?.[row.original.id]?.email || "—",
            },
            {
              header: t("Role"),
              cell: () => (vendor ? t("Supplier") : t("Customer")),
            },
            {
              header: `${t("Outstanding balance")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
              meta: { numeric: true },
              cell: ({ row }) =>
                balance(row.original.id) !== undefined ? (
                  <Money
                    value={balance(row.original.id)!}
                    unit={b.unit}
                    locale={b.numberLocale}
                  />
                ) : (
                  <span className="muted">{t("Report unavailable")}</span>
                ),
            },
          ]}
        />
      </section>
    </>
  );
}

export function Inventory() {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Stock");
  const r = b.workspace.resources;
  const stockFor = (id: string) =>
    b.report?.reconciliation?.stock
      .filter((s) => s.item_id === id)
      .reduce((sum, s) => sum + BigInt(s.quantity_thousandths), 0n);
  return (
    <Tabs
      value={tab}
      onValueChange={setTab}
      dir={b.locale === "fa" ? "rtl" : "ltr"}
    >
      <PageHeader
        title={b.t("Inventory")}
        actions={
          <div className="toolbar-actions">
            <Button onClick={() => b.openResource("item")}>
              <Plus data-icon="inline-start" />
              {t("New product")}
            </Button>
            {tab === "Stock" ? (
              <Button variant="outline" onClick={() => b.runAction("transfer")}>
                {t("Transfer stock")}
              </Button>
            ) : tab !== "Products" ? (
              <Button
                onClick={() =>
                  b.openResource(tab === "Warehouses" ? "warehouse" : "unit")
                }
              >
                <Plus />
                {t(
                  tab === "Products"
                    ? "New product"
                    : tab === "Warehouses"
                      ? "New warehouse"
                      : "New unit",
                )}
              </Button>
            ) : null}
          </div>
        }
      />
      <TabsList>
        {["Stock", "Products", "Warehouses", "Units"].map((x) => (
          <TabsTrigger key={x} value={x}>
            {b.t(x)}
          </TabsTrigger>
        ))}
      </TabsList>
      <TabHint>
        {
          {
            Stock: "What is on your shelves right now.",
            Products: "Your catalog, costs and selling prices.",
            Warehouses: "Where your stock is kept.",
            Units: "How items are counted, like piece, box or kg.",
          }[tab] ?? ""
        }
      </TabHint>
      <TabsContent value={tab}>
        <div className="panel">
          {tab === "Stock" ? (
            b.companyId && !b.report && !b.reportError ? (
              <Loading />
            ) : (
              <DataGrid
                data={b.report?.reconciliation?.stock ?? []}
                id={(x) => x.item_id + x.warehouse_id}
                empty={t("No stock")}
                columns={[
                  {
                    header: t("Product"),
                    cell: ({ row }) => (
                      <Button
                        variant="link"
                        onClick={() => b.openProduct(row.original.item_id)}
                      >
                        {r?.items.find((x) => x.id === row.original.item_id)
                          ?.name ?? row.original.item_id}
                      </Button>
                    ),
                  },
                  {
                    header: t("Warehouse"),
                    cell: ({ row }) =>
                      r?.warehouses.find(
                        (x) => x.id === row.original.warehouse_id,
                      )?.name ?? row.original.warehouse_id,
                  },
                  {
                    header: t("Quantity"),
                    meta: { numeric: true },
                    cell: ({ row }) =>
                      formatQuantity(
                        row.original.quantity_thousandths,
                        b.numberLocale,
                      ),
                  },
                  {
                    header: `${t("Carried value")} · ${b.unit === "Rial" ? "IRR" : "toman"}`,
                    meta: { numeric: true },
                    cell: ({ row }) => (
                      <Money
                        value={row.original.value}
                        unit={b.unit}
                        locale={b.numberLocale}
                      />
                    ),
                  },
                ]}
              />
            )
          ) : tab === "Products" ? (
            <DataGrid
              data={r?.items ?? []}
              id={(x) => x.id}
              searchText={(x) =>
                x.name +
                x.sku +
                Object.values(b.workspace.product_profiles?.[x.id] ?? {}).join(
                  " ",
                )
              }
              empty={t("No products")}
              columns={[
                { header: "SKU", accessorKey: "sku" },
                {
                  header: t("Stock on hand"),
                  cell: ({ row }) => {
                    const q = stockFor(row.original.id);
                    return q === undefined
                      ? "—"
                      : formatQuantity(q.toString(), b.numberLocale);
                  },
                },
                {
                  header: t("Stock alert"),
                  cell: ({ row }) => {
                    const p = b.workspace.product_profiles?.[row.original.id],
                      q = stockFor(row.original.id);
                    return p &&
                      q !== undefined &&
                      BigInt(p.minimum_stock_thousandths) > 0n &&
                      q <= BigInt(p.minimum_stock_thousandths) ? (
                      <span className="text-destructive">{t("Low stock")}</span>
                    ) : (
                      "—"
                    );
                  },
                },
                {
                  header: t("Product"),
                  cell: ({ row }) => (
                    <Button
                      variant="link"
                      onClick={() => b.openProduct(row.original.id)}
                    >
                      {row.original.name}
                    </Button>
                  ),
                },
                {
                  header: `${t("Purchase price")} · ${b.unit === "Rial" ? "IRR" : t("toman")}`,
                  cell: ({ row }) => {
                    const price =
                      b.workspace.product_profiles?.[row.original.id]
                        ?.purchase_price;
                    return price == null ? (
                      "—"
                    ) : (
                      <Money value={price} unit={b.unit} />
                    );
                  },
                },
                {
                  header: `${t("Selling price")} · ${b.unit === "Rial" ? "IRR" : t("toman")}`,
                  cell: ({ row }) => {
                    const price =
                      b.workspace.product_profiles?.[row.original.id]
                        ?.sale_price;
                    return price == null ? (
                      "—"
                    ) : (
                      <Money value={price} unit={b.unit} />
                    );
                  },
                },
                {
                  header: t("Estimated gross margin"),
                  cell: ({ row }) => {
                    const p = b.workspace.product_profiles?.[row.original.id];
                    return p
                      ? formatPercent(
                          estimatedPricing(p)?.margin ?? null,
                          b.numberLocale,
                        )
                      : "—";
                  },
                },
                {
                  header: t("Category"),
                  cell: ({ row }) =>
                    b.workspace.product_profiles?.[row.original.id]?.category ||
                    "—",
                },
                {
                  header: t("Base unit"),
                  cell: ({ row }) =>
                    r?.units.find((x) => x.id === row.original.base_unit_id)
                      ?.name ?? "—",
                },
              ]}
            />
          ) : (
            <DataGrid
              data={tab === "Units" ? (r?.units ?? []) : (r?.warehouses ?? [])}
              id={(x) => x.id}
              searchText={(x) => x.name}
              empty={t("No records")}
              columns={[{ header: t("Name"), accessorKey: "name" }]}
            />
          )}
        </div>
      </TabsContent>
    </Tabs>
  );
}

import { FieldGroup } from "../components/ui/field";
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
import { Money, formatQuantity, formatDate } from "../components/finance/money";
import type { Party } from "../lib/types";
export function ResourceDialog({
  kind,
  onClose,
}: {
  kind: "customer" | "vendor" | "item" | "warehouse" | "unit";
  onClose: () => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [name, setName] = useState("");
  const [sku, setSku] = useState("");
  const [unit, setUnit] = useState(b.workspace.resources?.units[0]?.id ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [key] = useState(uid);
  async function save() {
    setBusy(true);
    setError("");
    try {
      await command("resource", {
        kind,
        id: key,
        name,
        sku,
        unit_id: unit,
        company_id: b.companyId,
        period_id: b.periodId,
        source_id: key,
        request_key: key,
        date: today(),
        reason: `Register ${kind}`,
      });
      await b.refresh();
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
      title={t(`New ${kind}`)}
      description={t(
        "This record belongs to the selected company's operational subledger.",
      )}
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
              required
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={160}
            />
          </Field>
          {kind === "item" && (
            <>
              <Field label="SKU">
                <Input
                  required
                  value={sku}
                  onChange={(e) => setSku(e.target.value)}
                />
              </Field>
              <Field label={t("Base unit")}>
                <NativeSelect
                  required
                  value={unit}
                  onChange={(e) => setUnit(e.target.value)}
                >
                  <NativeSelectOption value="">
                    {t("Select unit")}
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
export function Customers({ vendor = false }: { vendor?: boolean }) {
  const t = useTranslation();
  const b = useBooks();
  const [create, setCreate] = useState(false);
  const [selected, setSelected] = useState<Party | null>(null);
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
            disabled={!b.workspace.initialized}
            onClick={() => setCreate(true)}
          >
            <Plus />
            {vendor ? t("New vendor") : b.t("New customer")}
          </Button>
        }
      />
      {!b.workspace.initialized && (
        <Note>{t("Opening balances required")}</Note>
      )}
      <section className="panel">
        <DataGrid
          data={parties}
          id={(x) => x.id}
          searchText={(x) => x.name}
          empty={t("No contacts")}
          columns={[
            {
              header: b.t("Name"),
              accessorKey: "name",
              cell: ({ row }) => (
                <Button
                  variant="link"
                  className="text-link h-auto justify-start p-0"
                  onClick={() => setSelected(row.original)}
                >
                  {row.original.name}
                </Button>
              ),
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
      {create && (
        <ResourceDialog
          kind={vendor ? "vendor" : "customer"}
          onClose={() => setCreate(false)}
        />
      )}
      <Modal
        open={!!selected}
        onOpenChange={(v) => {
          if (!v) setSelected(null);
        }}
        title={selected?.name ?? t("Contact")}
        description={t("Outstanding records for this contact.")}
      >
        <div className="grouped-list contact-details">
          <div className="summary-line">
            <span>
              {t("Outstanding balance ·")}
              {b.unit === "Rial" ? "IRR" : t("toman")}
            </span>
            {selected && balance(selected.id) !== undefined ? (
              <Money
                value={balance(selected.id)!}
                unit={b.unit}
                locale={b.numberLocale}
              />
            ) : (
              "—"
            )}
          </div>
          {(vendor
            ? b.report?.reconciliation?.payables
            : b.report?.reconciliation?.receivables
          )
            ?.filter((x) => (x.party_id ?? x.supplier_id) === selected?.id)
            .map((x) => (
              <div className="summary-line" key={x.id}>
                <span>
                  {x.reference} ·{" "}
                  {formatDate(
                    x.due_date,
                    b.dateLocale,
                    b.workspace.settings.date_display,
                  )}
                </span>
                <Money
                  value={x.remaining}
                  unit={b.unit}
                  locale={b.numberLocale}
                />
              </div>
            ))}
        </div>
      </Modal>
    </>
  );
}
import { TransferDialog } from "./transfer";

export function Inventory() {
  const t = useTranslation();
  const b = useBooks();
  const [tab, setTab] = useState("Stock");
  const [transfer, setTransfer] = useState(false);
  const [create, setCreate] = useState<"item" | "warehouse" | "unit" | null>(
    null,
  );
  const r = b.workspace.resources;
  return (
    <Tabs
      value={tab}
      onValueChange={setTab}
      dir={b.locale === "fa" ? "rtl" : "ltr"}
    >
      <PageHeader
        title={b.t("Inventory")}
        actions={
          tab === "Stock" ? (
            <Button
              disabled={!b.workspace.initialized}
              onClick={() => setTransfer(true)}
            >
              {t("Transfer stock")}
            </Button>
          ) : (
            <Button
              disabled={!b.workspace.initialized}
              onClick={() =>
                setCreate(
                  tab === "Products"
                    ? "item"
                    : tab === "Warehouses"
                      ? "warehouse"
                      : "unit",
                )
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
          )
        }
      />
      <TabsList>
        {["Stock", "Products", "Warehouses", "Units"].map((x) => (
          <TabsTrigger key={x} value={x}>
            {b.t(x)}
          </TabsTrigger>
        ))}
      </TabsList>
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
                    cell: ({ row }) =>
                      r?.items.find((x) => x.id === row.original.item_id)
                        ?.name ?? row.original.item_id,
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
              searchText={(x) => x.name + x.sku}
              empty={t("No products")}
              columns={[
                { header: "SKU", accessorKey: "sku" },
                { header: t("Product"), accessorKey: "name" },
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
        {transfer && <TransferDialog onClose={() => setTransfer(false)} />}
        {create && (
          <ResourceDialog kind={create} onClose={() => setCreate(null)} />
        )}
      </TabsContent>
    </Tabs>
  );
}

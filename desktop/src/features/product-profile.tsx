import { useEffect, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command } from "../lib/client";
import type { ProductDetail, ProductProfile as Profile } from "../lib/types";
import { Modal } from "../components/layout/modal";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import { Button } from "../components/ui/button";
import { Field, Note, Loading, ErrorState } from "../components/ui/shared";
import { DatePicker } from "../components/ui/date-picker";
import { DataGrid } from "../components/ui/data-grid";
import { ProductFields } from "../components/finance/product-fields";
import { AiSuggest } from "../components/finance/ai-suggest";
import { Money, formatDate, formatQuantity } from "../components/finance/money";
import { recordCommand, recordKind, recordData } from "./sales";
import { formatPercent } from "../lib/product-pricing";

export function ProductProfile({
  id,
  onClose,
}: {
  id: string;
  onClose: () => void;
}) {
  const b = useBooks(),
    t = b.t;
  const [data, setData] = useState<ProductDetail | null>(null),
    [profile, setProfile] = useState<Profile | null>(null);
  const [generation, setGeneration] = useState(""),
    [asof, setAsof] = useState(b.to),
    [offset, setOffset] = useState(0),
    [tab, setTab] = useState("details");
  const [dirty, setDirty] = useState(false),
    [discard, setDiscard] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    command<ProductDetail>("product_detail", {
      company_id: b.companyId,
      item_id: id,
      as_of: asof,
      offset: String(offset),
    })
      .then((d) => {
        if (active) {
          setData(d);
          setProfile((p) => p ?? d.profile);
          setGeneration((g) => g || d._generation);
          setError("");
        }
      })
      .catch((e) => {
        if (active) setError(e.message);
      });
    return () => {
      active = false;
    };
  }, [b.companyId, id, asof, offset]);
  async function save() {
    if (!profile) return;
    setBusy(true);
    setError("");
    try {
      await command("product_profile", {
        company_id: b.companyId,
        item_id: id,
        profile,
        expected_generation: generation,
      });
      await b.refresh();
      const detail = await command<ProductDetail>("product_detail", {
        company_id: b.companyId,
        item_id: id,
        as_of: asof,
        offset: String(offset),
      });
      setData(detail);
      setProfile(detail.profile);
      setGeneration(detail._generation);
      setDirty(false);
      toast.success(t("Product updated"));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const close = () => {
    if (busy) return;
    if (dirty) setDiscard(true);
    else onClose();
  };
  const money = (value: string | null) =>
    value === null ? (
      "—"
    ) : (
      <Money value={value} unit={b.unit} locale={b.numberLocale} />
    );
  const party = (id: string) =>
    b.workspace.party_profiles?.[id]?.display_name ||
    b.workspace.resources?.parties.find((p) => p.id === id)?.name ||
    id;
  const kinds: Record<string, string> = {
    Order: "Order",
    Fulfill: "Sales invoice",
    Return: "Return",
    Receive: "Goods receipt",
    Bill: "Purchase bill",
    Transfer: "Stock transfer",
  };
  return (
    <>
      <Modal
        open
        onOpenChange={(v) => {
          if (!v) close();
        }}
        title={data?.item.name ?? t("Product profile")}
        wide
        description={t(
          "Product details, stock costs and performance from posted documents.",
        )}
      >
        {error && <ErrorState error={error} />}
        {!data || !profile ? (
          <Loading shape="form" />
        ) : (
          <>
            <p className="muted">
              {data.item.sku} ·{" "}
              {
                b.workspace.resources?.units.find(
                  (u) => u.id === data.item.base_unit_id,
                )?.name
              }
            </p>
            <div className="overview-metrics finance-metrics">
              {[
                {
                  label: "Stock on hand",
                  value: formatQuantity(
                    data.stats.stock_quantity,
                    b.numberLocale,
                  ),
                },
                {
                  label: "Carried stock value",
                  value: money(data.stats.stock_value),
                },
                {
                  label: "Actual gross profit",
                  value: money(data.stats.gross_profit),
                },
              ].map((m) => (
                <div className="metric" key={m.label}>
                  <div className="metric-label">{t(m.label)}</div>
                  <div className="metric-value">{m.value}</div>
                  {m.label !== "Stock on hand" && (
                    <small>{b.unit === "Rial" ? "IRR" : t("toman")}</small>
                  )}
                </div>
              ))}
            </div>
            {data.stats.low_stock && (
              <Note>
                {t("Low stock: on-hand quantity is at or below your minimum.")}
              </Note>
            )}
            <Tabs
              value={tab}
              onValueChange={setTab}
              dir={b.locale === "fa" ? "rtl" : "ltr"}
            >
              <TabsList className="max-w-full flex-wrap h-auto group-data-horizontal/tabs:h-auto">
                <TabsTrigger value="details">
                  {t("Product details")}
                </TabsTrigger>
                <TabsTrigger value="stock">{t("Stock and costs")}</TabsTrigger>
                <TabsTrigger value="performance">
                  {t("Performance")}
                </TabsTrigger>
                <TabsTrigger value="activity">
                  {t("Linked documents")}
                </TabsTrigger>
              </TabsList>
              <TabsContent value="details">
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void save();
                  }}
                >
                  <ProductFields
                    value={profile}
                    onChange={(p) => {
                      setProfile(p);
                      setDirty(true);
                    }}
                  />
                  {data && (
                    <AiSuggest
                      label={t("Suggest category")}
                      state={{
                        context:
                          "One product in the inventory of a small business, amounts in Iranian rials",
                        name: data.item.name,
                        sku: data.item.sku,
                        brand: profile.brand,
                        existing_category: profile.category,
                        monthly_net_sales: data.stats.net_sales,
                        monthly_sold_quantity: Number(data.stats.sold_quantity),
                        purchase_price: profile.purchase_price,
                        sale_price: profile.sale_price,
                      }}
                      questions={{
                        category: {
                          type: "choice",
                          instructions:
                            "Which merchandising category fits this product best, given all fields?",
                          criteria: {
                            raw_materials:
                              "Inputs and materials consumed to make or assemble other goods",
                            components:
                              "Parts and spare components used with other products",
                            finished_goods:
                              "Complete products sold to customers as-is",
                            consumables:
                              "Frequently repurchased supplies with a short life",
                            equipment:
                              "Durable tools or machines kept and used over time",
                            resale:
                              "Goods bought finished and resold without change",
                          },
                        },
                      }}
                      answerKey="category"
                      labels={{
                        raw_materials: t("Raw materials"),
                        components: t("Components"),
                        finished_goods: t("Finished goods"),
                        consumables: t("Consumables"),
                        equipment: t("Equipment"),
                        resale: t("Resale goods"),
                      }}
                    />
                  )}
                  <div className="form-actions">
                    <Button type="submit" disabled={busy || !dirty}>
                      {t(busy ? "Saving…" : "Save product")}
                    </Button>
                  </div>
                </form>
              </TabsContent>
              <TabsContent value="stock">
                <Field label={t("As of")}>
                  <DatePicker
                    value={asof}
                    onChange={(v) => {
                      setAsof(v);
                      setOffset(0);
                    }}
                  />
                </Field>
                <Note>
                  {t(
                    "Average stock cost is carried value divided by on-hand base units, rounded to the nearest rial. Defaults do not revalue inventory.",
                  )}
                </Note>
                <div className="grouped-list">
                  {[
                    {
                      label: "Average carried cost per unit",
                      value: data.stats.average_stock_cost,
                    },
                    {
                      label: "Last received purchase price",
                      value: data.stats.last_purchase_price,
                    },
                    {
                      label: "Last invoiced net selling price",
                      value: data.stats.last_sale_price,
                    },
                  ].map((m) => (
                    <div className="summary-line px-4 py-3" key={m.label}>
                      <span>{t(m.label)}</span>
                      {money(m.value)}
                    </div>
                  ))}
                </div>
                <DataGrid
                  data={data.stock}
                  id={(r) => r.warehouse_id}
                  empty={t("No stock")}
                  columns={[
                    {
                      header: t("Warehouse"),
                      cell: ({ row }) =>
                        b.workspace.resources?.warehouses.find(
                          (w) => w.id === row.original.warehouse_id,
                        )?.name ?? row.original.warehouse_id,
                    },
                    {
                      header: t("Quantity"),
                      cell: ({ row }) =>
                        formatQuantity(
                          row.original.quantity_thousandths,
                          b.numberLocale,
                        ),
                    },
                    {
                      header: t("Carried value"),
                      cell: ({ row }) => money(row.original.value),
                    },
                  ]}
                />
              </TabsContent>
              <TabsContent value="performance">
                <Field label={t("As of")}>
                  <DatePicker
                    value={asof}
                    onChange={(v) => {
                      setAsof(v);
                      setOffset(0);
                    }}
                  />
                </Field>
                <Note>
                  {t(
                    "Actual gross profit uses posted invoice revenue minus original carried costs, including returns. It excludes operating expenses and tax.",
                  )}
                </Note>
                <div className="grouped-list">
                  {[
                    { label: "Net sales", value: data.stats.net_sales },
                    {
                      label: "Cost of goods sold",
                      value: data.stats.cost_of_goods_sold,
                    },
                    {
                      label: "Actual gross profit",
                      value: data.stats.gross_profit,
                    },
                    {
                      label: "Sales returns value",
                      value: data.stats.returns_value,
                    },
                    {
                      label: "Net received purchase value",
                      value: data.stats.net_purchase_value,
                    },
                  ].map((m) => (
                    <div className="summary-line px-4 py-3" key={m.label}>
                      <span>{t(m.label)}</span>
                      {money(m.value)}
                    </div>
                  ))}
                  {[
                    {
                      label: "Net units sold",
                      value: data.stats.sold_quantity,
                    },
                    {
                      label: "Units returned by customers",
                      value: data.stats.returned_quantity,
                    },
                    {
                      label: "Net units received",
                      value: data.stats.purchased_quantity,
                    },
                    {
                      label: "Units returned to suppliers",
                      value: data.stats.purchase_returns_quantity,
                    },
                  ].map((m) => (
                    <div className="summary-line px-4 py-3" key={m.label}>
                      <span>{t(m.label)}</span>
                      <strong>{formatQuantity(m.value, b.numberLocale)}</strong>
                    </div>
                  ))}
                  <div className="summary-line px-4 py-3">
                    <span>{t("Actual gross margin")}</span>
                    <strong>
                      {formatPercent(
                        BigInt(data.stats.net_sales) > 0n
                          ? (
                              (BigInt(data.stats.gross_profit) * 10000n) /
                              BigInt(data.stats.net_sales)
                            ).toString()
                          : null,
                        b.numberLocale,
                      )}
                    </strong>
                  </div>
                </div>
                <h3>{t("Monthly product performance")}</h3>
                <DataGrid
                  data={data.monthly}
                  id={(r) => r.month}
                  empty={t("No posted sales")}
                  columns={[
                    {
                      header: t("Month"),
                      cell: ({ row }) =>
                        row.original.month.replace(/\d+/g, (n) =>
                          new Intl.NumberFormat(b.numberLocale, {
                            useGrouping: false,
                          }).format(Number(n)),
                        ),
                    },
                    {
                      header: t("Net sales"),
                      cell: ({ row }) => money(row.original.net_sales),
                    },
                    {
                      header: t("Cost of goods sold"),
                      cell: ({ row }) => money(row.original.cost),
                    },
                    {
                      header: t("Actual gross profit"),
                      cell: ({ row }) => money(row.original.profit),
                    },
                  ]}
                />
                <h3>{t("Customers for this product")}</h3>
                <DataGrid
                  data={data.customers}
                  id={(r) => r.id}
                  empty={t("No posted sales")}
                  columns={[
                    {
                      header: t("Customer"),
                      cell: ({ row }) => (
                        <Button
                          variant="link"
                          disabled={dirty}
                          onClick={() => {
                            onClose();
                            b.openContact(row.original.id);
                          }}
                        >
                          {party(row.original.id)}
                        </Button>
                      ),
                    },
                    {
                      header: t("Net units sold"),
                      cell: ({ row }) =>
                        formatQuantity(row.original.quantity, b.numberLocale),
                    },
                    {
                      header: t("Net sales"),
                      cell: ({ row }) => money(row.original.net_sales),
                    },
                    {
                      header: t("Actual gross profit"),
                      cell: ({ row }) => money(row.original.profit),
                    },
                  ]}
                />
                <h3>{t("Suppliers for this product")}</h3>
                <DataGrid
                  data={data.suppliers}
                  id={(r) => r.id}
                  empty={t("No received purchases")}
                  columns={[
                    {
                      header: t("Supplier"),
                      cell: ({ row }) => (
                        <Button
                          variant="link"
                          disabled={dirty}
                          onClick={() => {
                            onClose();
                            b.openContact(row.original.id);
                          }}
                        >
                          {party(row.original.id)}
                        </Button>
                      ),
                    },
                    {
                      header: t("Net units received"),
                      cell: ({ row }) =>
                        formatQuantity(row.original.quantity, b.numberLocale),
                    },
                    {
                      header: t("Net received purchase value"),
                      cell: ({ row }) => money(row.original.value),
                    },
                  ]}
                />
              </TabsContent>
              <TabsContent value="activity">
                <Field label={t("As of")}>
                  <DatePicker
                    value={asof}
                    onChange={(v) => {
                      setAsof(v);
                      setOffset(0);
                    }}
                  />
                </Field>
                <DataGrid
                  data={data.records}
                  id={(r) => r.id}
                  empty={t("No linked documents")}
                  columns={[
                    {
                      header: t("Document"),
                      cell: ({ row }) =>
                        t(
                          kinds[recordKind(row.original)] ??
                            recordKind(row.original),
                        ),
                    },
                    {
                      header: t("Reference"),
                      cell: ({ row }) => (
                        <Button
                          variant="link"
                          disabled={dirty}
                          onClick={() => {
                            onClose();
                            b.openRecord(row.original);
                          }}
                        >
                          {recordCommand(row.original).invoice_reference ||
                            recordCommand(row.original).return_reference ||
                            recordCommand(row.original).reference ||
                            row.original.id}
                        </Button>
                      ),
                    },
                    {
                      header: t("Date"),
                      cell: ({ row }) =>
                        formatDate(
                          recordCommand(row.original).context.date,
                          b.dateLocale,
                          b.workspace.settings.date_display,
                        ),
                    },
                    {
                      header: t("Document total"),
                      cell: ({ row }) =>
                        money(recordData(row.original).total ?? null),
                    },
                  ]}
                />
                <div className="form-actions">
                  <Button
                    variant="outline"
                    disabled={!offset}
                    onClick={() => setOffset(Math.max(0, offset - 100))}
                  >
                    {t("Previous")}
                  </Button>
                  <Button
                    variant="outline"
                    disabled={offset + 100 >= Number(data.record_count)}
                    onClick={() => setOffset(offset + 100)}
                  >
                    {t("Next")}
                  </Button>
                </div>
              </TabsContent>
            </Tabs>
          </>
        )}
      </Modal>
      <Modal
        open={discard}
        onOpenChange={setDiscard}
        title={t("Leave unsaved changes?")}
        description={t("Discard unsaved product details?")}
        confirmation
      >
        <div className="form-actions">
          <Button variant="outline" onClick={() => setDiscard(false)}>
            {t("Keep editing")}
          </Button>
          <Button variant="destructive" onClick={onClose}>
            {t("Discard changes")}
          </Button>
        </div>
      </Modal>
    </>
  );
}

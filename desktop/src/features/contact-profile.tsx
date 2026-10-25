import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command } from "../lib/client";
import { ContactFields } from "../components/finance/contact-fields";
import { AiSuggest } from "../components/finance/ai-suggest";
import { DatePicker } from "../components/ui/date-picker";
import { Input } from "../components/ui/input";
import { Button } from "../components/ui/button";
import { Field, ErrorState, Loading, Note } from "../components/ui/shared";
import { Modal } from "../components/layout/modal";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
import { DataGrid } from "../components/ui/data-grid";
import { Money, formatDate, formatQuantity } from "../components/finance/money";
import { recordCommand, recordData, recordKind } from "./sales";
import type { ContactDetail, PartyProfile } from "../lib/types";

export function ContactProfile({
  id,
  onClose,
}: {
  id: string;
  onClose: () => void;
}) {
  const b = useBooks(),
    t = b.t;
  const [data, setData] = useState<ContactDetail | null>(null),
    [profile, setProfile] = useState<PartyProfile | null>(null);
  const [asof, setAsof] = useState(b.to),
    [offset, setOffset] = useState(0),
    [tab, setTab] = useState("profile");
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [editGeneration, setEditGeneration] = useState("");
  const [dirty, setDirty] = useState(false),
    [discard, setDiscard] = useState(false);
  const load = useCallback(async () => {
    const detail = await command<ContactDetail>("contact_detail", {
      company_id: b.companyId,
      party_id: id,
      as_of: asof,
      offset: String(offset),
    });
    setData(detail);
    return detail;
  }, [b.companyId, id, asof, offset]);
  useEffect(() => {
    let active = true;
    command<ContactDetail>("contact_detail", {
      company_id: b.companyId,
      party_id: id,
      as_of: asof,
      offset: String(offset),
    })
      .then((d) => {
        if (active) {
          setData(d);
          setProfile((p) => p ?? d.profile);
          setEditGeneration((g) => g || d._generation);
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
    if (!profile || !data) return;
    setBusy(true);
    setError("");
    try {
      const next = {
        ...profile,
        display_name:
          profile.display_name.trim() ||
          [profile.first_name, profile.last_name].filter(Boolean).join(" ") ||
          profile.organization ||
          data.party.name,
      };
      await command("party_profile", {
        company_id: b.companyId,
        party_id: id,
        profile: next,
        expected_generation: editGeneration,
      });
      await b.refresh();
      const detail = await load();
      setProfile(detail.profile);
      setEditGeneration(detail._generation);
      setDirty(false);
      toast.success(t("Contact updated"));
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setBusy(false);
    }
  }
  const money = (v: string) => (
    <Money value={v} unit={b.unit} locale={b.numberLocale} />
  );
  const date = (v: string) =>
    formatDate(v, b.dateLocale, b.workspace.settings.date_display);
  const close = () => {
    if (busy) return;
    if (dirty) setDiscard(true);
    else onClose();
  };
  return (
    <>
      <Modal
        open
        onOpenChange={(v) => {
          if (!v) close();
        }}
        title={
          profile?.display_name || data?.party.name || t("Customer profile")
        }
        wide
        description={t(
          "Contact details, linked documents and performance for this business.",
        )}
      >
        {error && (
          <ErrorState
            error={error}
            retry={() => void load().catch((e) => setError(e.message))}
          />
        )}
        {!data || !profile ? (
          <Loading shape="form" />
        ) : (
          <>
            <div className="overview-metrics finance-metrics">
              {(data.party.customer
                ? [
                    { label: "Net sales", value: data.stats.net_sales },
                    {
                      label: "Outstanding balance",
                      value: data.stats.outstanding,
                    },
                    { label: "Overdue", value: data.stats.overdue },
                  ]
                : [
                    {
                      label: "Supplier balances",
                      value: data.payables
                        .reduce((sum, r) => sum + BigInt(r.remaining), 0n)
                        .toString(),
                    },
                  ]
              ).map((m) => (
                <div className="metric" key={m.label}>
                  <div className="metric-label">{t(m.label)}</div>
                  <div className="metric-value">{money(m.value)}</div>
                  <small>{b.unit === "Rial" ? "IRR" : t("toman")}</small>
                </div>
              ))}
            </div>
            <Tabs
              value={tab}
              onValueChange={setTab}
              dir={b.locale === "fa" ? "rtl" : "ltr"}
            >
              <TabsList>
                <TabsTrigger value="profile">
                  {t("Contact details")}
                </TabsTrigger>
                <TabsTrigger value="activity">
                  {t("Linked documents")}
                </TabsTrigger>
                <TabsTrigger
                  value="performance"
                  disabled={!data.party.customer}
                >
                  {t("Performance")}
                </TabsTrigger>
              </TabsList>
              <TabsContent value="profile">
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void save();
                  }}
                >
                  <Field label={t("Name")}>
                    <Input
                      maxLength={160}
                      value={profile.display_name}
                      onChange={(e) => {
                        setProfile({
                          ...profile,
                          display_name: e.target.value,
                        });
                        setDirty(true);
                      }}
                    />
                  </Field>
                  <ContactFields
                    value={profile}
                    onChange={(p) => {
                      setProfile(p);
                      setDirty(true);
                    }}
                  />
                  <AiSuggest
                    label={t("Suggest segment")}
                    state={{
                      context:
                        "One contact of a small business doing double-entry accounting",
                      display_name: profile.display_name,
                      organization: profile.organization,
                      city: profile.city,
                      notes: profile.notes,
                      is_customer: data.party.customer,
                      is_supplier: data.party.supplier,
                      monthly_net_sales: data.stats.net_sales,
                      orders: Number(data.stats.orders),
                      invoices: Number(data.stats.invoices),
                      outstanding: data.stats.outstanding,
                    }}
                    questions={{
                      segment: {
                        type: "choice",
                        instructions:
                          "Which business segment best describes this contact, given all fields?",
                        criteria: {
                          individual:
                            "A person buying occasionally for personal use",
                          retail:
                            "A shop or retailer buying small quantities regularly",
                          wholesale:
                            "A wholesaler or distributor buying in bulk",
                          corporate:
                            "A company with structured purchasing and invoices",
                          online:
                            "An online-only store or channel",
                          service:
                            "A service provider rather than a goods buyer",
                        },
                      },
                    }}
                    answerKey="segment"
                    labels={{
                      individual: t("Individual"),
                      retail: t("Retail"),
                      wholesale: t("Wholesale"),
                      corporate: t("Corporate"),
                      online: t("Online"),
                      service: t("Service"),
                    }}
                  />
                  <div className="form-actions">
                    <Button type="submit" disabled={busy || !dirty}>
                      {t(busy ? "Saving…" : "Save contact")}
                    </Button>
                  </div>
                </form>
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
                          recordKind(row.original) === "Order"
                            ? "Sales order"
                            : recordKind(row.original) === "Fulfill"
                              ? "Sales invoice"
                              : "Sales return",
                        ),
                    },
                    {
                      header: t("Reference"),
                      cell: ({ row }) => (
                        <Button
                          variant="link"
                          type="button"
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
                        date(recordCommand(row.original).context.date),
                    },
                    {
                      header: t("Total"),
                      cell: ({ row }) =>
                        money(recordData(row.original).total ?? "0"),
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
                <h3>{t("Outstanding invoices")}</h3>
                <DataGrid
                  data={data.receivables}
                  id={(r) => r.id}
                  empty={t("No outstanding invoices")}
                  columns={[
                    { header: t("Reference"), accessorKey: "reference" },
                    {
                      header: t("Due date"),
                      cell: ({ row }) => date(row.original.due_date),
                    },
                    {
                      header: t("Remaining"),
                      cell: ({ row }) => money(row.original.remaining),
                    },
                  ]}
                />
                {data.party.supplier && (
                  <>
                    <h3>{t("Supplier balances")}</h3>
                    <DataGrid
                      data={data.payables}
                      id={(r) => r.id}
                      empty={t("No supplier balances")}
                      columns={[
                        { header: t("Reference"), accessorKey: "reference" },
                        {
                          header: t("Remaining"),
                          cell: ({ row }) => money(row.original.remaining),
                        },
                      ]}
                    />
                  </>
                )}
              </TabsContent>
              <TabsContent value="performance">
                <Note>
                  {t(
                    "Performance includes posted invoices and returns through the selected date. Orders are counted separately.",
                  )}
                </Note>
                <Field label={t("As of")}>
                  <DatePicker
                    value={asof}
                    onChange={(v) => {
                      setAsof(v);
                      setOffset(0);
                    }}
                  />
                </Field>
                <div className="grouped-list">
                  {[
                    { label: "Orders", value: data.stats.orders },
                    { label: "Invoices", value: data.stats.invoices },
                    { label: "Returns", value: data.stats.returns },
                  ].map((m) => (
                    <div className="summary-line px-4 py-3" key={m.label}>
                      <span>{t(m.label)}</span>
                      <strong>
                        {new Intl.NumberFormat(b.numberLocale).format(
                          BigInt(m.value),
                        )}
                      </strong>
                    </div>
                  ))}
                  <div className="summary-line px-4 py-3">
                    <span>{t("Gross profit")}</span>
                    {money(data.stats.gross_profit)}
                  </div>
                  <div className="summary-line px-4 py-3">
                    <span>{t("Returns value")}</span>
                    {money(data.stats.returns_value)}
                  </div>
                  <div className="summary-line px-4 py-3">
                    <span>{t("Last activity")}</span>
                    <span>
                      {data.stats.last_activity
                        ? date(data.stats.last_activity)
                        : "—"}
                    </span>
                  </div>
                </div>
                <h3>{t("Monthly net sales")}</h3>
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
                      cell: ({ row }) => money(row.original.net),
                    },
                  ]}
                />
                <h3>{t("Invoiced products")}</h3>
                <DataGrid
                  data={data.products}
                  id={(r) => r.id}
                  empty={t("No posted sales")}
                  columns={[
                    { header: t("Product"), accessorKey: "name" },
                    {
                      header: t("Quantity"),
                      cell: ({ row }) =>
                        formatQuantity(row.original.quantity, b.numberLocale),
                    },
                    {
                      header: t("Invoiced amount"),
                      cell: ({ row }) => money(row.original.revenue),
                    },
                  ]}
                />
              </TabsContent>
            </Tabs>
          </>
        )}
      </Modal>
      <Modal
        open={discard}
        onOpenChange={setDiscard}
        title={t("Leave unsaved changes?")}
        description={t("Discard unsaved contact details?")}
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

import { useMemo } from "react";
import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from "../components/ui/command";
import { Modal } from "../components/layout/modal";
import { navigation } from "../app/navigation";
import { useTranslation } from "../lib/i18n";
import { useBooks } from "../app/books";
import {
  recordCommand,
  recordData,
  recordKind,
} from "./sales";
import type { Page, RecordEntry } from "../lib/types";
import {
  creationShortcuts,
  shortcutKeys,
  type QuickAction,
} from "../lib/shortcuts";
import { UsersIcon as Users } from "@phosphor-icons/react/dist/csr/Users";
import { PackageIcon as Package } from "@phosphor-icons/react/dist/csr/Package";
import { FileTextIcon as FileText } from "@phosphor-icons/react/dist/csr/FileText";
import { ShoppingBagIcon as ShoppingBag } from "@phosphor-icons/react/dist/csr/ShoppingBag";
import { ArrowsLeftRightIcon as ArrowsLeftRight } from "@phosphor-icons/react/dist/csr/ArrowsLeftRight";
import { BookOpenIcon as BookOpen } from "@phosphor-icons/react/dist/csr/BookOpen";

/**
 * Universal search (Ctrl/⌘+K): pages and actions plus everything already
 * loaded in the workspace — contacts with outstanding balances, products,
 * sales/purchases/payment documents and journals. Everything filters
 * client-side over in-memory data, so results are instant.
 */
export function CommandMenu({
  onClose,
  onNavigate,
  onSetup,
  onJournal,
  onToggleSidebar,
  canPost,
  onAction,
}: {
  onClose: () => void;
  onNavigate: (page: Page) => void;
  onSetup: () => void;
  onJournal: () => void;
  onToggleSidebar: () => void;
  canPost: boolean;
  onAction: (action: QuickAction) => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const parties = useMemo(() => {
    const balances = b.report?.party_balances ?? {};
    return (b.workspace.resources?.parties ?? []).map((p) => ({
      party: p,
      balance: balances[`${p.customer && !p.supplier ? "customer" : "vendor"}:${p.id}`]
        ?? balances[`customer:${p.id}`]
        ?? balances[`vendor:${p.id}`]
        ?? "0",
    }));
  }, [b.workspace.resources, b.report]);
  const items = b.workspace.resources?.items ?? [];
  const sales = b.workspace.sales;
  const purchases = b.workspace.purchases;
  const payments = b.workspace.payments ?? [];
  const journals = b.workspace.journals;
  const drafts = b.workspace.drafts;

  const partyLabel = (id: string) =>
    b.workspace.resources?.parties.find((p) => p.id === id)?.name ?? id;
  const docTotal = (r: RecordEntry) =>
    recordData(r).total ?? recordData(r).applied ?? "";
  const docLabel = (r: RecordEntry) => {
    const kind = recordKind(r);
    const c = recordCommand(r);
    const data = recordData(r);
    if (kind === "Order")
      return `${t("Order")} ${c.reference ?? ""} · ${data.customer?.name ?? partyLabel(data.customer_id ?? "")}`;
    if (kind === "Fulfill")
      return `${t("Invoice")} ${c.invoice_reference ?? ""} · ${data.customer?.name ?? partyLabel(data.customer_id ?? "")}`;
    if (kind === "Return")
      return `${t("Return")} ${c.return_reference ?? ""} · ${partyLabel(data.customer_id ?? "")}`;
    if (kind === "Receive")
      return `${t("Receipt")} ${c.reference ?? ""} · ${partyLabel(data.supplier_id ?? "")}`;
    if (kind === "Bill")
      return `${t("Bill")} ${c.reference ?? ""} · ${partyLabel(data.supplier_id ?? "")}`;
    if (kind === "Payment" || kind === "Reversal")
      return `${t(kind === "Payment" ? "Payment" : "Reversal")} ${data.reference ?? ""} · ${partyLabel(data.party?.id ?? c.party_id ?? "")}`;
    return c.reference ?? r.id;
  };
  const money = (v: string) =>
    v
      ? new Intl.NumberFormat(b.numberLocale === "fa" ? "fa-IR" : "en-US").format(
          BigInt(v),
        )
      : "";

  return (
    <Modal
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={t("Command menu")}
      description={t(
        "Search pages, actions, contacts, products and documents. Use arrow keys and Enter to select.",
      )}
    >
      <Command label={t("Search commands")}>
        <CommandInput
          data-autofocus="true"
          placeholder={t("Type a command…")}
          aria-label={t("Search commands")}
        />
        <CommandList aria-label={t("Search results")}>
          <CommandEmpty>{t("No matching commands.")}</CommandEmpty>
          <CommandGroup heading={t("Pages")}>
            {navigation.map((n) => (
              <CommandItem
                key={n.page}
                value={`${t(n.label)} ${n.label} page`}
                keywords={[n.label]}
                onSelect={() => onNavigate(n.page)}
              >
                <n.icon />
                {t(n.label)}
              </CommandItem>
            ))}
          </CommandGroup>
          <CommandGroup heading={t("Actions")}>
            {creationShortcuts
              .filter((s) => s.action !== "journal")
              .map((s) => (
                <CommandItem
                  key={s.action}
                  value={`${t(s.label)} ${s.label}`}
                  keywords={[s.label]}
                  onSelect={() => onAction(s.action)}
                >
                  {t(s.label)}
                  <kbd dir="ltr">{shortcutKeys(s.code)}</kbd>
                </CommandItem>
              ))}
            <CommandItem onSelect={onSetup}>{t("Set up company")}</CommandItem>
            <CommandItem disabled={!canPost} onSelect={onJournal}>
              {t("New journal")}
            </CommandItem>
            <CommandItem onSelect={onToggleSidebar}>
              {t("Toggle sidebar")}
              <kbd>F9</kbd>
            </CommandItem>
          </CommandGroup>
          {parties.length > 0 && (
            <CommandGroup heading={t("Contacts")}>
              {parties.slice(0, 40).map(({ party, balance }) => (
                <CommandItem
                  key={party.id}
                  value={`${party.name} ${t(party.customer && !party.supplier ? "Customer" : party.supplier && !party.customer ? "Vendor" : "Contact")} ${balance !== "0" ? money(balance) : ""}`}
                  keywords={[party.name]}
                  onSelect={() => {
                    b.openContact(party.id);
                    onClose();
                  }}
                >
                  <Users />
                  <span>{party.name}</span>
                  <span className="command-meta">
                    {party.customer && !party.supplier
                      ? t("Customer")
                      : party.supplier && !party.customer
                        ? t("Vendor")
                        : t("Contact")}
                    {balance !== "0" &&
                      ` · ${t("Outstanding")} ${money(balance)}`}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {items.length > 0 && (
            <CommandGroup heading={t("Products")}>
              {items.slice(0, 40).map((i) => (
                <CommandItem
                  key={i.id}
                  value={`${i.name} ${i.sku} ${t("Product")}`}
                  keywords={[i.name, i.sku]}
                  onSelect={() => {
                    b.openProduct(i.id);
                    onClose();
                  }}
                >
                  <Package />
                  <span>{i.name}</span>
                  <span className="command-meta">{i.sku}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {drafts.length > 0 && (
            <CommandGroup heading={t("Drafts")}>
              {drafts.slice(0, 20).map((d) => (
                <CommandItem
                  key={d.id}
                  value={`${t("Draft")} ${d.reference} ${partyLabel(d.customer_id)}`}
                  keywords={[d.reference]}
                  onSelect={() => {
                    onNavigate("sales");
                    onClose();
                  }}
                >
                  <FileText />
                  <span>
                    {t("Draft")} {d.reference || t("Untitled order")}
                  </span>
                  <span className="command-meta">
                    {partyLabel(d.customer_id)}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {sales.length > 0 && (
            <CommandGroup heading={t("Sales")}>
              {sales.slice(0, 30).map((r) => (
                <CommandItem
                  key={r.id}
                  value={`${docLabel(r)} sale sales ${money(docTotal(r))}`}
                  keywords={[recordCommand(r).reference ?? ""]}
                  onSelect={() => {
                    b.openRecord(r);
                    onClose();
                  }}
                >
                  <FileText />
                  <span>{docLabel(r)}</span>
                  <span className="command-meta">
                    {money(docTotal(r))} {b.unit === "Rial" ? "IRR" : t("toman")}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {purchases.length > 0 && (
            <CommandGroup heading={t("Purchases")}>
              {purchases.slice(0, 30).map((r) => (
                <CommandItem
                  key={r.id}
                  value={`${docLabel(r)} purchase buy ${money(docTotal(r))}`}
                  keywords={[recordCommand(r).reference ?? ""]}
                  onSelect={() => {
                    b.openRecord(r);
                    onClose();
                  }}
                >
                  <ShoppingBag />
                  <span>{docLabel(r)}</span>
                  <span className="command-meta">
                    {money(docTotal(r))} {b.unit === "Rial" ? "IRR" : t("toman")}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          {payments.length > 0 && (
            <CommandGroup heading={t("Payments")}>
              {payments.slice(0, 30).map((p) => {
                const data = Object.values(p.data)[0];
                const ctx = Object.values(p.command)[0]?.context;
                return (
                  <CommandItem
                    key={p.id}
                    value={`payment ${data?.reference ?? ""} ${partyLabel(data?.party?.id ?? ctx?.party_id ?? "")} ${money(data?.total ?? "")}`}
                    keywords={[data?.reference ?? ""]}
                    onSelect={() => {
                      onNavigate("payments");
                      onClose();
                    }}
                  >
                    <ArrowsLeftRight />
                    <span>
                      {t(data?.payment_id ? "Reversal" : "Payment")}{" "}
                      {data?.reference}
                    </span>
                    <span className="command-meta">
                      {partyLabel(data?.party?.id ?? ctx?.party_id ?? "")} ·{" "}
                      {money(data?.total ?? "")}
                    </span>
                  </CommandItem>
                );
              })}
            </CommandGroup>
          )}
          {journals.length > 0 && (
            <CommandGroup heading={t("Journals")}>
              {journals.slice(0, 20).map((j) => (
                <CommandItem
                  key={j.id}
                  value={`journal ${j.reason} ${j.id.slice(0, 8)}`}
                  keywords={[j.reason]}
                  onSelect={() => {
                    onNavigate("accounting");
                    onClose();
                  }}
                >
                  <BookOpen />
                  <span>{j.reason || `#${j.id.slice(0, 8)}`}</span>
                  <span className="command-meta">
                    {money(j.total_debit)} {b.unit === "Rial" ? "IRR" : t("toman")}
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
        </CommandList>
      </Command>
    </Modal>
  );
}

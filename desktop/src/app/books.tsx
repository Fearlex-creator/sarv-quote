import { createContext, useContext } from "react";
import type { MessageValues } from "../lib/i18n";
import type { Page, Report, Workspace, RecordEntry } from "../lib/types";
import type { ResourceKind, QuickAction } from "../lib/shortcuts";
export interface Books {
  workspace: Workspace;
  companyId: string;
  periodId: string;
  report: Report | null;
  reportError: string;
  headerTarget: HTMLDivElement | null;
  lastSyncedAt: Date | null;
  from: string;
  to: string;
  setRange: (from: string, to: string) => void;
  refresh: () => Promise<void>;
  navigate: (page: Page) => void;
  unit: "Rial" | "Toman";
  locale: "en" | "fa";
  numberLocale: "en" | "fa";
  dateLocale: string;
  t: (s: string, values?: MessageValues) => string;
  openSetup: () => void;
  openCompanySetup: () => void;
  openJournal: () => void;
  openResource: (kind: ResourceKind, onCreated?: (id: string) => void) => void;
  runAction: (action: QuickAction) => void;
  openPurchase: (seed: { item_id: string; quantity: string }[]) => void;
  openContact: (id: string) => void;
  openProduct: (id: string) => void;
  openRecord: (record: RecordEntry) => void;
}
export const BooksContext = createContext<Books | null>(null);
export function useBooks() {
  const c = useContext(BooksContext);
  if (!c) throw Error("Books context missing");
  return c;
}

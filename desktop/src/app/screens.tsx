import { lazy, memo } from "react";
import type { Page } from "../lib/types";

// Load feature code only when needed; the module promises are shared by hover,
// keyboard focus, navigation, and related editors.
const overview = () => import("../features/overview");
const sales = () => import("../features/sales");
const purchases = () => import("../features/purchases");
const resources = () => import("../features/resources");
const accounting = () => import("../features/accounting");
const reports = () => import("../features/reports");
const settings = () => import("../features/settings");
export const Overview = lazy(() =>
  overview().then((m) => ({ default: memo(m.Overview) })),
);
export const Sales = lazy(() =>
  sales().then((m) => ({ default: memo(m.Sales) })),
);
export const OrderEditor = lazy(() =>
  sales().then((m) => ({ default: m.OrderEditor })),
);
export const QuickSale = lazy(() =>
  import("../features/quick-sale").then((m) => ({ default: m.QuickSale })),
);
export const Purchases = lazy(() =>
  purchases().then((m) => ({ default: memo(m.Purchases) })),
);
export const PurchaseEditor = lazy(() =>
  purchases().then((m) => ({ default: m.PurchaseEditor })),
);
export const ResourceDialog = lazy(() =>
  resources().then((m) => ({ default: m.ResourceDialog })),
);
export const TransferDialog = lazy(() =>
  import("../features/transfer").then((m) => ({ default: m.TransferDialog })),
);
export const Customers = lazy(() =>
  resources().then((m) => ({ default: memo(m.Contacts) })),
);
export const ContactProfile = lazy(() =>
  import("../features/contact-profile").then((m) => ({
    default: m.ContactProfile,
  })),
);
export const Banking = lazy(() =>
  import("../features/banking").then((m) => ({ default: m.Banking })),
);
export const Inventory = lazy(() =>
  resources().then((m) => ({ default: memo(m.Inventory) })),
);
export const Accounting = lazy(() =>
  accounting().then((m) => ({ default: memo(m.Accounting) })),
);
export const JournalDetail = lazy(() =>
  accounting().then((m) => ({ default: m.JournalDetail })),
);
export const JournalEditor = lazy(() =>
  accounting().then((m) => ({ default: m.JournalEditor })),
);
export const Reports = lazy(() =>
  reports().then((m) => ({ default: memo(m.Reports) })),
);
export const Settings = lazy(() =>
  settings().then((m) => ({ default: memo(m.Settings) })),
);
export const SetupDialog = lazy(() =>
  import("../features/setup").then((m) => ({ default: m.SetupDialog })),
);
export const Payments = lazy(() =>
  import("../features/payments").then((m) => ({ default: memo(m.Payments) })),
);
export const Pos = lazy(() =>
  import("../features/pos").then((m) => ({ default: memo(m.Pos) })),
);
export const ExcelIO = lazy(() =>
  import("../features/excel").then((m) => ({ default: memo(m.ExcelIO) })),
);
export const RecordDetail = lazy(() =>
  import("../features/records").then((m) => ({ default: m.RecordDetail })),
);
export const Onboarding = lazy(() =>
  import("../features/onboarding").then((m) => ({ default: m.Onboarding })),
);
const loaders: Partial<Record<Page, () => Promise<unknown>>> = {
  overview,
  sales,
  purchases,
  payments: () => import("../features/payments"),
  pos: () => import("../features/pos"),
  excel: () => import("../features/excel"),
  inventory: resources,
  customers: resources,
  accounting,
  reports,
  settings,
  banking: () => import("../features/banking"),
};
export function preloadPage(page: Page) {
  // Failed preloads must not replace the current screen or interrupt input.
  void loaders[page]?.().catch(() => undefined);
}

export function preloadAll() {
  (Object.keys(loaders) as Page[]).forEach(preloadPage);
  for (const load of [
    () => import("../features/records"),
    () => import("../features/quick-sale"),
    () => import("../features/setup"),
    () => import("../features/command-menu"),
    () => import("../features/product-profile"),
    () => import("../features/contact-profile"),
  ])
    void load().catch(() => undefined);
}

export const CommandMenu = lazy(() =>
  import("../features/command-menu").then((m) => ({ default: m.CommandMenu })),
);

export const ProductProfile = lazy(() =>
  import("../features/product-profile").then((m) => ({
    default: m.ProductProfile,
  })),
);

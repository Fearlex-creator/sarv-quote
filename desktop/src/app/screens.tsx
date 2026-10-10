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
export const Purchases = lazy(() =>
  purchases().then((m) => ({ default: memo(m.Purchases) })),
);
export const Customers = lazy(() =>
  resources().then((m) => ({ default: memo(m.Customers) })),
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
  inventory: resources,
  customers: resources,
  accounting,
  reports,
  settings,
};
export function preloadPage(page: Page) {
  // Failed preloads must not replace the current screen or interrupt input.
  void loaders[page]?.().catch(() => undefined);
}

export const CommandMenu = lazy(() =>
  import("../features/command-menu").then((m) => ({ default: m.CommandMenu })),
);

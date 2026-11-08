import { Avatar, AvatarFallback } from "../components/ui/avatar";
import { Separator } from "../components/ui/separator";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import {
  Suspense,
  startTransition,
  useMemo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { toast } from "sonner";
import {
  creationShortcuts,
  isTyping,
  type ResourceKind,
  type QuickAction,
} from "../lib/shortcuts";
import { ShortcutList } from "../components/finance/shortcut-list";
import { Toaster } from "../components/ui/sonner";
import { SidebarSimpleIcon as PanelLeftClose } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { SidebarSimpleIcon as PanelLeftOpen } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { MagnifyingGlassIcon as Search } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { SunIcon as Sun } from "@phosphor-icons/react/dist/csr/Sun";
import { MoonIcon as Moon } from "@phosphor-icons/react/dist/csr/Moon";
import { QuestionIcon as CircleHelp } from "@phosphor-icons/react/dist/csr/Question";
import { PlantIcon as Sprout } from "@phosphor-icons/react/dist/csr/Plant";
import { ArrowsClockwiseIcon as RefreshCw } from "@phosphor-icons/react/dist/csr/ArrowsClockwise";
import { command, today } from "../lib/client";
import { readSession, writeSession } from "../lib/session";
import { LockScreen } from "../features/lock";
import { AccessPanel } from "../features/access";
import { installEnterAdvance } from "../lib/enter-advance";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Draft,
  Journal,
  Page,
  RecordEntry,
  Report,
  Workspace,
} from "../lib/types";
import { DirectionProvider } from "../components/ui/direction";
import { BooksContext, type Books } from "./books";
import { navigation, navigationGroups } from "./navigation";
import {
  I18nContext,
  initialLocale,
  translate,
  type MessageValues,
} from "../lib/i18n";
import {
  Sidebar,
  SidebarHeader,
  SidebarContent,
  SidebarFooter,
  SidebarProvider,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from "../components/ui/sidebar";
import { Button } from "../components/ui/button";
import { Skeleton } from "../components/ui/skeleton";
import { Modal } from "../components/layout/modal";
import { ErrorState, Loading } from "../components/ui/shared";
import {
  Overview,
  SetupDialog,
  Sales,
  OrderEditor,
  QuickSale,
  RecordDetail,
  Purchases,
  PurchaseEditor,
  Payments,
  ResourceDialog,
  TransferDialog,
  Customers,
  ContactProfile,
  ProductProfile,
  Banking,
  Inventory,
  Accounting,
  JournalDetail,
  JournalEditor,
  Reports,
  Settings,
  Pos,
  ExcelIO,
  Onboarding,
  preloadPage,
  preloadAll,
  CommandMenu,
} from "./screens";
export default function App() {
  const [page, setPage] = useState<Page>("overview");
  const [locked, setLocked] = useState(false);
  const [setupRequired, setSetupRequired] = useState(false);
  const [authChecked, setAuthChecked] = useState(false);
  const qaPreview = Boolean(import.meta.env.VITE_QA_URL);
  const [workspace, setWorkspace] = useState<Workspace | null>(null);
  const [companyId, setCompanyId] = useState(
    () => localStorage.getItem("sarv.company") ?? "",
  );
  const [periodId, setPeriodId] = useState("");
  const [error, setError] = useState("");
  const [refreshing, setRefreshing] = useState(false);
  const [workspaceSyncedAt, setWorkspaceSyncedAt] = useState<Date | null>(null);
  const [reportSyncedAt, setReportSyncedAt] = useState<Date | null>(null);
  const [headerTarget, setHeaderTarget] = useState<HTMLDivElement | null>(null);
  const [mobileNavigation, setMobileNavigation] = useState(false);
  const [report, setReport] = useState<Report | null>(null);
  const [reportError, setReportError] = useState("");
  const [range, setRange] = useState({
    from: today().slice(0, 4) + "-01-01",
    to: today(),
  });
  const [locale, setLocale] = useState<"en" | "fa">(initialLocale);
  const [collapsed, setCollapsed] = useState(
    () => localStorage.getItem("sarv.sidebar") !== "expanded",
  );
  const [palette, setPalette] = useState(false);
  const [help, setHelp] = useState(false);
  const [setup, setSetup] = useState(false);
  const [onboarding, setOnboarding] = useState(false);
  const [newCompany, setNewCompany] = useState(false);
  const [journalEditor, setJournalEditor] = useState(false);
  const [editor, setEditor] = useState<{
    draft?: Draft;
    invoice?: boolean;
  } | null>(null);
  const [resource, setResource] = useState<{
    kind: ResourceKind;
    onCreated?: (id: string) => void;
  } | null>(null);
  const [purchaseEditor, setPurchaseEditor] = useState(false);
  const [purchaseSeed, setPurchaseSeed] = useState<
    { item_id: string; quantity: string }[]
  >([]);
  const [transferEditor, setTransferEditor] = useState(false);
  const [pendingAction, setPendingAction] = useState<QuickAction | null>(null);
  const [journal, setJournal] = useState<Journal | null>(null);
  const [record, setRecord] = useState<RecordEntry | null>(null);
  const [productId, setProductId] = useState<string | null>(null);
  const [contactId, setContactId] = useState<string | null>(null);
  const scroll = useRef<HTMLElement>(null);
  useLayoutEffect(() => {
    scroll.current?.scrollTo({ top: 0 });
  }, [page, editor]);
  const request = useRef(0);
  const reportRequest = useRef(0);
  const t = useCallback(
    (s: string, values?: MessageValues) => translate(s, locale, values),
    [locale],
  );
  const refresh = useCallback(async () => {
    const version = ++request.current;
    setRefreshing(true);
    try {
      const w = await command<Workspace>("workspace", {
        company_id: companyId,
      });
      if (version !== request.current) return;
      setWorkspace(w);
      setWorkspaceSyncedAt(new Date());
      setError("");
      if (!companyId && w.companies.length) {
        setCompanyId(w.companies[0].id);
        return;
      }
      setPeriodId((previous) =>
        w.periods.some((p) => p.id === previous)
          ? previous
          : (w.periods.find(
              (p) => p.start_date <= today() && p.end_date >= today(),
            )?.id ??
            w.periods[0]?.id ??
            ""),
      );
    } catch (e) {
      if (version !== request.current) return;
      const message = (e as Error).message;
      // A restore replaces company ids. Drop the stale selection and load again.
      if (companyId && message.includes("Company not found")) {
        setCompanyId("");
        return;
      }
      setError(message);
    } finally {
      if (version === request.current) setRefreshing(false);
    }
  }, [companyId]);
  useEffect(() => {
    let live = true;
    void command<{ configured: boolean }>("auth_status")
      .then((status) => {
        if (!live) return;
        setSetupRequired(!qaPreview && !status.configured);
        setLocked(status.configured && !readSession());
        setAuthChecked(true);
      })
      .catch(() => {
        if (live) setAuthChecked(true);
      });
    const onLock = () => setLocked(true);
    window.addEventListener("sarv-locked", onLock);
    return () => {
      live = false;
      window.removeEventListener("sarv-locked", onLock);
    };
  }, [qaPreview]);
  useEffect(() => {
    if (!authChecked || locked || setupRequired) return;
    void refresh();
  }, [refresh, authChecked, locked, setupRequired]);
  useEffect(() => installEnterAdvance(), []);
  useEffect(() => {
    // Warm every screen once the shell is idle so navigation never waits on a
    // chunk download or parse.
    const warm = () => preloadAll();
    const id = window.setTimeout(warm, 300);
    return () => window.clearTimeout(id);
  }, []);
  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    let unlisten: (() => void) | undefined;
    void listen("sarv-books-changed", () => {
      if (active) void refresh();
    }).then(
      (stop) => {
        if (active) unlisten = stop;
        else stop();
      },
      (e: Error) => {
        if (active) setError(e.message);
      },
    );
    return () => {
      active = false;
      unlisten?.();
    };
  }, [refresh]);
  useEffect(() => {
    localStorage.setItem("sarv.company", companyId);
    setWorkspace(null);
    setReport(null);
    setReportError("");
    setRecord(null);
    setJournal(null);
  }, [companyId]);
  useLayoutEffect(() => {
    localStorage.setItem("sarv.language", locale);
    document.documentElement.lang = locale;
    document.documentElement.dir = locale === "fa" ? "rtl" : "ltr";
  }, [locale]);
  useEffect(() => {
    localStorage.setItem("sarv.sidebar", collapsed ? "collapsed" : "expanded");
  }, [collapsed]);
  useEffect(() => {
    const appearance = workspace?.settings.appearance ?? "Light";
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () =>
      document.documentElement.classList.toggle(
        "dark",
        appearance === "Dark" || (appearance === "System" && media.matches),
      );
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [workspace?.settings.appearance]);
  useEffect(() => {
    // Keep the previous report on screen while the next one loads; the company
    // effect clears it when the books themselves change.
    const version = ++reportRequest.current;
    if (!workspace || !companyId) return;
    command<Report>("report", {
      company_id: companyId,
      from: range.from,
      to: range.to,
    })
      .then((r) => {
        if (version === reportRequest.current) {
          setReport(r);
          setReportError("");
          setReportSyncedAt(new Date());
        }
      })
      .catch((e) => {
        if (version === reportRequest.current)
          setReportError((e as Error).message);
      });
  }, [workspace, companyId, range]);
  const navigate = useCallback(
    (next: Page) => {
      if (editor) {
        toast.info(
          t("Use Back to sales to save or discard your current draft."),
        );
        return;
      }
      preloadPage(next);
      startTransition(() => setPage(next));
      setPalette(false);
    },
    [editor, t],
  );
  const openResource = useCallback(
    (kind: ResourceKind, onCreated?: (id: string) => void) => {
      if (!workspace || (!companyId && workspace.companies.length)) return;
      setResource({ kind, onCreated });
      if (!workspace?.initialized) {
        toast.info(
          t(
            "Confirm your starting balances first. New shops can start with zero balances.",
          ),
        );
        setNewCompany(!companyId);
        setSetup(true);
      }
    },
    [workspace, companyId, t],
  );
  const runAction = useCallback(
    (action: QuickAction) => {
      if (!workspace || (!companyId && workspace.companies.length)) return;
      setPalette(false);
      if (editor) {
        toast.info(
          t("Use Back to sales to save or discard your current draft."),
        );
        return;
      }
      if (
        ["customer", "vendor", "item", "warehouse", "unit"].includes(action)
      ) {
        openResource(action as ResourceKind);
        return;
      }
      if (!workspace?.initialized) {
        setPendingAction(action);
        setNewCompany(!companyId);
        setSetup(true);
        toast.info(
          t(
            "Confirm your starting balances first. New shops can start with zero balances.",
          ),
        );
        return;
      }
      if (action === "invoice" || action === "order") {
        navigate("sales");
        setEditor({ invoice: action === "invoice" });
      } else if (action === "purchase") {
        setPurchaseSeed([]);
        setPurchaseEditor(true);
      }
      else if (action === "journal") setJournalEditor(true);
      else if (action === "transfer") setTransferEditor(true);
    },
    [editor, openResource, workspace, companyId, navigate, t],
  );
  useEffect(() => {
    if (pendingAction && workspace?.initialized && !setup) {
      setPendingAction(null);
      runAction(pendingAction);
    }
  }, [pendingAction, workspace?.initialized, setup, runAction]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.repeat || e.isComposing || e.getModifierState("AltGraph")) return;
      const modalOpen = !!document.querySelector(
        '[role="dialog"][data-state="open"]',
      );
      if (
        (e.ctrlKey || e.metaKey) &&
        !e.altKey &&
        !e.shiftKey &&
        e.code === "KeyK"
      ) {
        if (modalOpen && !palette) return;
        e.preventDefault();
        setPalette((v) => !v);
        return;
      }
      if (modalOpen || isTyping(e.target)) return;
      if ((e.ctrlKey || e.metaKey) && e.altKey && !e.shiftKey) {
        const shortcut = creationShortcuts.find((s) => s.code === e.code);
        if (shortcut) {
          e.preventDefault();
          runAction(shortcut.action);
        } else if (e.code === "KeyS") {
          e.preventDefault();
          navigate("settings");
        }
      } else if (!e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
        if (e.key === "F9") {
          e.preventDefault();
          setCollapsed((v) => !v);
        }
        if (e.key === "F1") {
          e.preventDefault();
          setHelp(true);
        }
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [runAction, navigate, palette]);
  const openJournal = useCallback(
    async (id: string) => {
      try {
        const j = await command<Journal>("detail", {
          company_id: companyId,
          id,
        });
        setRecord(null);
        setJournal(j);
      } catch (e) {
        toast.error((e as Error).message);
      }
    },
    [companyId],
  );
  async function theme() {
    if (!workspace) return;
    const next = document.documentElement.classList.contains("dark")
      ? "Light"
      : "Dark";
    // Switch instantly; the saved setting catches up in the background.
    document.documentElement.classList.toggle("dark", next === "Dark");
    setWorkspace({
      ...workspace,
      settings: { ...workspace.settings, appearance: next },
    });
    try {
      await command("settings", {
        ...workspace.settings,
        appearance: next,
      });
    } catch (e) {
      toast.error((e as Error).message);
    }
  }
  const navItem = (n: (typeof navigation)[number]) => {
    const active = page === n.page;
    return (
      <SidebarMenuItem key={n.page}>
        <SidebarMenuButton
          isActive={active}
          aria-current={active ? "page" : undefined}
          tooltip={t(n.label)}
          onPointerEnter={() => preloadPage(n.page)}
          onFocus={() => preloadPage(n.page)}
          onClick={() => navigate(n.page)}
        >
          <n.icon
            data-nav-icon={n.page}
            weight="duotone"
            className="sidebar-nav-icon"
          />
          <span>{t(n.label)}</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  };
  const companyName =
    workspace?.companies.find((x) => x.id === companyId)?.name ??
    t("Your business");
  const editDraft = useCallback((draft?: Draft) => setEditor({ draft }), []);
  const viewJournal = useCallback(
    (id: string) => {
      void openJournal(id);
    },
    [openJournal],
  );
  const books = useMemo<Books | null>(
    () =>
      workspace
        ? {
            workspace,
            companyId,
            periodId,
            report,
            reportError,
            headerTarget,
            lastSyncedAt:
              page === "settings" || editor
                ? null
                : ["overview", "reports", "inventory", "customers"].includes(
                      page,
                    )
                  ? report
                    ? reportSyncedAt
                    : null
                  : workspaceSyncedAt,
            from: range.from,
            to: range.to,
            setRange: (from, to) => setRange({ from, to }),
            refresh,
            navigate,
            unit: workspace.settings.default_currency,
            locale,
            numberLocale:
              workspace.settings.display_digits === "Persian" ? "fa" : "en",
            dateLocale: `${locale === "fa" ? "fa-IR" : workspace.settings.date_display === "DayMonthYear" ? "en-GB" : "en-US"}-u-ca-${workspace.settings.date_display === "Jalali" ? "persian" : "gregory"}-nu-${workspace.settings.display_digits === "Persian" ? "arabext" : "latn"}`,
            t,
            openCompanySetup: () => {
              setNewCompany(true);
              setSetup(true);
            },
            openSetup: () => {
              setNewCompany(!companyId);
              setSetup(true);
            },
            openJournal: () => setJournalEditor(true),
            openResource,
            runAction,
            openPurchase: (seed) => {
              setPurchaseSeed(seed);
              setPurchaseEditor(true);
            },
            openContact: setContactId,
            openProduct: setProductId,
            openRecord: setRecord,
          }
        : null,
    [
      workspace,
      companyId,
      periodId,
      report,
      reportError,
      headerTarget,
      page,
      editor,
      reportSyncedAt,
      workspaceSyncedAt,
      range,
      refresh,
      navigate,
      locale,
      t,
      openResource,
      runAction,
    ],
  );
  const content =
    workspace && books ? (
      <BooksContext.Provider value={books}>
        <div className="page-content" key={page}>
          <Suspense
            fallback={
              <Loading
                shape={page === "settings" || editor ? "form" : "table"}
              />
            }
          >
            {editor?.invoice ? (
              <QuickSale onClose={() => setEditor(null)} />
            ) : editor ? (
              <OrderEditor
                initial={editor.draft}
                invoice={false}
                onClose={() => setEditor(null)}
              />
            ) : page === "overview" ? (
              !workspace.companies.length ? (
                <Onboarding
                  onLocale={setLocale}
                  onStart={() => {
                    setOnboarding(true);
                    setNewCompany(true);
                    setSetup(true);
                  }}
                />
              ) : (
                <Overview onDetail={setJournal} />
              )
            ) : page === "sales" ? (
              <Sales onEdit={editDraft} onDetail={setRecord} />
            ) : page === "pos" ? (
              <Pos />
            ) : page === "excel" ? (
              <ExcelIO />
            ) : page === "purchases" ? (
              <Purchases onDetail={setRecord} />
            ) : page === "payments" ? (
              <Payments onJournal={viewJournal} />
            ) : page === "accounting" ? (
              <Accounting onDetail={setJournal} />
            ) : page === "inventory" ? (
              <Inventory />
            ) : page === "reports" ? (
              <Reports onJournal={viewJournal} />
            ) : page === "customers" ? (
              <Customers />
            ) : page === "settings" ? (
              <Settings onLocale={setLocale} />
            ) : (
              <Banking onJournal={viewJournal} />
            )}
          </Suspense>
        </div>
        <Suspense fallback={null}>
          {productId && (
            <ProductProfile
              key={productId}
              id={productId}
              onClose={() => setProductId(null)}
            />
          )}
          {contactId && (
            <ContactProfile
              key={contactId}
              id={contactId}
              onClose={() => setContactId(null)}
            />
          )}
          {setup && (
            <SetupDialog
              firstRun={onboarding}
              onInitialized={() => {
                setSetup(false);
                setOnboarding(false);
              }}
              newCompany={newCompany}
              open={setup}
              onClose={() => {
                setSetup(false);
                setOnboarding(false);
                if (!workspace.initialized) {
                  setResource(null);
                  setPendingAction(null);
                }
              }}
              onCreated={(id) => {
                setCompanyId(id);
                setNewCompany(false);
                setSetup(true);
              }}
            />
          )}
          {journalEditor && (
            <JournalEditor onClose={() => setJournalEditor(false)} />
          )}{" "}
          {resource && workspace.initialized && !setup && (
            <ResourceDialog
              kind={resource.kind}
              onCreated={resource.onCreated}
              onClose={() => setResource(null)}
            />
          )}
          {purchaseEditor && (
            <PurchaseEditor
              seed={purchaseSeed}
              onClose={() => setPurchaseEditor(false)}
            />
          )}
          {transferEditor && (
            <TransferDialog onClose={() => setTransferEditor(false)} />
          )}
          {journal && (
            <JournalDetail journal={journal} onClose={() => setJournal(null)} />
          )}{" "}
          {record && (
            <RecordDetail
              record={record}
              onClose={() => setRecord(null)}
              onJournal={viewJournal}
            />
          )}{" "}
          {palette && (
            <Suspense fallback={null}>
              <CommandMenu
                onClose={() => setPalette(false)}
                onNavigate={navigate}
                canPost={!!workspace?.initialized}
                onAction={runAction}
                onSetup={() => {
                  setPalette(false);
                  setSetup(true);
                }}
                onJournal={() => {
                  setPalette(false);
                  setJournalEditor(true);
                }}
                onToggleSidebar={() => {
                  setCollapsed((v) => !v);
                  setPalette(false);
                }}
              />
            </Suspense>
          )}
        </Suspense>
      </BooksContext.Provider>
    ) : null;
  return (
    <I18nContext.Provider value={locale}>
      <DirectionProvider dir={locale === "fa" ? "rtl" : "ltr"}>
        {setupRequired ? (
          <div className="flex min-h-svh items-center justify-center p-6">
            <AccessPanel
              onReady={() => {
                setSetupRequired(false);
                void refresh();
              }}
            />
          </div>
        ) : locked ? (
          <LockScreen
            onUnlock={() => {
              setLocked(false);
              void refresh();
            }}
          />
        ) : (
          <>
        <SidebarProvider
          open={!collapsed}
          onOpenChange={(open) => setCollapsed(!open)}
        >
          <div
            className={`app-shell ${workspace?.settings.density === "Compact" ? "compact" : ""}`}
            data-first-run={
              workspace?.companies.length === 0 && page === "overview"
            }
          >
            <Sidebar
              side={locale === "fa" ? "right" : "left"}
              dir={locale === "fa" ? "rtl" : "ltr"}
              collapsible="icon"
              className={collapsed ? "sidebar sidebar-collapsed" : "sidebar"}
            >
              <SidebarHeader>
                <div className="brand-row">
                  <div className="brand">
                    <span className="brand-mark">
                      <Sprout />
                    </span>
                    <span className="brand-text">SARV</span>
                  </div>
                  <Button
                    className="collapse-button"
                    variant="ghost"
                    size="icon"
                    aria-label={t("Collapse sidebar")}
                    onClick={() => setCollapsed(true)}
                  >
                    <PanelLeftClose className="directional-icon" />
                  </Button>
                </div>
                <NativeSelect
                  className="workspace-select w-full"
                  aria-label={t("Organization")}
                  value={companyId}
                  disabled={!!editor}
                  onChange={(e) => {
                    setCompanyId(e.target.value);
                    setPage("overview");
                  }}
                >
                  <NativeSelectOption value="">
                    {t("Your business")}
                  </NativeSelectOption>
                  {workspace?.companies.map((c) => (
                    <NativeSelectOption key={c.id} value={c.id}>
                      {c.name}
                    </NativeSelectOption>
                  ))}
                </NativeSelect>
              </SidebarHeader>
              <SidebarContent>
                <Button
                  className="sidebar-primary-action"
                  aria-label={t("New sales invoice")}
                  title={t("New sales invoice")}
                  onClick={() => runAction("invoice")}
                  disabled={!workspace?.companies.length}
                >
                  <Plus />
                  <span>{t("New sales invoice")}</span>
                </Button>
                {navigationGroups
                  .filter((g) => g.id !== "workspace")
                  .map((g) => (
                    <SidebarGroup key={g.id}>
                      <SidebarGroupLabel>{t(g.label)}</SidebarGroupLabel>
                      <SidebarMenu aria-label={t(g.label)}>
                        {navigation
                          .filter((n) => n.group === g.id)
                          .map(navItem)}
                      </SidebarMenu>
                    </SidebarGroup>
                  ))}
              </SidebarContent>
              <SidebarFooter>
                <SidebarGroup>
                  <SidebarMenu>
                    {navigation.filter((n) => n.group === "workspace").map(navItem)}
                    <SidebarMenuItem>
                      <SidebarMenuButton
                        tooltip={t("Help & shortcuts")}
                        onClick={() => setHelp(true)}
                      >
                        <CircleHelp
                          data-nav-icon="help"
                          weight="duotone"
                          className="sidebar-nav-icon"
                        />
                        <span>{t("Help & shortcuts")}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  </SidebarMenu>
                </SidebarGroup>
                <div className="profile-row">
                  <Avatar>
                    <AvatarFallback>
                      {companyName.slice(0, 1).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <span className="truncate">{companyName}</span>
                  {readSession() && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => {
                        void command("auth_logout").finally(() => {
                          writeSession(null);
                          setLocked(true);
                        });
                      }}
                    >
                      {t("Sign out")}
                    </Button>
                  )}
                </div>
              </SidebarFooter>
            </Sidebar>
            <div className="main-shell">
              <header className="topbar">
                <div className="topbar-title">
                  {collapsed && (
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={t("Expand sidebar")}
                      onClick={() => setCollapsed(false)}
                    >
                      <PanelLeftOpen className="directional-icon" />
                    </Button>
                  )}
                  <div className="topbar-page" ref={setHeaderTarget}>
                    {!workspace && (
                      <h1>
                        {t(
                          navigation.find((n) => n.page === page)?.label ??
                            "Overview",
                        )}
                      </h1>
                    )}
                  </div>
                </div>
                <div className="topbar-actions">
                  {refreshing && workspace && (
                    <span className="refreshing">
                      <Skeleton aria-label={t("Refreshing")} />
                    </span>
                  )}
                  <Button
                    variant="ghost"
                    size="icon"
                    className="command-trigger"
                    aria-label={t("Search anything…")}
                    title={t("Search · Ctrl/⌘ K")}
                    onClick={() => setPalette(true)}
                  >
                    <Search />
                  </Button>
                  <Separator orientation="vertical" className="h-6" />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="theme-toggle"
                    aria-label={t("Toggle theme")}
                    onClick={() => void theme()}
                  >
                    {workspace?.settings.appearance === "Dark" ? (
                      <Sun />
                    ) : (
                      <Moon />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t("Refresh books")}
                    onClick={() => void refresh()}
                  >
                    <RefreshCw />
                  </Button>
                </div>
              </header>
              <main className="page-scroll" ref={scroll}>
                <div className="page" data-page={page}>
                  {error && (
                    <ErrorState error={error} retry={() => void refresh()} />
                  )}{" "}
                  {!workspace && !error && (
                    <Loading shape={page === "settings" ? "form" : "table"} />
                  )}
                  {content}
                </div>
              </main>
            </div>
            <nav
              className="mobile-navigation"
              aria-label={t("Mobile navigation")}
            >
              {navigation
                .filter((n) =>
                  ["overview", "sales", "reports"].includes(n.page),
                )
                .map((n) => (
                  <Button
                    variant="ghost"
                    key={n.page}
                    className="nav-item"
                    aria-current={page === n.page ? "page" : undefined}
                    onPointerEnter={() => preloadPage(n.page)}
                    onFocus={() => preloadPage(n.page)}
                    onClick={() => navigate(n.page)}
                  >
                    <n.icon />
                    <span>{t(n.label)}</span>
                  </Button>
                ))}
              <Button
                variant="ghost"
                className="nav-item"
                aria-label={t("More pages")}
                onClick={() => setMobileNavigation(true)}
              >
                <PanelLeftOpen className="directional-icon" />
                <span>{t("More")}</span>
              </Button>
            </nav>
          </div>
        </SidebarProvider>
        <Modal
          open={mobileNavigation}
          onOpenChange={setMobileNavigation}
          title={t("Navigate")}
        >
          <NativeSelect
            className="workspace-select w-full"
            aria-label={t("Mobile organization")}
            value={companyId}
            disabled={!!editor}
            onChange={(e) => {
              setCompanyId(e.target.value);
              setPage("overview");
              setMobileNavigation(false);
            }}
          >
            <NativeSelectOption value="">
              {t("Your business")}
            </NativeSelectOption>
            {workspace?.companies.map((c) => (
              <NativeSelectOption key={c.id} value={c.id}>
                {c.name}
              </NativeSelectOption>
            ))}
          </NativeSelect>
          <nav className="mobile-pages" aria-label={t("All pages")}>
            {navigation.map((n) => (
              <Button
                variant="ghost"
                key={n.page}
                className="nav-item"
                aria-current={page === n.page ? "page" : undefined}
                onClick={() => {
                  navigate(n.page);
                  setMobileNavigation(false);
                }}
              >
                <n.icon />
                <span>{t(n.label)}</span>
              </Button>
            ))}
          </nav>
        </Modal>
        <Modal
          open={help}
          onOpenChange={setHelp}
          title={t("Help & shortcuts")}
          description={t(
            "Local, offline accounting with exact amounts and traceable sources.",
          )}
        >
          <ShortcutList />
        </Modal>
        <Toaster
          theme={
            workspace?.settings.appearance === "Dark"
              ? "dark"
              : workspace?.settings.appearance === "Light"
                ? "light"
                : "system"
          }
          dir={locale === "fa" ? "rtl" : "ltr"}
          position={locale === "fa" ? "bottom-left" : "bottom-right"}
          toastOptions={{ closeButtonAriaLabel: t("Close notification") }}
          containerAriaLabel={t("Notifications")}
          closeButton
        />
          </>
        )}
      </DirectionProvider>
    </I18nContext.Provider>
  );
}

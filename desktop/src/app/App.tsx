import {
  Empty,
  EmptyHeader,
  EmptyTitle,
  EmptyContent,
} from "../components/ui/empty";
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
import { Toaster, toast } from "sonner";
import { SidebarSimpleIcon as PanelLeftClose } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { SidebarSimpleIcon as PanelLeftOpen } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { MagnifyingGlassIcon as Search } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { SunIcon as Sun } from "@phosphor-icons/react/dist/csr/Sun";
import { MoonIcon as Moon } from "@phosphor-icons/react/dist/csr/Moon";
import { QuestionIcon as CircleHelp } from "@phosphor-icons/react/dist/csr/Question";
import { PlantIcon as Sprout } from "@phosphor-icons/react/dist/csr/Plant";
import { PlusIcon as Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { ArrowsClockwiseIcon as RefreshCw } from "@phosphor-icons/react/dist/csr/ArrowsClockwise";
import { command, today } from "../lib/client";
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
import { navigation } from "./navigation";
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
import { ErrorState, Loading, PageHeader } from "../components/ui/shared";
import {
  Overview,
  SetupDialog,
  Sales,
  OrderEditor,
  RecordDetail,
  Purchases,
  Customers,
  Inventory,
  Accounting,
  JournalDetail,
  JournalEditor,
  Reports,
  Settings,
  Onboarding,
  preloadPage,
  CommandMenu,
} from "./screens";
export default function App() {
  const [page, setPage] = useState<Page>("overview");
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
    () => localStorage.getItem("sarv.sidebar") === "collapsed",
  );
  const [palette, setPalette] = useState(false);
  const [help, setHelp] = useState(false);
  const [setup, setSetup] = useState(false);
  const [onboarding, setOnboarding] = useState(false);
  const [newCompany, setNewCompany] = useState(false);
  const [journalEditor, setJournalEditor] = useState(false);
  const [editor, setEditor] = useState<{ draft?: Draft } | null>(null);
  const [journal, setJournal] = useState<Journal | null>(null);
  const [record, setRecord] = useState<RecordEntry | null>(null);
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
      if (version === request.current) setError((e as Error).message);
    } finally {
      if (version === request.current) setRefreshing(false);
    }
  }, [companyId]);
  useEffect(() => {
    void refresh();
  }, [refresh]);
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
    const version = ++reportRequest.current;
    setReport(null);
    setReportError("");
    if (!workspace || !companyId) return;
    command<Report>("report", {
      company_id: companyId,
      from: range.from,
      to: range.to,
    })
      .then((r) => {
        if (version === reportRequest.current) {
          setReport(r);
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
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette((v) => !v);
      }
      if (e.key === "F9") {
        e.preventDefault();
        setCollapsed((v) => !v);
      }
      if (e.key === "F1") {
        e.preventDefault();
        setHelp(true);
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
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
    try {
      await command("settings", {
        ...workspace.settings,
        appearance: document.documentElement.classList.contains("dark")
          ? "Light"
          : "Dark",
      });
      await refresh();
    } catch (e) {
      toast.error((e as Error).message);
    }
  }
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
            openSetup: () => {
              setNewCompany(!companyId);
              setSetup(true);
            },
            openJournal: () => setJournalEditor(true),
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
            {editor ? (
              <OrderEditor
                initial={editor.draft}
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
            ) : page === "purchases" ? (
              <Purchases onDetail={setRecord} />
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
              <>
                <PageHeader title={t("Banking")} />
                <Empty>
                  <EmptyHeader>
                    <EmptyTitle>{t("Unavailable")}</EmptyTitle>
                  </EmptyHeader>
                  <EmptyContent>
                    <Button
                      variant="outline"
                      onClick={() => navigate("reports")}
                    >
                      {t("Open financial reports")}
                    </Button>
                  </EmptyContent>
                </Empty>
              </>
            )}
          </Suspense>
        </div>
        <Suspense fallback={null}>
          {setup && (
            <SetupDialog
              firstRun={onboarding}
              newCompany={newCompany}
              open={setup}
              onClose={() => {
                setSetup(false);
                setOnboarding(false);
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
        </Suspense>
      </BooksContext.Provider>
    ) : null;
  return (
    <I18nContext.Provider value={locale}>
      <DirectionProvider dir={locale === "fa" ? "rtl" : "ltr"}>
        <SidebarProvider
          open={!collapsed}
          onOpenChange={(open) => setCollapsed(!open)}
        >
          <div
            className={`app-shell ${workspace?.settings.density === "Compact" ? "compact" : ""}`}
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
                <SidebarGroup>
                  <SidebarGroupLabel>{t("Books")}</SidebarGroupLabel>
                  <SidebarMenu aria-label={t("Main navigation")}>
                    {navigation.slice(0, 7).map((n) => (
                      <SidebarMenuItem key={n.page}>
                        <SidebarMenuButton
                          isActive={page === n.page}
                          aria-current={page === n.page ? "page" : undefined}
                          tooltip={t(n.label)}
                          onPointerEnter={() => preloadPage(n.page)}
                          onFocus={() => preloadPage(n.page)}
                          onClick={() => navigate(n.page)}
                        >
                          <n.icon />
                          <span>{t(n.label)}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))}
                  </SidebarMenu>
                </SidebarGroup>
                <SidebarGroup>
                  <SidebarGroupLabel>
                    {companyId ? t("Organization") : t("Get started")}
                  </SidebarGroupLabel>
                  <SidebarMenu>
                    <SidebarMenuItem>
                      <SidebarMenuButton
                        tooltip={t("Add company")}
                        title={t("Create another company")}
                        onClick={() => {
                          if (editor) {
                            toast.info(
                              t(
                                "Save or discard the current sales draft first.",
                              ),
                            );
                            return;
                          }
                          setNewCompany(true);
                          setSetup(true);
                        }}
                      >
                        <Plus />
                        <span>{t("Add company")}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  </SidebarMenu>
                </SidebarGroup>
              </SidebarContent>
              <SidebarFooter>
                <SidebarGroup>
                  <SidebarGroupLabel>{t("Workspace")}</SidebarGroupLabel>
                  <SidebarMenu>
                    {navigation.slice(7).map((n) => (
                      <SidebarMenuItem key={n.page}>
                        <SidebarMenuButton
                          isActive={page === n.page}
                          aria-current={page === n.page ? "page" : undefined}
                          tooltip={t(n.label)}
                          onPointerEnter={() => preloadPage(n.page)}
                          onFocus={() => preloadPage(n.page)}
                          onClick={() => navigate(n.page)}
                        >
                          <n.icon />
                          <span>{t(n.label)}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))}
                    <SidebarMenuItem>
                      <SidebarMenuButton
                        tooltip={t("Help & shortcuts")}
                        onClick={() => setHelp(true)}
                      >
                        <CircleHelp />
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
        {palette && (
          <Suspense fallback={null}>
            <CommandMenu
              onClose={() => setPalette(false)}
              onNavigate={navigate}
              canPost={!!workspace?.initialized}
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
        <Modal
          open={help}
          onOpenChange={setHelp}
          title={t("Help & shortcuts")}
          description={t(
            "Local, offline accounting with exact amounts and traceable sources.",
          )}
        >
          <div className="setup-steps">
            <div className="summary-line">
              <span>{t("Command menu")}</span>
              <kbd>Ctrl / ⌘ K</kbd>
            </div>
            <div className="summary-line">
              <span>{t("Toggle sidebar")}</span>
              <kbd>F9</kbd>
            </div>
            <div className="summary-line">
              <span>{t("Help")}</span>
              <kbd>F1</kbd>
            </div>
            <div className="summary-line">
              <span>{t("Close dialog")}</span>
              <kbd>Esc</kbd>
            </div>
          </div>
        </Modal>
        <Toaster
          visibleToasts={1}
          duration={2500}
          dir={locale === "fa" ? "rtl" : "ltr"}
          position={locale === "fa" ? "bottom-left" : "bottom-right"}
          toastOptions={{ closeButtonAriaLabel: t("Close notification") }}
          containerAriaLabel={t("Notifications")}
          closeButton
        />
      </DirectionProvider>
    </I18nContext.Provider>
  );
}

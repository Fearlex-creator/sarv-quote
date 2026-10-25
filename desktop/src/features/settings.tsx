import { BusinessFields } from "../components/finance/business-fields";
import { ShortcutList } from "../components/finance/shortcut-list";
import { emptyBusinessProfile } from "../lib/types";
import { jevKey, jevReady, setJevConfig } from "../lib/jev";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command } from "../lib/client";
import type { Settings as Preferences } from "../lib/types";
import { PageHeader, ErrorState } from "../components/ui/shared";
import { Input } from "../components/ui/input";
import { Switch } from "../components/ui/switch";
import {
  SettingsBody,
  SettingsGroup,
  SettingsPage,
  SettingsRow,
} from "../components/ui/settings";
import { Button } from "../components/ui/button";
import { Agents } from "./agents";
import { Backups } from "./backups";
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "../components/ui/tabs";
type Tab = "general" | "backups" | "agents" | "shortcuts";

function Options<T extends string>({
  id,
  value,
  onChange,
  options,
  ...aria
}: {
  id?: string;
  "aria-describedby"?: string;
  value: T;
  onChange: (value: T) => void;
  options: [T, string][];
}) {
  return (
    <NativeSelect
      {...aria}
      id={id}
      className="w-full!"
      value={value}
      onChange={(e) => onChange(e.target.value as T)}
    >
      {options.map(([v, label]) => (
        <NativeSelectOption key={v} value={v}>
          {label}
        </NativeSelectOption>
      ))}
    </NativeSelect>
  );
}

export function Settings({
  onLocale,
}: {
  onLocale: (locale: "en" | "fa") => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [settings, setSettings] = useState(b.workspace.settings);
  const [profile, setProfile] = useState(
    b.workspace.business_profile ?? emptyBusinessProfile,
  );
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<Tab>("general");
  const [aiEnabled, setAiEnabled] = useState(() => jevReady());
  const [aiKey, setAiKey] = useState(() => jevKey());
  const company = b.workspace.companies.find((c) => c.id === b.companyId);
  function update<K extends keyof Preferences>(key: K, value: Preferences[K]) {
    setSettings((s) => ({ ...s, [key]: value }));
  }
  return (
    <Tabs
      value={tab}
      onValueChange={(value) => setTab(value as Tab)}
      dir={b.locale === "fa" ? "rtl" : "ltr"}
      className="gap-6"
    >
      <PageHeader
        title={b.t("Settings")}
        actions={
          tab === "general" && (
            <Button type="submit" form="settings-general" disabled={busy}>
              {busy ? t("Saving…") : t("Save preferences")}
            </Button>
          )
        }
      />
      <TabsList
        variant="line"
        aria-label={t("Settings")}
        className="h-10 w-full justify-start gap-4 overflow-x-auto rounded-none border-b border-border/70 p-0"
      >
        {(
          [
            ["general", "General"],
            ["backups", "Backups"],
            ["agents", "Agents"],
            ["shortcuts", "Keyboard shortcuts"],
          ] as [Tab, string][]
        ).map(([value, label]) => (
          <TabsTrigger
            key={value}
            value={value}
            className="h-10 flex-none rounded-none px-0 text-sm after:bottom-[-1px]"
          >
            {t(label)}
          </TabsTrigger>
        ))}
      </TabsList>
      {tab === "shortcuts" ? (
        <TabsContent value="shortcuts">
          <SettingsPage>
            <ShortcutList />
          </SettingsPage>
        </TabsContent>
      ) : tab === "agents" ? (
        <TabsContent value="agents">
          <Agents />
        </TabsContent>
      ) : tab === "backups" ? (
        <TabsContent value="backups">
          <Backups />
        </TabsContent>
      ) : (
        <TabsContent value="general">
          <form
            id="settings-general"
            onSubmit={async (e) => {
              e.preventDefault();
              setBusy(true);
              setError("");
              try {
                await command("settings", settings);
                if (b.companyId)
                  await command("business_profile", {
                    company_id: b.companyId,
                    profile,
                  });
                await b.refresh();
                toast.success(t("Preferences saved"));
              } catch (e) {
                setError((e as Error).message);
              } finally {
                setBusy(false);
              }
            }}
          >
            <SettingsPage>
              {error && <ErrorState error={error} />}
              <SettingsGroup
                title={t("Appearance and language")}
                description={t(
                  "Language is saved on this device. Financial data stays unchanged.",
                )}
              >
                <SettingsRow label={t("Theme")}>
                  <Options
                    value={settings.appearance}
                    onChange={(v) => update("appearance", v)}
                    options={[
                      ["System", t("System")],
                      ["Light", t("Light")],
                      ["Dark", t("Dark")],
                    ]}
                  />
                </SettingsRow>
                <SettingsRow label={t("Density")}>
                  <Options
                    value={settings.density}
                    onChange={(v) => update("density", v)}
                    options={[
                      ["Comfortable", t("Comfortable")],
                      ["Compact", t("Compact")],
                    ]}
                  />
                </SettingsRow>
                <SettingsRow label={t("Interface language")}>
                  <Options
                    value={b.locale}
                    onChange={onLocale}
                    options={[
                      ["en", t("English")],
                      ["fa", "فارسی"],
                    ]}
                  />
                </SettingsRow>
              </SettingsGroup>
              <SettingsGroup title={t("Formats")}>
                <SettingsRow label={t("Amount display")}>
                  <Options
                    value={settings.default_currency}
                    onChange={(v) => update("default_currency", v)}
                    options={[
                      ["Rial", t("Iranian rial · IRR")],
                      ["Toman", t("Toman · 10 IRR")],
                    ]}
                  />
                </SettingsRow>
                <SettingsRow label={t("Date display")}>
                  <Options
                    value={settings.date_display ?? "Iso"}
                    onChange={(v) => update("date_display", v)}
                    options={[
                      ["Iso", t("Gregorian")],
                      ["DayMonthYear", t("Day / month / year")],
                      ["Jalali", t("Jalali")],
                    ]}
                  />
                </SettingsRow>
                <SettingsRow label={t("Display digits")}>
                  <Options
                    value={settings.display_digits}
                    onChange={(v) => update("display_digits", v)}
                    options={[
                      ["Latin", t("Latin")],
                      ["Persian", t("Persian")],
                    ]}
                  />
                </SettingsRow>
              </SettingsGroup>
              <SettingsGroup
                title={t("Businesses")}
                description={t(
                  "Each business has separate books and fiscal periods.",
                )}
                action={
                  <Button
                    type="button"
                    variant="outline"
                    onClick={b.openCompanySetup}
                  >
                    {t("Add company")}
                  </Button>
                }
              >
                <SettingsBody className="px-5 py-4">
                  <ul className="flex flex-col gap-1 text-sm">
                    {b.workspace.companies.map((c) => (
                      <li
                        key={c.id}
                        dir="auto"
                        className="flex items-center justify-between gap-3"
                      >
                        <span className="font-medium">{c.name}</span>
                        {c.id === b.companyId && (
                          <span className="text-[13px] text-muted-foreground">
                            {t("Current")}
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>
                </SettingsBody>
              </SettingsGroup>
              <SettingsGroup
                title={t("Business and invoice details")}
                description={company?.name}
              >
                <SettingsBody>
                  <BusinessFields value={profile} onChange={setProfile} />
                </SettingsBody>
              </SettingsGroup>
              <SettingsGroup
                title={t("AI assistance")}
                badge={<span className="beta-badge !ms-0">Beta</span>}
                description={t(
                  "Optional decision-model features (contact and product suggestions). Off by default; works entirely without it.",
                )}
                footer={t(
                  "The key stays on this device. AI judgments are advisory and never change your books.",
                )}
              >
                <SettingsRow
                  label={t("Enable AI assistance")}
                  description={t(
                    "When on, a compact digest of the relevant figures is sent to the TypeSafe API with your key. Nothing is sent while it is off.",
                  )}
                >
                  <Switch
                    checked={aiEnabled}
                    onCheckedChange={(checked) => {
                      setAiEnabled(checked);
                      setJevConfig(checked, aiKey);
                    }}
                  />
                </SettingsRow>
                <SettingsRow label={t("TypeSafe API key")}>
                  <Input
                    type="password"
                    autoComplete="off"
                    dir="ltr"
                    value={aiKey}
                    placeholder="tsk-…"
                    onChange={(e) => {
                      setAiKey(e.target.value);
                      setJevConfig(aiEnabled, e.target.value);
                    }}
                  />
                </SettingsRow>
              </SettingsGroup>
            </SettingsPage>
          </form>
        </TabsContent>
      )}
    </Tabs>
  );
}

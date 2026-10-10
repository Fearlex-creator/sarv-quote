import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import { Textarea } from "../components/ui/textarea";
import { Input } from "../components/ui/input";
import { useTranslation } from "../lib/i18n";
import { useState } from "react";
import { toast } from "sonner";
import { useBooks } from "../app/books";
import { command } from "../lib/client";
import type { Settings as Preferences } from "../lib/types";
import { PageHeader, Field, ErrorState } from "../components/ui/shared";
import { Button } from "../components/ui/button";
import { Agents } from "./agents";
import { tabKeyboard } from "../lib/tab-keyboard";
export function Settings({
  onLocale,
}: {
  onLocale: (locale: "en" | "fa") => void;
}) {
  const t = useTranslation();
  const b = useBooks();
  const [settings, setSettings] = useState(b.workspace.settings);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<"general" | "agents">("general");
  function update<K extends keyof Preferences>(key: K, value: Preferences[K]) {
    setSettings((s) => ({ ...s, [key]: value }));
  }
  return (
    <>
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
      <div
        className="tabs"
        role="tablist"
        aria-label={t("Settings")}
        onKeyDown={tabKeyboard}
      >
        <button
          type="button"
          role="tab"
          id="settings-general-tab"
          aria-controls="settings-general"
          aria-selected={tab === "general"}
          tabIndex={tab === "general" ? 0 : -1}
          className="tab"
          onClick={() => setTab("general")}
        >
          {t("General")}
        </button>
        <button
          type="button"
          role="tab"
          id="settings-agents-tab"
          aria-controls="settings-agents"
          aria-selected={tab === "agents"}
          tabIndex={tab === "agents" ? 0 : -1}
          className="tab"
          onClick={() => setTab("agents")}
        >
          {t("Agents")}
        </button>
      </div>
      {tab === "agents" ? (
        <div
          id="settings-agents"
          role="tabpanel"
          aria-labelledby="settings-agents-tab"
        >
          <Agents />
        </div>
      ) : (
        <form
          id="settings-general"
          role="tabpanel"
          aria-labelledby="settings-general-tab"
          onSubmit={async (e) => {
            e.preventDefault();
            setBusy(true);
            setError("");
            try {
              await command("settings", settings);
              await b.refresh();
              toast.success(t("Preferences saved"));
            } catch (e) {
              setError((e as Error).message);
            } finally {
              setBusy(false);
            }
          }}
        >
          {error && <ErrorState error={error} />}
          <div className="settings-layout">
            <section className="settings-section">
              <h2>{t("Appearance and language")}</h2>
              <div className="grouped-list">
                <Field label={t("Theme")}>
                  <NativeSelect
                    value={settings.appearance}
                    onChange={(e) =>
                      update(
                        "appearance",
                        e.target.value as Preferences["appearance"],
                      )
                    }
                  >
                    <NativeSelectOption value="System">
                      {t("System")}
                    </NativeSelectOption>
                    <NativeSelectOption value="Light">
                      {t("Light")}
                    </NativeSelectOption>
                    <NativeSelectOption value="Dark">
                      {t("Dark")}
                    </NativeSelectOption>
                  </NativeSelect>
                </Field>
                <Field label={t("Density")}>
                  <NativeSelect
                    value={settings.density}
                    onChange={(e) =>
                      update(
                        "density",
                        e.target.value as Preferences["density"],
                      )
                    }
                  >
                    <NativeSelectOption value="Comfortable">
                      {t("Comfortable")}
                    </NativeSelectOption>
                    <NativeSelectOption value="Compact">
                      {t("Compact")}
                    </NativeSelectOption>
                  </NativeSelect>
                </Field>
                <Field
                  label={t("Interface language")}
                  hint={t(
                    "Language is saved on this device. Financial data stays unchanged.",
                  )}
                >
                  <NativeSelect
                    value={b.locale}
                    onChange={(e) => onLocale(e.target.value as "en" | "fa")}
                  >
                    <NativeSelectOption value="en">
                      {t("English")}
                    </NativeSelectOption>
                    <NativeSelectOption value="fa">فارسی</NativeSelectOption>
                  </NativeSelect>
                </Field>
                <Field label={t("Amount display")}>
                  <NativeSelect
                    value={settings.default_currency}
                    onChange={(e) =>
                      update(
                        "default_currency",
                        e.target.value as "Rial" | "Toman",
                      )
                    }
                  >
                    <NativeSelectOption value="Rial">
                      {t("Iranian rial · IRR")}
                    </NativeSelectOption>
                    <NativeSelectOption value="Toman">
                      {t("Toman · 10 IRR")}
                    </NativeSelectOption>
                  </NativeSelect>
                </Field>
                <Field label={t("Date display")}>
                  <NativeSelect
                    value={settings.date_display ?? "Iso"}
                    onChange={(e) =>
                      update(
                        "date_display",
                        e.target.value as Preferences["date_display"],
                      )
                    }
                  >
                    <NativeSelectOption value="Iso">
                      {t("Gregorian")}
                    </NativeSelectOption>
                    <NativeSelectOption value="DayMonthYear">
                      {t("Day / month / year")}
                    </NativeSelectOption>
                    <NativeSelectOption value="Jalali">
                      {t("Jalali")}
                    </NativeSelectOption>
                  </NativeSelect>
                </Field>
                <Field label={t("Display digits")}>
                  <NativeSelect
                    value={settings.display_digits}
                    onChange={(e) =>
                      update(
                        "display_digits",
                        e.target.value as Preferences["display_digits"],
                      )
                    }
                  >
                    <NativeSelectOption value="Latin">
                      {t("Latin")}
                    </NativeSelectOption>
                    <NativeSelectOption value="Persian">
                      {t("Persian")}
                    </NativeSelectOption>
                  </NativeSelect>
                </Field>
              </div>
            </section>
            <section className="settings-section">
              <h2>{t("Quotation issuer details")}</h2>
              <div className="grouped-list">
                <Field label={t("Business name")}>
                  <Input
                    value={settings.business_name}
                    onChange={(e) => update("business_name", e.target.value)}
                  />
                </Field>
                <Field label={t("Business phone")}>
                  <Input
                    value={settings.business_phone}
                    onChange={(e) => update("business_phone", e.target.value)}
                  />
                </Field>
                <Field label={t("Business address")}>
                  <Textarea
                    value={settings.business_address}
                    onChange={(e) => update("business_address", e.target.value)}
                  />
                </Field>
                <Field label={t("Default quotation terms")}>
                  <Textarea
                    value={settings.default_terms}
                    onChange={(e) => update("default_terms", e.target.value)}
                  />
                </Field>
              </div>
            </section>
          </div>
        </form>
      )}
    </>
  );
}

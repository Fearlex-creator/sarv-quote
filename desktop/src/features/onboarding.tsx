import { PlantIcon as Sprout } from "@phosphor-icons/react/dist/csr/Plant";
import { BuildingsIcon as Building2 } from "@phosphor-icons/react/dist/csr/Buildings";
import { CalendarDotsIcon as CalendarDays } from "@phosphor-icons/react/dist/csr/CalendarDots";
import { BookOpenIcon as BookOpen } from "@phosphor-icons/react/dist/csr/BookOpen";
import { ArrowRightIcon as ArrowRight } from "@phosphor-icons/react/dist/csr/ArrowRight";
import { ShieldCheckIcon as ShieldCheck } from "@phosphor-icons/react/dist/csr/ShieldCheck";
import { GlobeHemisphereEastIcon as Globe } from "@phosphor-icons/react/dist/csr/GlobeHemisphereEast";
import { useBooks } from "../app/books";
import { Button } from "../components/ui/button";
import {
  NativeSelect,
  NativeSelectOption,
} from "../components/ui/native-select";
import type { Locale } from "../lib/i18n";

export function Onboarding({
  onStart,
  onLocale,
}: {
  onStart: () => void;
  onLocale: (locale: Locale) => void;
}) {
  const { t, locale } = useBooks();
  const steps = [
    [
      Building2,
      "Company",
      "Give your business a home in Sarv.",
    ],
    [
      CalendarDays,
      "Fiscal period",
      "Choose the dates that fit your financial year.",
    ],
    [
      BookOpen,
      "Opening balances",
      "Start fresh or bring in your existing balances.",
    ],
  ] as const;
  const format =
    locale === "fa"
      ? "Persian · Jalali calendar · Persian digits"
      : "English · Gregorian calendar · Latin digits";

  return (
    <section className="onboarding" aria-label={t("First-time setup")}>
      <div className="welcome-stage">
        <div className="welcome-mark" aria-hidden="true">
          <Sprout />
        </div>

        <header className="welcome-heading">
          <h1>{t("Welcome to Sarv")}</h1>
          <p>
            {t(
              "Set up your company, choose your financial year, and begin with balances you can trust.",
            )}
          </p>
        </header>

        <div className="welcome-group">
          <p className="welcome-group-title">{t("What you’ll set up")}</p>
          <ol className="welcome-steps" aria-label={t("What you’ll set up")}>
            {steps.map(([Icon, label, description]) => (
              <li className="welcome-step" key={label}>
                <span className="welcome-step-icon" aria-hidden="true">
                  <Icon />
                </span>
                <span className="welcome-step-copy">
                  <strong>{t(label)}</strong>
                  <p>{t(description)}</p>
                </span>
              </li>
            ))}
            <li className="welcome-step welcome-language">
              <span className="welcome-step-icon" aria-hidden="true">
                <Globe />
              </span>
              <span className="welcome-step-copy">
                <strong>{t("Language")}</strong>
                <p className="welcome-calendar" aria-live="polite">
                  {t(format)}
                </p>
              </span>
              <NativeSelect
                className="welcome-language-select"
                aria-label={t("App language")}
                value={locale}
                onChange={(e) => onLocale(e.target.value as Locale)}
              >
                <NativeSelectOption value="fa">فارسی</NativeSelectOption>
                <NativeSelectOption value="en">English</NativeSelectOption>
              </NativeSelect>
            </li>
          </ol>
        </div>

        <Button className="welcome-cta" size="lg" onClick={onStart}>
          {t("Get started")}
          <ArrowRight className="directional-icon" data-icon="inline-end" />
        </Button>

        <p className="welcome-footnote">
          <ShieldCheck aria-hidden="true" />
          {t("Works offline · Saved on this device")}
        </p>
      </div>
    </section>
  );
}

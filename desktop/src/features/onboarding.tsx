import { PlantIcon as Sprout } from "@phosphor-icons/react/dist/csr/Plant";
import { BuildingsIcon as Building2 } from "@phosphor-icons/react/dist/csr/Buildings";
import { CalendarDotsIcon as CalendarDays } from "@phosphor-icons/react/dist/csr/CalendarDots";
import { BookOpenIcon as BookOpen } from "@phosphor-icons/react/dist/csr/BookOpen";
import { ArrowRightIcon as ArrowRight } from "@phosphor-icons/react/dist/csr/ArrowRight";
import { useBooks } from "../app/books";
import { Button } from "../components/ui/button";
import { Field, PageHeader } from "../components/ui/shared";
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
  return (
    <>
      <PageHeader title={t("Welcome to Sarv")} />
      <section className="onboarding" aria-label={t("First-time setup")}>
        <div className="welcome-mark" aria-hidden="true">
          <Sprout />
        </div>
        <div className="grouped-list welcome-steps">
          {[
            [Building2, "Company"],
            [CalendarDays, "Fiscal period"],
            [BookOpen, "Opening balances"],
          ].map(([Icon, label]) => {
            const StepIcon = Icon as typeof Building2;
            return (
              <div className="welcome-step" key={String(label)}>
                <StepIcon aria-hidden="true" />
                <span>{t(String(label))}</span>
              </div>
            );
          })}
        </div>
        <Field label={t("App language")}>
          <NativeSelect
            value={locale}
            onChange={(e) => onLocale(e.target.value as Locale)}
          >
            <NativeSelectOption value="fa">فارسی</NativeSelectOption>
            <NativeSelectOption value="en">English</NativeSelectOption>
          </NativeSelect>
        </Field>
        <Button onClick={onStart}>
          {t("Get started")}
          <ArrowRight className="directional-icon" data-icon="inline-end" />
        </Button>
      </section>
    </>
  );
}

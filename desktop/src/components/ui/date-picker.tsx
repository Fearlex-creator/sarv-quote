import {
  faIR as persianLocale,
  enUS as englishPersianLocale,
} from "@daypicker/persian";
import { faIR as persianGregorianLocale } from "react-day-picker/locale";
import { BooksContext } from "../../app/books";
import { useTranslation, I18nContext } from "../../lib/i18n";
import { useContext, useState } from "react";
import { CalendarDotsIcon } from "@phosphor-icons/react/dist/csr/CalendarDots";
import { Button } from "./button";
import { Calendar } from "./calendar";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import { formatCalendarDate, isoDate, parseDate } from "../../lib/calendar";
import type { Locale } from "../../lib/i18n";

export function DatePicker({
  value,
  onChange,
  locale,
  min,
  calendar: calendarOverride,
  id,
  "aria-label": label,
  "aria-describedby": description,
}: {
  value: string;
  onChange: (value: string) => void;
  locale?: Locale;
  calendar?: "persian" | "gregory";
  "aria-label"?: string;
  min?: string;
  id?: string;
  "aria-describedby"?: string;
}) {
  const t = useTranslation();
  const books = useContext(BooksContext);
  const parsed = parseDate(value);
  const selected =
    value && !Number.isNaN(parsed.getTime()) ? parsed : undefined;
  const currentYear = new Date().getFullYear();
  const selectedYear = selected?.getFullYear() ?? currentYear;
  const language = useContext(I18nContext);
  const activeLocale = locale ?? books?.locale ?? language;
  const calendar =
    calendarOverride ??
    (books?.workspace.settings.date_display === "Jalali"
      ? "persian"
      : books?.workspace.companies.length
        ? "gregory"
        : activeLocale === "fa"
          ? "persian"
          : "gregory");
  const [open, setOpen] = useState(false);
  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          id={id}
          aria-describedby={description}
          aria-label={label}
          type="button"
          variant="outline"
          className="w-full justify-between"
        >
          {selected
            ? formatCalendarDate(value, activeLocale, calendar)
            : t("Choose date")}
          <CalendarDotsIcon data-icon="inline-end" />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="start"
        className="w-auto p-0"
        dir={activeLocale === "fa" ? "rtl" : "ltr"}
      >
        <Calendar
          calendar={calendar === "persian" ? "persian" : "gregorian"}
          locale={
            calendar === "persian"
              ? activeLocale === "fa"
                ? persianLocale
                : englishPersianLocale
              : activeLocale === "fa"
                ? persianGregorianLocale
                : undefined
          }
          dir={activeLocale === "fa" ? "rtl" : "ltr"}
          numerals={activeLocale === "fa" ? "arabext" : "latn"}
          mode="single"
          required
          captionLayout="dropdown"
          startMonth={new Date(Math.min(currentYear - 100, selectedYear), 0, 1)}
          endMonth={new Date(Math.max(currentYear + 20, selectedYear), 11, 31)}
          defaultMonth={selected}
          selected={selected}
          disabled={min ? { before: parseDate(min) } : undefined}
          onSelect={(date) => {
            if (date) {
              onChange(isoDate(date));
              setOpen(false);
            }
          }}
        />
      </PopoverContent>
    </Popover>
  );
}

import { useTranslation } from "../../lib/i18n";
import { WarningCircleIcon as AlertCircle } from "@phosphor-icons/react/dist/csr/WarningCircle";
import {
  cloneElement,
  isValidElement,
  useId,
  useContext,
  type ReactElement,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { BooksContext } from "../../app/books";
import { Button } from "./button";
import { Skeleton } from "./skeleton";
import { Field as FieldPrimitive, FieldLabel } from "./field";
import { Alert, AlertDescription, AlertTitle } from "./alert";
import { DatePicker } from "./date-picker";
export function PageHeader({
  title,
  actions,
  syncedAt,
}: {
  title: string;
  actions?: ReactNode;
  syncedAt?: Date | null;
}) {
  const t = useTranslation();
  const books = useContext(BooksContext);
  const timestamp = syncedAt === undefined ? books?.lastSyncedAt : syncedAt;
  const heading = (
    <header className="page-heading">
      <div>
        <h1>{title}</h1>
      </div>
      <div className="toolbar-actions">{actions}</div>
    </header>
  );
  return (
    <>
      {books?.headerTarget
        ? createPortal(heading, books.headerTarget)
        : heading}
      {timestamp && (
        <div className="sync-time">
          {t("Last synced")}{" "}
          <time dateTime={timestamp.toISOString()}>
            {new Intl.DateTimeFormat(books?.dateLocale ?? "en-US", {
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
            }).format(timestamp)}
          </time>
        </div>
      )}
    </>
  );
}
export function Field({
  label,
  children,
  hint,
  action,
}: {
  label: string;
  children: ReactNode;
  hint?: string;
  action?: ReactNode;
}) {
  const id = useId();
  const control = isValidElement(children)
    ? cloneElement(
        children as ReactElement<{ id: string; "aria-describedby"?: string }>,
        { id, "aria-describedby": hint ? `${id}-hint` : undefined },
      )
    : children;
  return (
    <FieldPrimitive className="field">
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      {control}
      {action}
      {hint && (
        <span className="sr-only w-px!" id={`${id}-hint`}>
          {hint}
        </span>
      )}
    </FieldPrimitive>
  );
}
export function ErrorState({
  error,
  retry,
}: {
  error: string;
  retry?: () => void;
}) {
  const t = useTranslation();
  return (
    <Alert variant="destructive">
      <AlertCircle />
      <AlertTitle>{t(error)}</AlertTitle>
      <AlertDescription>
        {retry && (
          <Button variant="outline" onClick={retry}>
            {t("Try again")}
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
}
export function Loading({ shape = "table" }: { shape?: "table" | "form" }) {
  const t = useTranslation();
  return (
    <div
      className={`loading skeleton-${shape}`}
      role="status"
      aria-label={t("Loading")}
    >
      <span className="sr-only">{t("Loading…")}</span>
      {Array.from({ length: shape === "table" ? 6 : 4 }, (_, i) => (
        <div className="skeleton-row" aria-hidden="true" key={i}>
          <Skeleton className="h-3 w-full" />
          <Skeleton className="h-3 w-full" />
          {shape === "table" && <Skeleton className="h-3 w-full" />}
        </div>
      ))}
    </div>
  );
}
export function Note({ children }: { children: ReactNode }) {
  return (
    <Alert>
      <AlertDescription>{children}</AlertDescription>
    </Alert>
  );
}

/** From / Through pickers on one compact row, with an optional trailing note. */
export function DateRange({
  from,
  to,
  onChange,
  children,
}: {
  from: string;
  to: string;
  onChange: (from: string, to: string) => void;
  children?: ReactNode;
}) {
  const t = useTranslation();
  return (
    <div className="date-range">
      <Field label={t("From")}>
        <DatePicker value={from} onChange={(v) => onChange(v, to)} />
      </Field>
      <Field label={t("Through")}>
        <DatePicker value={to} onChange={(v) => onChange(from, v)} />
      </Field>
      {children && <span className="date-range-note">{children}</span>}
    </div>
  );
}

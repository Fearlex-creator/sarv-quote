import { useTranslation } from "../../lib/i18n";
import { AlertCircle } from "lucide-react";
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
}: {
  label: string;
  children: ReactNode;
  hint?: string;
}) {
  const id = useId();
  const control = isValidElement(children)
    ? cloneElement(
        children as ReactElement<{ id: string; "aria-describedby"?: string }>,
        { id, "aria-describedby": hint ? `${id}-hint` : undefined },
      )
    : children;
  return (
    <div className="field" data-slot="field">
      <label htmlFor={id}>{label}</label>
      {control}
      {hint && (
        <span className="sr-only" id={`${id}-hint`}>
          {hint}
        </span>
      )}
    </div>
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
    <div className="error-state" role="alert">
      <AlertCircle />
      <div>
        <p>{t(error)}</p>
        {retry && (
          <Button variant="outline" onClick={retry}>
            {t("Try again")}
          </Button>
        )}
      </div>
    </div>
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
          <Skeleton />
          <Skeleton />
          {shape === "table" && <Skeleton />}
        </div>
      ))}
    </div>
  );
}
export function Note({ children }: { children: ReactNode }) {
  return <div className="note">{children}</div>;
}

import {
  cloneElement,
  isValidElement,
  useId,
  type ReactElement,
  type ReactNode,
} from "react";
import { cn } from "cn";

/** Page column shared by every settings tab, so tabs never jump in width. */
export function SettingsPage({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return (
    <div
      className={cn("flex w-full flex-col gap-8", className)}
      {...props}
    />
  );
}

/** A titled card of divided rows, the unit every settings screen is built from. */
export function SettingsGroup({
  title,
  description,
  badge,
  action,
  footer,
  children,
  className,
}: {
  title: string;
  description?: ReactNode;
  badge?: ReactNode;
  action?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  const id = useId();
  return (
    <section
      aria-labelledby={id}
      className={cn("flex flex-col gap-3", className)}
    >
      <header className="flex items-end justify-between gap-4">
        <div className="min-w-0">
          <h2
            id={id}
            className="flex items-center gap-2 text-base leading-6 font-semibold"
          >
            {title}
            {badge}
          </h2>
          {description && (
            <p className="mt-0.5 text-sm leading-5 text-muted-foreground">
              {description}
            </p>
          )}
        </div>
        {action}
      </header>
      <div className="divide-y divide-border/60 overflow-hidden rounded-xl border border-border/70 bg-card shadow-xs">
        {children}
      </div>
      {footer && (
        <p className="text-sm leading-5 text-muted-foreground">
          {footer}
        </p>
      )}
    </section>
  );
}

/**
 * Label and description on one side, control on the other. When the control is
 * a single element it receives the generated id so the label stays clickable.
 */
export function SettingsRow({
  label,
  description,
  children,
  stacked,
}: {
  label: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  /** Put the control below the label (textareas, long values). */
  stacked?: boolean;
}) {
  const id = useId();
  const control =
    isValidElement(children) && typeof children.type !== "symbol"
      ? cloneElement(
          children as ReactElement<{ id?: string; "aria-describedby"?: string }>,
          {
            id: (children.props as { id?: string }).id ?? id,
            "aria-describedby": description ? `${id}-description` : undefined,
          },
        )
      : children;
  const controlId =
    isValidElement(children) && (children.props as { id?: string }).id
      ? (children.props as { id: string }).id
      : id;
  return (
    <div
      className={cn(
        "flex gap-x-6 gap-y-3 px-5 py-4 text-sm",
        stacked
          ? "flex-col"
          : "flex-col sm:flex-row sm:items-center sm:justify-between",
      )}
    >
      <div className="min-w-0 sm:max-w-[55%] sm:flex-1">
        <label htmlFor={controlId} className="block leading-5 font-medium">
          {label}
        </label>
        {description && (
          <p
            id={`${id}-description`}
            className="mt-0.5 text-[13px] leading-5 text-muted-foreground"
          >
            {description}
          </p>
        )}
      </div>
      {control !== undefined && control !== null && (
        <div
          className={cn(
            "flex shrink-0 items-center sm:justify-end",
            stacked ? "w-full" : "w-full sm:w-72",
          )}
        >
          {control}
        </div>
      )}
    </div>
  );
}

/** A row with no form control, for read-only values and inline actions. */
export function SettingsItem({
  label,
  description,
  children,
}: {
  label: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-3 px-5 py-4 text-sm sm:flex-row sm:items-center sm:justify-between sm:gap-6">
      <div className="min-w-0">
        <div className="leading-5 font-medium">{label}</div>
        {description && (
          <p className="mt-0.5 text-[13px] leading-5 text-muted-foreground">
            {description}
          </p>
        )}
      </div>
      {children && (
        <div className="flex shrink-0 items-center gap-2">{children}</div>
      )}
    </div>
  );
}

/** Free-form content inside a group (tables, field grids) with matching padding. */
export function SettingsBody({
  className,
  ...props
}: React.ComponentProps<"div">) {
  return <div className={cn("p-5", className)} {...props} />;
}

import { useId, useRef, type ReactNode } from "react";
import { XIcon } from "@phosphor-icons/react/dist/csr/X";
import { useTranslation } from "../../lib/i18n";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";

// Financial dialogs share the preset primitives while keeping localized close
// labels, editor autofocus and independently scrolling posting reviews.
export function Modal({
  open,
  onOpenChange,
  title,
  description,
  children,
  wide = false,
  footer,
  confirmation = false,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  children: ReactNode;
  wide?: boolean;
  footer?: ReactNode;
  confirmation?: boolean;
}) {
  const t = useTranslation();
  const content = useRef<HTMLDivElement>(null);
  const descriptionId = useId();
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        ref={content}
        showCloseButton={false}
        className={cn(
          "max-h-[90dvh] overflow-y-auto",
          wide && "sm:max-w-4xl",
          footer && "flex flex-col overflow-hidden",
        )}
        aria-describedby={description ? descriptionId : undefined}
        onOpenAutoFocus={(event) => {
          const target =
            content.current?.querySelector<HTMLElement>(
              '[data-autofocus="true"]',
            ) ??
            content.current?.querySelector<HTMLElement>(
              "input:not([disabled]), textarea:not([disabled]), select:not([disabled])",
            );
          if (target) {
            event.preventDefault();
            target.focus();
          }
        }}
      >
        <DialogHeader className="shrink-0 pe-10">
          <DialogTitle>{title}</DialogTitle>
          {description && (
            <DialogDescription
              className={confirmation ? "confirmation-text" : "sr-only"}
              id={descriptionId}
            >
              {description}
            </DialogDescription>
          )}
        </DialogHeader>
        <DialogClose asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            className="absolute top-4 end-4"
            aria-label={t("Close dialog")}
          >
            <XIcon />
          </Button>
        </DialogClose>
        {footer ? (
          <div className="min-h-0 overflow-y-auto">{children}</div>
        ) : (
          children
        )}
        {footer && <DialogFooter className="shrink-0">{footer}</DialogFooter>}
      </DialogContent>
    </Dialog>
  );
}

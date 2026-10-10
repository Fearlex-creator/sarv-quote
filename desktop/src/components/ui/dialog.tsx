import { useTranslation } from "../../lib/i18n";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { useId, useRef, type ReactNode } from "react";
import { Button } from "./button";
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
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="dialog-overlay" />
        <DialogPrimitive.Content
          ref={content}
          className={`dialog-content ${wide ? "dialog-wide" : ""} ${footer ? "dialog-with-footer" : ""}`}
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
          <div className="dialog-heading">
            <div>
              <DialogPrimitive.Title>{title}</DialogPrimitive.Title>
              {description && (
                <DialogPrimitive.Description
                  className={confirmation ? "confirmation-text" : "sr-only"}
                  id={descriptionId}
                >
                  {description}
                </DialogPrimitive.Description>
              )}
            </div>
            <DialogPrimitive.Close asChild>
              <Button
                variant="ghost"
                size="icon"
                aria-label={t("Close dialog")}
              >
                <X />
              </Button>
            </DialogPrimitive.Close>
          </div>
          {footer ? <div className="dialog-scroll">{children}</div> : children}
          {footer}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

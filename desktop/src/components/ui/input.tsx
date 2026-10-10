import * as React from "react";
import { cn } from "../../lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      dir={
        type === "date" ||
        type === "email" ||
        type === "tel" ||
        props.inputMode === "numeric" ||
        props.inputMode === "decimal"
          ? "ltr"
          : "auto"
      }
      data-slot="input"
      className={cn(
        "w-full min-w-0 rounded-md border border-border/60 bg-transparent px-3 py-2 text-sm transition-colors selection:bg-primary selection:text-primary-foreground placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50",
        "focus-visible:outline-2 focus-visible:outline-ring aria-invalid:border-destructive",
        className,
      )}
      {...props}
    />
  );
}

export { Input };

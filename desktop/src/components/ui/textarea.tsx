import * as React from "react";
import { cn } from "../../lib/utils";

function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      dir="auto"
      className={cn(
        "min-h-16 w-full rounded-md border border-border/60 bg-transparent px-3 py-2 text-sm transition-colors placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50",
        "focus-visible:outline-2 focus-visible:outline-ring aria-invalid:border-destructive",
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };

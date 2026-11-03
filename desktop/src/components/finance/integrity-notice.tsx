import { useEffect } from "react";
import { Alert, AlertDescription, AlertTitle } from "../ui/alert";
import { useTranslation } from "../../lib/i18n";
import type { IntegrityStatus } from "../../lib/types";

/**
 * Shows the result of the full integrity check that runs after an import. Silent while the
 * book is healthy. While a check is running it re-reads the workspace until it finishes.
 */
export function IntegrityNotice({
  status,
  refresh,
}: {
  status?: IntegrityStatus;
  refresh: () => Promise<void>;
}) {
  const t = useTranslation();
  const running = status?.status === "running";
  useEffect(() => {
    if (!running) return;
    const id = window.setTimeout(() => void refresh(), 1500);
    return () => window.clearTimeout(id);
  }, [running, refresh]);
  if (status?.status !== "failed") return null;
  return (
    <Alert variant="destructive" data-testid="integrity-failed">
      <AlertTitle>{t("The integrity check found a problem")}</AlertTitle>
      <AlertDescription>
        <p>{status.message}</p>
        <p>
          {t(
            "Closing periods is blocked until this is resolved. Your data has not been changed.",
          )}
        </p>
      </AlertDescription>
    </Alert>
  );
}

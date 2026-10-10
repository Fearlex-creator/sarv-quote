import { useTranslation } from "./i18n";
import { useEffect, useRef } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
export function useUnsavedChanges(dirty: boolean) {
  const t = useTranslation();
  const discard = useRef(false);
  const prompting = useRef(false);
  useEffect(() => {
    discard.current = false;
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (dirty && !isTauri()) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", beforeUnload);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    if (dirty && isTauri())
      void getCurrentWindow()
        .onCloseRequested(async (event) => {
          if (discard.current) return;
          event.preventDefault();
          if (prompting.current) return;
          prompting.current = true;
          try {
            if (
              await ask(
                t("Edits since the last save will be lost. Close the window?"),
                {
                  title: t("Unsaved changes"),
                  kind: "warning",
                  okLabel: t("Discard and close"),
                  cancelLabel: t("Keep editing"),
                },
              )
            ) {
              await getCurrentWindow().destroy();
              discard.current = true;
            }
          } catch (error) {
            toast.error(
              t("Unable to close the window: {error}", {
                error: String(error),
              }),
            );
          } finally {
            prompting.current = false;
          }
        })
        .then((fn) => {
          if (disposed) fn();
          else unlisten = fn;
        });
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener("beforeunload", beforeUnload);
    };
  }, [dirty, t]);
}

import { invoke, isTauri } from "@tauri-apps/api/core";
export class AccountingError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
  }
}
export async function command<T>(
  action: string,
  payload: unknown = {},
): Promise<T> {
  try {
    if (isTauri()) return await invoke<T>("accounting", { action, payload });
    if (
      import.meta.env.DEV &&
      import.meta.env.VITE_QA_URL &&
      import.meta.env.VITE_QA_TOKEN
    ) {
      const r = await fetch(import.meta.env.VITE_QA_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-sarv-qa-token": import.meta.env.VITE_QA_TOKEN,
        },
        body: JSON.stringify({ action, payload }),
      });
      const body = await r.json();
      if (body.error) throw body.error;
      return body.result as T;
    }
    throw new AccountingError(
      "desktop_required",
      "Open Sarv in the desktop application to access your books. Browser QA requires the explicit Rust test adapter.",
    );
  } catch (e) {
    if (e instanceof Error) throw e;
    const err = e as { code?: string; message?: string };
    throw new AccountingError(
      err.code ?? "command_failed",
      err.message ?? String(e),
    );
  }
}
export async function exportReport(
  action: string,
  payload: unknown,
): Promise<boolean> {
  if (isTauri()) return invoke<boolean>("save_report", { action, payload });
  const result = await command<{ filename: string; content: string }>(
    action,
    payload,
  );
  const blob = new Blob([result.content], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = result.filename;
  link.click();
  URL.revokeObjectURL(url);
  return true;
}
export const uid = () => crypto.randomUUID();
export const today = () =>
  new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Tehran",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date());

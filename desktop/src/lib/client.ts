import { invoke, isTauri } from "@tauri-apps/api/core";
import { sessionToken, writeSession } from "./session";
export class AccountingError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
  }
}
function withSession(payload: unknown) {
  const token = sessionToken();
  const body =
    payload && typeof payload === "object" && !Array.isArray(payload)
      ? { ...(payload as Record<string, unknown>) }
      : {};
  if (token) body.session_token = token;
  return body;
}
export async function command<T>(
  action: string,
  payload: unknown = {},
): Promise<T> {
  const body = withSession(payload);
  try {
    if (isTauri()) return await invoke<T>("accounting", { action, payload: body });
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
        body: JSON.stringify({ action, payload: body }),
      });
      const parsed = await r.json();
      if (parsed.error) throw parsed.error;
      return parsed.result as T;
    }
    throw new AccountingError(
      "desktop_required",
      "Open Sarv in the desktop application to access your books. Browser QA requires the explicit Rust test adapter.",
    );
  } catch (e) {
    const err = e as { code?: string; message?: string };
    const message = e instanceof Error ? e.message : (err.message ?? String(e));
    if (message.includes("Sign in required")) {
      writeSession(null);
      window.dispatchEvent(new Event("sarv-locked"));
    }
    if (e instanceof Error) throw e;
    throw new AccountingError(err.code ?? "command_failed", message);
  }
}
export async function downloadWorkbook(action: string, payload: unknown) {
  const result = await command<{
    filename: string;
    content_base64: string;
    content_type: string;
  }>(action, payload);
  const binary = atob(result.content_base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  const blob = new Blob([bytes], { type: result.content_type });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = result.filename;
  link.click();
  URL.revokeObjectURL(url);
}
export async function exportReport(
  action: string,
  payload: unknown,
): Promise<boolean> {
  if (isTauri())
    return invoke<boolean>("save_report", { action, payload: withSession(payload) });
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

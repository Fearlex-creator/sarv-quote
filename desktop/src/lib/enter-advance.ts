// Enter moves to the next field like a form on paper; on the last field it
// submits. Ctrl/⌘+Enter submits from anywhere (including notes).
const FIELD =
  "input:not([type=hidden]):not([type=checkbox]):not([type=radio]):not([type=button]):not([disabled]):not([readonly]), select:not([disabled]), textarea:not([disabled])";
const SCOPE = "form, [role=dialog], [data-enter-scope]";
function visible(el: HTMLElement) {
  return el.offsetParent !== null || el.getClientRects().length > 0;
}
export function fieldsIn(scope: Element) {
  return [...scope.querySelectorAll<HTMLElement>(FIELD)].filter(
    (el) => visible(el) && el.tabIndex !== -1,
  );
}
export function focusNext(from: HTMLElement, step = 1) {
  const scope = from.closest(SCOPE);
  if (!scope) return false;
  const all = fieldsIn(scope);
  const next = all[all.indexOf(from) + step];
  if (!next) return false;
  next.focus();
  if (next instanceof HTMLInputElement) next.select();
  return true;
}
function submit(scope: Element) {
  const form = scope instanceof HTMLFormElement ? scope : scope.querySelector("form");
  const primary = scope.querySelector<HTMLElement>("[data-enter-submit]");
  if (primary && !(primary as HTMLButtonElement).disabled) primary.click();
  else form?.requestSubmit();
}
export function installEnterAdvance() {
  const handler = (e: KeyboardEvent) => {
    if (e.key !== "Enter" || e.defaultPrevented || e.isComposing) return;
    const target = e.target;
    if (!(target instanceof HTMLElement)) return;
    const scope = target.closest(SCOPE);
    if (!scope) return;
    if ((e.ctrlKey || e.metaKey) && !e.altKey) {
      if (scope.querySelector("[data-enter-submit]") || scope.querySelector("form")) {
        e.preventDefault();
        submit(scope);
      }
      return;
    }
    if (e.shiftKey || e.altKey) return;
    if (
      !(target instanceof HTMLInputElement || target instanceof HTMLSelectElement)
    )
      return;
    if (
      target instanceof HTMLInputElement &&
      !["text", "number", "tel", "email", "search", "url", "password", "date", ""].includes(target.type)
    )
      return;
    if (
      target.closest("[cmdk-root], [role=listbox]") ||
      target.getAttribute("aria-expanded") === "true" ||
      target.dataset.enter === "native"
    )
      return;
    e.preventDefault();
    if (!focusNext(target)) submit(scope);
  };
  document.addEventListener("keydown", handler);
  return () => document.removeEventListener("keydown", handler);
}

export type ResourceKind =
  "customer" | "vendor" | "item" | "warehouse" | "unit";
export type QuickAction =
  "invoice" | "order" | "purchase" | "journal" | "transfer" | ResourceKind;

// One source for the keyboard handler, Settings and the command menu.
export const creationShortcuts: {
  action: QuickAction;
  code: string;
  label: string;
}[] = [
  { action: "invoice", code: "KeyN", label: "New sales invoice" },
  { action: "order", code: "KeyO", label: "New order" },
  { action: "customer", code: "KeyC", label: "New customer" },
  { action: "item", code: "KeyP", label: "New product" },
  { action: "vendor", code: "KeyV", label: "New vendor" },
  { action: "purchase", code: "KeyB", label: "New purchase order" },
  { action: "warehouse", code: "KeyW", label: "New warehouse" },
  { action: "unit", code: "KeyU", label: "New unit" },
  { action: "journal", code: "KeyJ", label: "New journal" },
  { action: "transfer", code: "KeyT", label: "Transfer stock" },
];
export const utilityShortcuts = [
  { label: "Command menu", keys: "Ctrl / ⌘ K" },
  { label: "Settings", keys: "Ctrl / ⌘ Alt S" },
  { label: "Toggle sidebar", keys: "F9" },
  { label: "Help & shortcuts", keys: "F1" },
  { label: "Close dialog", keys: "Esc" },
];
export function shortcutKeys(code: string) {
  return `Ctrl / ⌘ Alt ${code.replace("Key", "")}`;
}
export function isTyping(target: EventTarget | null) {
  return (
    target instanceof HTMLElement &&
    !!target.closest(
      "input, textarea, select, [contenteditable]:not([contenteditable='false']), [role='textbox']",
    )
  );
}

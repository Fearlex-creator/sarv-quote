import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { PlusIcon } from "@phosphor-icons/react/dist/csr/Plus";
import { cn } from "../../lib/utils";
import { focusNext } from "../../lib/enter-advance";

export interface ComboOption {
  id: string;
  label: string;
  code?: string;
  meta?: string;
  hint?: string;
  tone?: "positive" | "warning" | "neutral";
  search?: string;
}
// Type to search; arrows and Enter pick; typing something new offers to create
// it. The whole thing is keyboard-first: Enter picks and moves to the next field.
export function Combobox({
  value,
  options,
  onChange,
  onCreate,
  createLabel,
  placeholder,
  label,
  autoFocus,
  className,
  emptyText,
}: {
  value: string;
  options: ComboOption[];
  onChange: (id: string) => void;
  onCreate?: (text: string) => void;
  createLabel?: (text: string) => string;
  placeholder?: string;
  label: string;
  autoFocus?: boolean;
  className?: string;
  emptyText?: string;
}) {
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  const selected = options.find((o) => o.id === value);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [navigated, setNavigated] = useState(false);
  const [box, setBox] = useState<{ top: number; left: number; width: number } | null>(null);
  // The list floats above scroll containers (like the sale lines) instead of being clipped.
  useLayoutEffect(() => {
    if (!open || !input.current) return;
    const place = () => {
      const r = input.current!.getBoundingClientRect();
      setBox({ top: r.bottom + 4, left: r.left, width: r.width });
    };
    place();
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    return () => {
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
    };
  }, [open]);
  const typing = open && query !== "";
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = q
      ? options.filter((o) =>
          `${o.label} ${o.search ?? ""} ${o.hint ?? ""}`.toLowerCase().includes(q),
        )
      : options;
    return list.slice(0, 50);
  }, [options, query]);
  const canCreate =
    !!onCreate &&
    query.trim() !== "" &&
    !options.some((o) => o.label.toLowerCase() === query.trim().toLowerCase());
  const rows = shown.length + (canCreate ? 1 : 0);
  useEffect(() => {
    setActive(0);
    setNavigated(false);
  }, [query, open]);
  function pick(index: number) {
    if (index < shown.length) {
      onChange(shown[index].id);
    } else if (canCreate) {
      onCreate?.(query.trim());
    } else return;
    setOpen(false);
    setQuery("");
    const el = input.current;
    // Let the parent update first, then move on to the next field.
    if (el) setTimeout(() => focusNext(el), 0);
  }
  return (
    <div className={cn("combobox", className)}>
      <input
        ref={input}
        autoFocus={autoFocus}
        role="combobox"
        aria-label={label}
        aria-expanded={open}
        aria-controls={`${id}-list`}
        aria-autocomplete="list"
        autoComplete="off"
        data-slot="input"
        className="combobox-input"
        placeholder={placeholder}
        value={typing ? query : (selected?.label ?? query)}
        onFocus={(e) => e.currentTarget.select()}
        onClick={() => setOpen(true)}
        onBlur={() => {
          setOpen(false);
          setQuery("");
        }}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
          if (value && e.target.value === "") onChange("");
        }}
        onKeyDown={(e) => {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setOpen(true);
            setNavigated(true);
            setActive((a) => Math.min(a + 1, Math.max(rows - 1, 0)));
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setNavigated(true);
            setActive((a) => Math.max(a - 1, 0));
          } else if (
            e.key === "Enter" &&
            open &&
            rows > 0 &&
            (query !== "" || navigated)
          ) {
            e.preventDefault();
            e.stopPropagation();
            pick(active);
          } else if (e.key === "Enter" && open) {
            e.preventDefault();
            e.stopPropagation();
            setOpen(false);
            focusNext(e.currentTarget);
          } else if (e.key === "Tab" && open && query !== "" && shown.length) {
            pick(active);
          } else if (e.key === "Escape" && open) {
            e.stopPropagation();
            setOpen(false);
            setQuery("");
          }
        }}
      />
      {open && box && createPortal(
        <ul
          id={`${id}-list`}
          role="listbox"
          className="combobox-list"
          style={{
            top: box.top,
            left: Math.max(12, Math.min(box.left, window.innerWidth - Math.max(box.width, 380) - 16)),
            minWidth: Math.max(box.width, 380),
          }}
        >
          {shown.map((o, i) => (
            <li
              key={o.id}
              role="option"
              aria-selected={i === active}
              data-active={i === active}
              onMouseDown={(e) => {
                e.preventDefault();
                pick(i);
              }}
              onMouseEnter={() => setActive(i)}
            >
              <div className="combobox-option-main">
                {o.code && <span className="combobox-code">{o.code}</span>}
                <span className="combobox-label">{o.label}</span>
              </div>
              <div className="combobox-option-trail">
                {o.meta && <span className="combobox-meta">{o.meta}</span>}
                {o.hint && (
                  <small className="combobox-hint" data-tone={o.tone ?? "neutral"}>
                    {o.hint}
                  </small>
                )}
              </div>
            </li>
          ))}
          {canCreate && (
            <li
              role="option"
              aria-selected={active === shown.length}
              data-active={active === shown.length}
              className="combobox-create"
              onMouseDown={(e) => {
                e.preventDefault();
                pick(shown.length);
              }}
              onMouseEnter={() => setActive(shown.length)}
            >
              <PlusIcon />
              <span>{createLabel ? createLabel(query.trim()) : `+ ${query.trim()}`}</span>
              <kbd className="combobox-kbd">↵</kbd>
            </li>
          )}
          {rows === 0 && <li className="combobox-empty">{emptyText ?? "—"}</li>}
        </ul>,
        document.body,
      )}
    </div>
  );
}

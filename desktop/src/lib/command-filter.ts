import { defaultFilter } from "cmdk";
import { normalizeSearch } from "./normalize-fa";

/**
 * cmdk filter: keeps cmdk's fuzzy ranking but compares normalised text, so Arabic ي/ك,
 * ZWNJ and any digit system find the same results.
 */
export function commandFilter(
  value: string,
  search: string,
  keywords?: string[],
): number {
  const needle = normalizeSearch(search);
  if (!needle) return 1;
  const hay = [value, ...(keywords ?? [])].map(normalizeSearch).join(" ");
  return defaultFilter(hay, needle);
}

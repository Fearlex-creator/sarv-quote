import { test, expect } from "@playwright/test";
import {
  fiscalYear,
  parseDate,
  isoDate,
  canonicalDraftDate,
} from "../src/lib/calendar";

test("Jalali fiscal years follow Nowruz and include leap Esfand", () => {
  expect(fiscalYear("fa", parseDate("2025-03-20"))).toEqual({
    name: "۱۴۰۳",
    start: "2024-03-20",
    end: "2025-03-20",
  });
  expect(fiscalYear("fa", parseDate("2025-03-21"))).toEqual({
    name: "۱۴۰۴",
    start: "2025-03-21",
    end: "2026-03-20",
  });
  expect(fiscalYear("en", parseDate("2024-02-29"))).toEqual({
    name: "2024",
    start: "2024-01-01",
    end: "2024-12-31",
  });
  expect(isoDate(parseDate("2024-02-29"))).toBe("2024-02-29");
});

test("restored Jalali draft dates remain the same accounting day", () => {
  expect(canonicalDraftDate("۱۴۰۵/۰۱/۰۱", "Jalali")).toBe("2026-03-21");
  expect(canonicalDraftDate("1403-12-30", "Jalali")).toBe("2025-03-20");
  expect(canonicalDraftDate("", "Jalali")).toBe("");
  expect(canonicalDraftDate("2024-02-29", "Iso")).toBe("2024-02-29");
});

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  matchesSearch,
  normalizeDigits,
  normalizeSearch,
} from "./normalize-fa.ts";

test("digits: Persian, Arabic-Indic and grouping noise", () => {
  assert.equal(normalizeDigits("۱۲٬۵۰۰"), "12500");
  assert.equal(normalizeDigits("١٢٣-٤٥٦"), "123456");
  assert.equal(normalizeDigits("12 500"), "12500");
});

test("Arabic letters, ZWNJ and diacritics fold to the Persian form", () => {
  assert.equal(normalizeSearch("علي"), normalizeSearch("علی"));
  assert.equal(normalizeSearch("كتاب"), normalizeSearch("کتاب"));
  assert.equal(normalizeSearch("می‌خواهم"), normalizeSearch("میخواهم"));
  assert.equal(normalizeSearch("مُحَمَّد"), normalizeSearch("محمد"));
  assert.equal(normalizeSearch("  فاکتور   ۱۲۳ "), "فاکتور 123");
});

test("matchesSearch requires every word and ignores digit system", () => {
  assert.ok(matchesSearch("فاکتور فروش ۱۴۰۵-۰۲۳۱", "فاکتور 0231"));
  assert.ok(matchesSearch("شرکت علی‌زاده", "علي"), "Arabic ي finds Persian ی");
  assert.ok(matchesSearch("شرکت علی‌زاده", "عليزاده"), "ZWNJ-insensitive");
  assert.ok(matchesSearch("Ali Rezaei", "rezaei ali"));
  assert.ok(!matchesSearch("فاکتور فروش", "خرید"));
});

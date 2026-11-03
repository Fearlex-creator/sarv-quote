import assert from "node:assert/strict";
import { test } from "node:test";
import { amountInWords, numberToPersianWords as w } from "./words-fa.ts";

test("small numbers and teens", () => {
  assert.equal(w("0"), "صفر");
  assert.equal(w("1"), "یک");
  assert.equal(w("11"), "یازده");
  assert.equal(w("19"), "نوزده");
  assert.equal(w("20"), "بیست");
  assert.equal(w("21"), "بیست و یک");
  assert.equal(w("99"), "نود و نه");
});

test("hundreds", () => {
  assert.equal(w("100"), "صد");
  assert.equal(w("101"), "صد و یک");
  assert.equal(w("250"), "دویست و پنجاه");
  assert.equal(w("999"), "نهصد و نود و نه");
});

test("scales", () => {
  assert.equal(w("1000"), "یک هزار");
  assert.equal(w("1001"), "یک هزار و یک");
  assert.equal(w("12500000"), "دوازده میلیون و پانصد هزار");
  assert.equal(w("1000000"), "یک میلیون");
  assert.equal(
    w("1234567890"),
    "یک میلیارد و دویست و سی و چهار میلیون و پانصد و شصت و هفت هزار و هشتصد و نود",
  );
  assert.equal(w("1000000000000"), "یک تریلیون");
});

test("accepts Persian digits and separators, bigint and negatives", () => {
  assert.equal(w("۱۲٬۵۰۰"), "دوازده هزار و پانصد");
  assert.equal(w(25n), "بیست و پنج");
  assert.equal(w("-5"), "منفی پنج");
});

test("upper bound and invalid input", () => {
  assert.equal(
    w("999999999999999999"),
    "نهصد و نود و نه کوادریلیون و نهصد و نود و نه تریلیون و نهصد و نود و نه میلیارد و نهصد و نود و نه میلیون و نهصد و نود و نه هزار و نهصد و نود و نه",
  );
  assert.throws(() => w("1000000000000000000"), RangeError);
  assert.throws(() => w("12.5"), RangeError);
  assert.throws(() => w("abc"), RangeError);
});

test("amountInWords: rial exact, toman only when it divides evenly", () => {
  assert.equal(amountInWords("12500000"), "دوازده میلیون و پانصد هزار ریال");
  assert.equal(
    amountInWords("12500000", "toman"),
    "یک میلیون و دویست و پنجاه هزار تومان",
  );
  assert.equal(
    amountInWords("12500005", "toman"),
    "دوازده میلیون و پانصد هزار و پنج ریال",
  );
  assert.equal(amountInWords(""), undefined);
  assert.equal(amountInWords("1.5"), undefined);
});

test("a sign is never lost and stray hyphens are rejected", () => {
  assert.equal(amountInWords("-5"), "منفی پنج ریال");
  assert.equal(amountInWords("\u22125"), "منفی پنج ریال");
  assert.equal(amountInWords("12-5"), undefined);
  assert.equal(amountInWords("5-"), undefined);
  assert.throws(() => w("1-2"), RangeError);
});

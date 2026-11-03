// Persian text and digit normalisation for search and input.
// Arabic keyboards produce ي/ك; Persian text uses ی/ک. Users mix both freely.

/** Persian/Arabic-Indic digits to Latin; drops spaces, hyphens, ZWNJ and grouping separators. */
export function normalizeDigits(input: string): string {
  let out = "";
  for (const ch of input) {
    const c = ch.codePointAt(0)!;
    if (c >= 0x06f0 && c <= 0x06f9) out += String(c - 0x06f0);
    else if (c >= 0x0660 && c <= 0x0669) out += String(c - 0x0660);
    else if (" -‌‎‏,٬،".includes(ch)) continue;
    else out += ch;
  }
  return out;
}

const LETTERS: Record<string, string> = {
  ي: "ی", // ي -> ی
  ى: "ی", // ى -> ی
  ك: "ک", // ك -> ک
  ة: "ه", // ة -> ه
};

/**
 * Canonical form for matching: unified letters, Latin digits, no diacritics, tatweel or
 * ZWNJ (so «می‌خواهم» matches «میخواهم»), single spaces, lower case.
 */
export function normalizeSearch(input: string): string {
  let out = "";
  for (const ch of input) {
    const c = ch.codePointAt(0)!;
    if (c >= 0x064b && c <= 0x065f) continue; // diacritics
    if (c === 0x0670 || c === 0x0640 || c === 0x200c || c === 0x200d) continue;
    if (c >= 0x06f0 && c <= 0x06f9) out += String(c - 0x06f0);
    else if (c >= 0x0660 && c <= 0x0669) out += String(c - 0x0660);
    else out += LETTERS[ch] ?? ch;
  }
  return out.replace(/\s+/g, " ").trim().toLowerCase();
}

/** True when every word of `query` appears in `text` after normalisation. */
export function matchesSearch(text: string, query: string): boolean {
  const hay = normalizeSearch(text);
  return normalizeSearch(query)
    .split(" ")
    .filter(Boolean)
    .every((word) => hay.includes(word));
}

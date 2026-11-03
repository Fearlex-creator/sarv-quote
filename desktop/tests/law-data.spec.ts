import { test, expect } from "@playwright/test";

test("legal values are listed with a visible 'not yet reviewed' note in English and Persian", async ({
  page,
}) => {
  for (const [locale, tab, notice, badge] of [
    ["en", "Law data", /Not yet reviewed\. Do not use these values/, "Not reviewed"],
    ["fa", "داده‌های قانونی", /هنوز بازبینی نشده است/, "بازبینی‌نشده"],
  ] as const) {
    await page.addInitScript((l) => localStorage.setItem("sarv.language", l), locale);
    await page.goto("/");
    await page
      .locator('[data-slot="sidebar"]')
      .getByRole("button", { name: locale === "en" ? "Settings" : "تنظیمات", exact: true })
      .click();
    await page.getByRole("tab", { name: tab, exact: true }).click();
    await expect(page.getByTestId("law-not-reviewed")).toContainText(notice);
    // Every legal value carries the badge; the flagged ones also say they need verification.
    await expect(page.getByText(badge, { exact: true }).first()).toBeVisible();
    expect(await page.getByText(badge, { exact: true }).count()).toBeGreaterThanOrEqual(3);
    await expect(page.getByText(locale === "en" ? "10%" : "10%").first()).toBeVisible();
  }
});

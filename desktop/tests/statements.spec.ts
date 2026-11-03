import { test, expect } from "@playwright/test";

test("balance sheet balances and aging lists open invoices and bills", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("sarv.language", "en"));
  await page.goto("/");
  await page.locator('[data-slot="sidebar"]').getByRole("button", { name: "Reports", exact: true }).click();

  await page.getByRole("tab", { name: "Balance sheet", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Balance sheet", exact: true })).toBeVisible();
  await expect(page.getByText("Balanced", { exact: true })).toBeVisible();
  for (const section of ["Assets", "Liabilities", "Equity"]) {
    await expect(page.getByRole("heading", { name: section, exact: true })).toBeVisible();
  }

  await page.getByRole("tab", { name: "Aging", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Receivables by days overdue" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Payables by days overdue" })).toBeVisible();
  // The synthetic fixture has open invoices and bills, so both tables end with a Total row.
  await expect(page.getByRole("cell", { name: "Total", exact: true })).toHaveCount(2);
  for (const bucket of ["Not due", "1–30 days", "31–60 days", "61–90 days", "Over 90 days"]) {
    await expect(page.getByRole("columnheader", { name: bucket, exact: true })).toHaveCount(2);
  }
});

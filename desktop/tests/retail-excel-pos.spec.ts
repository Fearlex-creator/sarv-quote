import { test, expect } from "@playwright/test";
import { mkdirSync } from "node:fs";
import type { Workspace } from "../src/lib/types";

const shots = "/opt/cursor/artifacts/sarv-retail";
mkdirSync(shots, { recursive: true });

test("excel import, manual till sale, and honest graph status", async ({
  page,
  request,
}) => {
  test.setTimeout(120_000);
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  async function api<T>(action: string, payload: unknown): Promise<T> {
    const response = await request.post("http://127.0.0.1:1421", {
      headers: {
        "x-sarv-qa-token":
          process.env.SARV_QA_TOKEN ?? "sarv-isolated-browser-qa-token-2026",
      },
      data: { action, payload },
    });
    const body = await response.json();
    expect(body.error, JSON.stringify(body)).toBeUndefined();
    return body.result as T;
  }
  const prefix = `retail-${Date.now()}`;
  const created = await api<{ company_id: string; period_id: string }>(
    "setup_company",
    {
      name: prefix,
      fiscal_name: "2026",
      start_date: "2026-01-01",
      end_date: "2026-12-31",
    },
  );
  await api("initialize_zero", {
    company_id: created.company_id,
    source_id: prefix,
    request_key: prefix,
    confirmed: true,
  });
  await page.addInitScript((company) => {
    localStorage.setItem("sarv.company", company);
    if (!localStorage.getItem("sarv.language")) {
      localStorage.setItem("sarv.language", "en");
    }
    localStorage.setItem("sarv.sidebar", "expanded");
  }, created.company_id);
  await page.goto("/");
  await page.getByRole("heading", { name: "Overview", exact: true }).waitFor();
  const nav = (name: string) =>
    page.locator(".sidebar").getByRole("button", { name, exact: true }).click();

  await nav("Excel");
  await expect(page.getByRole("heading", { name: "Excel", exact: true })).toBeVisible();
  await expect(page.getByText("not_configured", { exact: false })).toBeVisible();
  await expect(page.getByText("Rial amounts, not tomans", { exact: false })).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: `${shots}/01-excel-desktop.png`,
  });

  await page.locator("input[type=file]").setInputFiles("tests/fixtures/products-sample.xlsx");
  await page.getByRole("button", { name: "Preview workbook", exact: true }).click();
  await expect(page.getByText("1 rows can be committed", { exact: false })).toBeVisible();
  await page.getByRole("checkbox", { name: "Commit these rows into the books" }).check();
  await page.getByRole("button", { name: "Commit workbook", exact: true }).click();
  await expect(page.getByText("Imported 1 new", { exact: false })).toBeVisible();

  await page
    .getByRole("combobox", { name: "Sheet", exact: true })
    .selectOption("opening_stock", { force: true });
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/opening-sample.xlsx");
  await page.getByRole("button", { name: "Preview workbook", exact: true }).click();
  await expect(page.getByText("1 rows can be committed", { exact: false })).toBeVisible();
  await page.screenshot({
    animations: "disabled",
    path: `${shots}/02-excel-opening-preview.png`,
  });
  await page.getByRole("checkbox", { name: "Commit these rows into the books" }).check();
  await page.getByRole("button", { name: "Commit workbook", exact: true }).click();
  await expect(page.getByText("Imported 1 new", { exact: false })).toBeVisible();

  await nav("Point of sale");
  await expect(page.getByRole("heading", { name: "Point of sale", exact: true })).toBeVisible();
  await expect(page.getByText("not a live collection", { exact: false })).toBeVisible();
  await page.getByLabel("Cashier", { exact: true }).fill("Niloofar");
  await page.getByLabel("Opening cash", { exact: true }).fill("0");
  await page.getByRole("button", { name: "Open till", exact: true }).click();
  await expect(page.getByText("Till open", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /Tea/ }).click();
  await expect(page.getByLabel("Quantity", { exact: true })).toHaveValue("1");
  await page.screenshot({
    animations: "disabled",
    path: `${shots}/03-pos-desktop.png`,
  });
  await page.getByRole("button", { name: "Complete sale", exact: true }).click();
  await expect(page.getByText("Sale recorded", { exact: false })).toBeVisible();
  await expect(page.locator(".pos-receipt")).toContainText("Manual payment");
  await page.screenshot({
    animations: "disabled",
    path: `${shots}/04-pos-receipt.png`,
  });

  const books = await api<Workspace>("workspace", { company_id: created.company_id });
  const report = await api<{
    trial: { total_debit: string; total_credit: string };
    reconciliation: { stock: { quantity_thousandths: string }[] };
  }>("report", {
    company_id: created.company_id,
    from: "2026-01-01",
    to: "2026-12-31",
  });
  expect(report.trial.total_debit).toBe(report.trial.total_credit);
  expect(report.reconciliation.stock[0].quantity_thousandths).toBe("4000");
  expect(books.retail?.open_shift?.cashier).toBe("Niloofar");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.evaluate(() => localStorage.setItem("sarv.language", "fa"));
  await page.reload();
  await page.getByRole("button", { name: "صفحه‌های بیشتر", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "صندوق فروش", exact: true })
    .click();
  await expect(page.getByRole("heading", { name: "صندوق فروش", exact: true })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  await page.screenshot({
    animations: "disabled",
    path: `${shots}/05-pos-mobile-rtl.png`,
  });
  expect(errors).toEqual([]);
});

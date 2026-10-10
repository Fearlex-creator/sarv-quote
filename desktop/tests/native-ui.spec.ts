import { test, expect } from "@playwright/test";
import { mkdirSync } from "node:fs";
import type { Workspace, Journal, Report } from "../src/lib/types";

const out = "../artifacts/tauri-native-redesign/after";
mkdirSync(out, { recursive: true });

test("native layout preserves exact amounts and review does not post", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
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
  const prefix = `native-ui-${Date.now()}`;
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
  const initial = await api<Workspace>("workspace", {
    company_id: created.company_id,
  });
  await api("settings", {
    ...initial.settings,
    appearance: "Light",
    display_digits: "Latin",
    default_currency: "Rial",
  });
  try {
    const amount = "9007199254740993";
    const payload = {
      ...created,
      date: "2026-10-10",
      reason: "Exact amount",
      source_id: `${prefix}-journal`,
      request_key: `${prefix}-journal`,
      lines: [
        {
          account_id: initial.accounts.find((a) => a.code === "5201")!.id,
          debit: amount,
          credit: "0",
          description: "Expense",
        },
        {
          account_id: initial.accounts.find((a) => a.code === "3101")!.id,
          debit: "0",
          credit: amount,
          description: "Equity",
        },
      ],
    };
    const preview = await api<Journal>("journal_preview", payload);
    await api("journal_post", {
      ...payload,
      expected_generation: preview._generation,
    });
    await page.addInitScript((company) => {
      localStorage.setItem("sarv.company", company);
      localStorage.setItem("sarv.language", "en");
      localStorage.setItem("sarv.sidebar", "expanded");
    }, created.company_id);
    await page.goto("/");
    await page
      .getByRole("heading", { name: "Overview", exact: true })
      .waitFor();
    const nav = (name: string) =>
      page
        .locator(".sidebar")
        .getByRole("button", { name, exact: true })
        .click();
    await nav("Accounting");
    await expect(page.locator(".topbar h1")).toHaveText("Accounting");
    await expect(page.locator(".topbar .button-primary")).toHaveCount(1);
    await expect(
      page.getByRole("columnheader", { name: "Debit · IRR", exact: true }),
    ).toBeVisible();
    await expect(page.locator(".page tbody .money")).toHaveText([
      "9,007,199,254,740,993",
      "9,007,199,254,740,993",
    ]);
    await expect(page.locator(".page .currency")).toHaveCount(0);
    await expect(page.locator(".sync-time time")).toHaveAttribute(
      "datetime",
      /T/,
    );
    await expect(
      page.getByRole("cell", { name: "Oct 10, 2026", exact: true }),
    ).toBeVisible();
    await page.screenshot({
      animations: "disabled",
      path: `${out}/03-journals-exact-amounts.png`,
    });
    await page
      .getByRole("button", { name: "New journal", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Reason", { exact: true }).fill("Review only");
    await dialog
      .getByLabel("Journal account 1", { exact: true })
      .selectOption(initial.accounts.find((a) => a.code === "5201")!.id);
    await dialog.getByLabel("Journal debit 1", { exact: true }).fill("7");
    await dialog
      .getByLabel("Journal account 2", { exact: true })
      .selectOption(initial.accounts.find((a) => a.code === "3101")!.id);
    await dialog.getByLabel("Journal credit 2", { exact: true }).fill("7");
    await dialog
      .getByRole("button", { name: "Validate journal", exact: true })
      .click();
    await expect(
      dialog.getByRole("button", {
        name: "Confirm and post journal",
        exact: true,
      }),
    ).toBeEnabled();
    await expect(dialog.locator(".confirmation-text")).toContainText(
      "Review only",
    );
    await expect(dialog.locator(".confirmation-text")).toContainText(
      "immutable journal",
    );
    await page.screenshot({
      animations: "disabled",
      path: `${out}/04-posting-review.png`,
    });
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    const after = await api<Workspace>("workspace", {
      company_id: created.company_id,
    });
    expect(after.journal_count).toBe("1");
    expect(after.journals[0].total_debit).toBe(amount);
    await nav("Reports");
    await page.getByRole("tab", { name: "Trial balance", exact: true }).click();
    await expect(page.locator("tfoot .money")).toHaveText([
      "9,007,199,254,740,993",
      "9,007,199,254,740,993",
    ]);
    await page.screenshot({
      animations: "disabled",
      path: `${out}/05-trial-totals.png`,
    });
    await page.locator(".page tbody .text-link").first().click();
    await expect(page.getByRole("dialog").getByRole("table")).toBeVisible();
    await page.screenshot({
      animations: "disabled",
      path: `${out}/06-ledger.png`,
    });
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Close dialog" })
      .click();
    await nav("Settings");
    await page.getByLabel("Amount display").selectOption("Toman");
    await page
      .getByRole("button", { name: "Save preferences", exact: true })
      .click();
    await expect(
      page.getByText("Preferences saved", { exact: true }),
    ).toBeVisible();
    await nav("Accounting");
    await expect(
      page.getByRole("columnheader", { name: "Debit · toman", exact: true }),
    ).toBeVisible();
    await expect(page.locator(".page tbody .money")).toHaveText([
      "900,719,925,474,099.3",
      "900,719,925,474,099.3",
    ]);
    await nav("Settings");
    await page.screenshot({
      animations: "disabled",
      path: `${out}/07-settings.png`,
    });
    await page.getByRole("tab", { name: "Agents", exact: true }).click();
    await expect(page.getByRole("switch")).toBeVisible();
    await page.screenshot({
      animations: "disabled",
      path: `${out}/08-agents.png`,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.locator(".sidebar")).toBeHidden();
    await expect(
      page.getByRole("navigation", { name: "Mobile navigation" }),
    ).toBeVisible();
    await page.screenshot({
      animations: "disabled",
      path: `${out}/09-agents-mobile.png`,
    });
    await page.getByRole("button", { name: "More pages", exact: true }).click();
    await page
      .getByRole("navigation", { name: "All pages" })
      .getByRole("button", { name: "Accounting", exact: true })
      .click();
    await expect(page.locator(".topbar h1")).toHaveText("Accounting");
    await page.screenshot({
      animations: "disabled",
      path: `${out}/10-journals-mobile.png`,
    });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
    ).toBe(false);
    const report = await api<Report>("report", {
      company_id: created.company_id,
      from: "2026-01-01",
      to: "2026-10-10",
    });
    expect(report.trial.total_debit).toBe(amount);
    expect(report.trial.total_credit).toBe(amount);
    expect(errors).toEqual([]);
  } finally {
    await api("settings", initial.settings);
  }
});

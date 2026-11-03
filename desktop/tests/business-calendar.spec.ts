import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

mkdirSync("/tmp/sarv-business-calendar", { recursive: true });
test("business creation belongs to Settings and every transaction uses the configured calendar", async ({
  page,
  request,
}) => {
  page.setDefaultTimeout(10_000);
  const profile = mkdtempSync(join(tmpdir(), "sarv-calendar-"));
  const token = "sarv-onboarding-isolated-qa-2026";
  const backend = "http://127.0.0.1:1422";
  const server = spawn(resolve("../target/debug/examples/desktop_qa"), [], {
    cwd: resolve(".."),
    env: {
      ...process.env,
      SARV_QA_DATA_DIR: profile,
      SARV_QA_TOKEN: token,
      SARV_QA_SEED: "1",
      SARV_QA_PORT: "1422",
    },
    stdio: "ignore",
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  try {
    await expect
      .poll(() =>
        request
          .get(backend)
          .then((r) => r.status())
          .catch(() => 0),
      )
      .toBe(403);
    await page.route("http://127.0.0.1:1421/**", async (route) => {
      const response = await route.fetch({
        url: backend,
        headers: { ...route.request().headers(), "x-sarv-qa-token": token },
      });
      await route.fulfill({ response });
    });
    await page.addInitScript(() => localStorage.setItem("sarv.language", "en"));
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: "Overview", exact: true }),
    ).toBeVisible();
    const nav = async (name: string) =>
      page
        .locator('[data-slot="sidebar"]')
        .getByRole("button", { name, exact: true })
        .click();
    await expect(
      page
        .locator('[data-slot="sidebar"]')
        .getByRole("button", { name: "Add company", exact: true }),
    ).toHaveCount(0);
    await nav("Settings");
    await page
      .getByLabel("Date display", { exact: true })
      .selectOption("Jalali");
    await page
      .getByLabel("Business phone", { exact: true })
      .fill("021-11223344");
    await page
      .getByLabel("Registration / national ID", { exact: true })
      .fill("REG-456");
    await page
      .getByRole("button", { name: "Save preferences", exact: true })
      .click();
    await expect(
      page.getByText("Preferences saved", { exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Add company", exact: true })
      .click();
    await expect(
      page.getByRole("dialog").getByLabel("Start date", { exact: true }),
    ).toContainText(/Farvardin|فروردین/);
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    // The sidebar starts collapsed; only collapse it if a previous step expanded it.
    const collapse = page.getByRole("button", {
      name: "Collapse sidebar",
      exact: true,
    });
    if (await collapse.isVisible()) await collapse.click();
    await expect.poll(async () => (await page.locator('[data-slot="sidebar-container"]').boundingBox())!.width).toBe(72);
    const settingsIcon = page
      .locator('[data-slot="sidebar"]')
      .getByRole("button", { name: "Settings", exact: true });
    expect(
      await settingsIcon
        .locator("svg")
        .evaluate((el) => getComputedStyle(el).width),
    ).toBe("24px");
    const buttonBox = await settingsIcon.boundingBox();
    expect(buttonBox!.width).toBeGreaterThanOrEqual(48);
    await page.screenshot({
      path: "/tmp/sarv-business-calendar/collapsed-settings.png",
    });
    const check = async (label: string) => {
      const input = page.getByLabel(label, { exact: true });
      await input.click();
      const calendar = page.locator('[data-slot="calendar"]');
      await expect(calendar).toBeVisible();
      await expect(calendar).toContainText(
        /Farvardin|Ordibehesht|Khordad|Tir|Mordad|Shahrivar|Mehr|Aban|Azar|Dey|Bahman|Esfand/,
      );
      const years = await calendar.locator("select.rdp-years_dropdown option").evaluateAll((options) => options.map((option) => Number((option as HTMLOptionElement).value)));
      expect(Math.max(...years)).toBeGreaterThan(Number(await calendar.locator("select.rdp-years_dropdown").inputValue()));
      await page.keyboard.press("Escape");
      await expect(calendar).toHaveCount(0);
      await expect(page.locator('input[type="date"]')).toHaveCount(0);
    };
    await nav("Overview");
    await check("Overview cutoff date");
    await page
      .getByRole("button", { name: "New journal", exact: true })
      .click();
    await check("Financial date");
    await page
      .getByRole("button", { name: "Close dialog", exact: true })
      .click();
    await nav("Reports");
    await check("From");
    await check("Through");
    await nav("Purchases");
    await page
      .getByRole("button", { name: "New purchase order", exact: true })
      .click();
    await check("Financial / document date");
    await page
      .getByRole("button", { name: "Close dialog", exact: true })
      .click();
    await page.getByRole("button", { name: "Discard editor", exact: true }).click();
    await nav("Sales");
    await page.getByRole("button", { name: "New order", exact: true }).click();
    await check("Document date");
    await check("Financial date");
    await page.screenshot({
      path: "/tmp/sarv-business-calendar/sales-jalali.png",
    });
    expect(errors).toEqual([]);
    await expect(page.locator("vite-error-overlay")).toHaveCount(0);
  } finally {
    if (server.exitCode === null && server.signalCode === null) {
      server.kill("SIGTERM");
      await once(server, "exit");
    }
    rmSync(profile, { recursive: true, force: true });
  }
});

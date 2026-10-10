import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const token = "sarv-onboarding-isolated-qa-2026";
const backend = "http://127.0.0.1:1422";
const evidence = "/tmp/sarv-onboarding-motion";
mkdirSync(evidence, { recursive: true });

for (const locale of ["en", "fa"] as const) {
  test(`first-run ${locale} setup is animated, reversible, and requires opening confirmation`, async ({
    page,
    request,
  }) => {
    const profile = mkdtempSync(join(tmpdir(), "sarv-onboarding-"));
    const server = spawn(resolve("../target/debug/examples/desktop_qa"), [], {
      cwd: resolve(".."),
      env: {
        ...process.env,
        SARV_QA_DATA_DIR: profile,
        SARV_QA_TOKEN: token,
        SARV_QA_SEED: "0",
        SARV_QA_PORT: "1422",
      },
      stdio: "ignore",
    });
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const api = async (action: string, payload: unknown = {}) => {
      const response = await request.post(backend, {
        headers: { "x-sarv-qa-token": token },
        data: { action, payload },
      });
      const body = await response.json();
      expect(body.error).toBeUndefined();
      return body.result;
    };
    const fa = locale === "fa";
    try {
      await expect
        .poll(async () =>
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
      await page.addInitScript(
        (l) => localStorage.setItem("sarv.language", l),
        locale,
      );
      await page.goto("/");
      await expect(page).toHaveTitle(/Sarv/);
      await expect(
        page.getByRole("heading", {
          name: fa ? "به سرو خوش آمدید" : "Welcome to Sarv",
        }),
      ).toBeVisible();
      await expect(page.locator("html")).toHaveAttribute(
        "dir",
        fa ? "rtl" : "ltr",
      );
      await expect(
        page.getByLabel(fa ? "زبان برنامه" : "App language", { exact: true }),
      ).toBeVisible();
      expect((await api("workspace")).companies).toEqual([]);
      expect(
        await page
          .locator(".welcome-mark")
          .evaluate((el) => getComputedStyle(el).animationName),
      ).toBe("sprout-enter");
      await page.screenshot({
        path: `${evidence}/welcome-${locale}.png`,
        animations: "disabled",
      });
      await page.setViewportSize({ width: 390, height: 844 });
      await expect(page.locator(".onboarding")).toBeVisible();
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
      ).toBeLessThanOrEqual(390);
      await page.screenshot({
        path: `${evidence}/welcome-mobile-${locale}.png`,
        animations: "disabled",
      });
      await page.emulateMedia({ reducedMotion: "reduce" });
      expect(
        await page
          .locator(".welcome-mark")
          .evaluate((el) => getComputedStyle(el).animationName),
      ).toBe("none");
      await page
        .getByRole("button", { name: fa ? "شروع" : "Get started", exact: true })
        .click();
      const dialog = page.getByRole("dialog");
      const company = dialog.getByLabel(fa ? "نام کسب‌وکار" : "Company name", {
        exact: true,
      });
      await expect(company).toBeFocused();
      await company.fill(fa ? "کسب‌وکار تازه" : "Fresh business");
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await expect(dialog.locator('[aria-current="step"]')).toContainText(
        fa ? "دورهٔ مالی" : "Fiscal period",
      );
      expect((await api("workspace")).companies).toEqual([]);
      await dialog
        .getByRole("button", { name: fa ? "بازگشت" : "Back", exact: true })
        .click();
      await expect(company).toHaveValue(
        fa ? "کسب‌وکار تازه" : "Fresh business",
      );
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await page.screenshot({
        path: `${evidence}/period-mobile-${locale}.png`,
      });
      await dialog
        .getByRole("button", {
          name: fa ? "ایجاد کسب‌وکار" : "Create company",
          exact: true,
        })
        .click();
      await expect(dialog.getByRole("checkbox")).toBeVisible();
      const companyId = (await api("workspace")).companies[0].id;
      const before = await api("workspace", { company_id: companyId });
      expect(before.initialized).toBe(false);
      expect(before.journal_count).toBe("0");
      const start = dialog.getByRole("button", {
        name: fa ? "شروع با مانده‌های صفر" : "Start with zero balances",
        exact: true,
      });
      await expect(start).toBeDisabled();
      await dialog.getByRole("checkbox").check();
      await expect(start).toBeEnabled();
      await start.click();
      await expect(dialog).not.toBeVisible();
      expect(
        (await api("workspace", { company_id: companyId })).initialized,
      ).toBe(true);
      await page.reload();
      await expect(page.locator(".onboarding")).toHaveCount(0);
      await expect(
        page.getByRole("heading", {
          name: fa ? "نمای کلی" : "Overview",
          exact: true,
        }),
      ).toBeVisible();
      expect(
        await page
          .locator(".page")
          .evaluate((el) => getComputedStyle(el).animationName),
      ).toBe("none");
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
}

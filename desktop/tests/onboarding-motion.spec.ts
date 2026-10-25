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
    page.on("console", (e) => {
      if (e.type() === "error") errors.push(e.text());
    });
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
      await page.clock.setFixedTime(new Date("2026-10-10T12:00:00Z"));
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
      await expect(page.locator(".mobile-navigation")).not.toBeVisible();
      const language = page.getByLabel(fa ? "زبان برنامه" : "App language", {
        exact: true,
      });
      await language.selectOption(fa ? "en" : "fa");
      await expect(page.locator(".welcome-calendar")).toContainText(
        fa ? "Gregorian" : "جلالی",
      );
      await page
        .getByLabel(fa ? "App language" : "زبان برنامه", { exact: true })
        .selectOption(locale);
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
        fa ? "اطلاعات کسب‌وکار" : "Business details",
      );
      await dialog
        .getByLabel(fa ? "تلفن کسب‌وکار" : "Business phone", { exact: true })
        .fill("021-12345678");
      await dialog
        .getByLabel(
          fa ? "شماره ثبت / شناسه ملی" : "Registration / national ID",
          { exact: true },
        )
        .fill("REG-123");
      await dialog
        .getByLabel(fa ? "نشانی کسب‌وکار" : "Business address", { exact: true })
        .fill(fa ? "تهران، خیابان سرو" : "12 Sarv Street");
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await expect(dialog.locator('[aria-current="step"]')).toContainText(
        fa ? "دورهٔ مالی" : "Fiscal period",
      );
      const fiscalName = dialog.getByLabel(
        fa ? "دورهٔ مالی" : "Fiscal period",
        { exact: true },
      );
      await expect(fiscalName).toHaveValue(fa ? "۱۴۰۵" : "2026");
      const startDate = dialog.getByLabel(fa ? "تاریخ شروع" : "Start date", {
        exact: true,
      });
      await expect(startDate).toContainText(fa ? "فروردین" : "January");
      await startDate.click();
      const calendar = page.locator('[data-slot="calendar"]');
      await expect(calendar).toBeVisible();
      await expect(calendar).toHaveAttribute("dir", fa ? "rtl" : "ltr");
      await expect(calendar).toContainText(fa ? "۱۴۰۵" : "2026");
      // Pick another day and verify it persists as an ISO accounting date.
      await calendar
        .locator("button[data-day]")
        .filter({ hasText: fa ? /^۲$/ : /^2$/ })
        .first()
        .click();
      await expect(calendar).not.toBeVisible();
      expect((await api("workspace")).companies).toEqual([]);
      await dialog
        .getByRole("button", { name: fa ? "بازگشت" : "Back", exact: true })
        .click();
      await expect(
        dialog.getByLabel(
          fa ? "شماره ثبت / شناسه ملی" : "Registration / national ID",
          { exact: true },
        ),
      ).toHaveValue("REG-123");
      await dialog
        .getByRole("button", { name: fa ? "بازگشت" : "Back", exact: true })
        .click();
      await expect(company).toHaveValue(
        fa ? "کسب‌وکار تازه" : "Fresh business",
      );
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await page.screenshot({
        path: `${evidence}/period-mobile-${locale}.png`,
      });
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await expect(dialog.locator(".setup-review")).toContainText(
        fa ? "کسب‌وکار تازه" : "Fresh business",
      );
      expect((await api("workspace")).companies).toEqual([]);
      await dialog
        .getByRole("button", { name: fa ? "بازگشت" : "Back", exact: true })
        .click();
      await expect(fiscalName).toHaveValue(fa ? "۱۴۰۵" : "2026");
      await dialog
        .getByRole("button", { name: fa ? "ادامه" : "Continue", exact: true })
        .click();
      await page.screenshot({
        path: `${evidence}/review-mobile-${locale}.png`,
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
      expect(before.periods[0].start_date).toBe(
        fa ? "2026-03-22" : "2026-01-02",
      );
      expect(before.periods[0].end_date).toBe(fa ? "2027-03-20" : "2026-12-31");
      expect(before.settings.date_display ?? "Iso").toBe(fa ? "Jalali" : "Iso");
      expect(before.settings.display_digits).toBe(fa ? "Persian" : "Latin");
      expect(before.business_profile.phone).toBe("021-12345678");
      expect(before.business_profile.registration_id).toBe("REG-123");
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

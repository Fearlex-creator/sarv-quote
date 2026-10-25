import { test, expect } from "@playwright/test";
import { mkdirSync } from "node:fs";

mkdirSync("/tmp/sarv-toasts", { recursive: true });
for (const locale of ["en", "fa"] as const) {
  for (const theme of ["Light", "Dark"] as const) {
    test(`notifications are readable and dismissible in ${locale} ${theme}`, async ({
      page,
    }) => {
      const errors: string[] = [];
      page.on("pageerror", (error) => errors.push(error.message));
      await page.route("http://127.0.0.1:1421/**", (route) =>
        route.fulfill({
          json: {
            result: {
              companies: [],
              periods: [],
              settings: {
                appearance: theme,
                density: "Comfortable",
                default_currency: "Rial",
                display_digits: locale === "fa" ? "Persian" : "Latin",
              },
              initialized: false,
            },
          },
        }),
      );
      await page.addInitScript(
        (value) => localStorage.setItem("sarv.language", value),
        locale,
      );
      await page.goto("/");
      await expect(page.locator(".onboarding")).toBeVisible();
      const message =
        locale === "fa"
          ? "اطلاعات کسب‌وکار ذخیره شد. برای ادامه، سال مالی و مانده‌های آغازین را بررسی کنید."
          : "Your business information has been saved. Review the financial year and opening balances to continue.";
      const show = async (type: "success" | "error" | "info" | "warning") => {
        await page.evaluate(
          async ({ type, message }) => {
            // Use the exact Vite module URL so this is the app's Sonner store.
            const source = await (
              await fetch("/src/components/ui/sonner.tsx")
            ).text();
            const moduleUrl = source.match(
              /from\s*["']([^"']*\/sonner\.js[^"']*)["']/,
            )?.[1];
            if (!moduleUrl) throw new Error("Sonner module URL missing");
            const { toast } = await import(moduleUrl);
            toast[type](message, {
              description:
                "SARV-2026 / " + "LongNotificationReference".repeat(8),
            });
          },
          { type, message },
        );
      };
      for (const type of ["success", "error", "info", "warning"] as const) {
        await show(type);
        const toast = page.locator("[data-sonner-toast]").last();
        await expect(toast).toContainText(message);
        const title = toast.locator("[data-title]");
        expect(
          await title.evaluate((element) => getComputedStyle(element).fontSize),
        ).toBe("14px");
        expect(
          await title.evaluate(
            (element) => getComputedStyle(element).textAlign,
          ),
        ).toBe("start");
        expect(
          await toast.evaluate(
            (element) => element.scrollWidth <= element.clientWidth,
          ),
        ).toBe(true);
        await toast
          .getByRole("button", {
            name: locale === "fa" ? "بستن اعلان" : "Close notification",
          })
          .click();
        await expect(toast).toHaveCount(0);
      }
      await page.setViewportSize({ width: 390, height: 844 });
      await show("success");
      const toast = page.locator("[data-sonner-toast]");
      await expect(toast).toBeVisible();
      await expect(page.locator("[data-sonner-toaster]")).toHaveAttribute(
        "dir",
        locale === "fa" ? "rtl" : "ltr",
      );
      await expect(page.locator("[data-sonner-toaster]")).toHaveAttribute(
        "data-sonner-theme",
        theme.toLowerCase(),
      );
      const box = await toast.boundingBox();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(390);
      await page.waitForTimeout(3000);
      await expect(toast).toBeVisible();
      await page.screenshot({
        path: `/tmp/sarv-toasts/${locale}-${theme}.png`,
      });
      expect(errors).toEqual([]);
      await expect(page.locator("vite-error-overlay")).toHaveCount(0);
    });
  }
}

import { test, expect } from "@playwright/test";
import type { Workspace, Journal, Report, Preview } from "../src/lib/types";
import { mkdirSync } from "node:fs";
const evidence = "/tmp/sarv-persian-rtl";
mkdirSync(evidence, { recursive: true });

test("Persian is complete across settings, local agents, grids, reviews and mobile", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (e) => {
    if (e.type() === "error") errors.push(e.text());
  });
  async function api<T = unknown>(
    action: string,
    payload: unknown,
  ): Promise<T> {
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
  const prefix = `rtl-${Date.now()}`;
  const created = await api<{ company_id: string; period_id: string }>(
    "setup_company",
    {
      name: "بازرگانی سرو · SARV-001",
      fiscal_name: "۱۴۰۵",
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
    default_currency: "Rial",
    display_digits: "Persian",
    date_display: "Jalali",
  });
  const context = (key: string) => ({
    ...created,
    date: "2026-10-10",
    source_id: `${prefix}-${key}`,
    request_key: `${prefix}-${key}`,
    reason: "Persian presentation fixture",
    reference: `PO-${key}`,
    due_date: "2026-10-30",
  });
  try {
    const amount = "9007199254740993";
    const journalPayload = {
      ...context("journal"),
      reason: "خرید تجهیزات · INV-001",
      lines: [
        {
          account_id: initial.accounts.find((a) => a.code === "5201")!.id,
          debit: amount,
          credit: "0",
          description: "هزینه",
        },
        {
          account_id: initial.accounts.find((a) => a.code === "3101")!.id,
          debit: "0",
          credit: amount,
          description: "سرمایه",
        },
      ],
    };
    const journalPreview = await api<Journal>(
      "journal_preview",
      journalPayload,
    );
    await api("journal_post", {
      ...journalPayload,
      expected_generation: journalPreview._generation,
    });
    for (const [kind, id, name] of [
      ["unit", "unit", "عدد"],
      ["warehouse", "warehouse", "انبار تهران"],
      ["vendor", "vendor", "تأمین‌کنندهٔ ایران"],
      ["item", "item", "کالای نمونه · SKU-001"],
    ]) {
      await api("resource", {
        ...context(id),
        kind,
        id: `${prefix}-${id}`,
        name,
        sku: "SKU-001",
        unit_id: `${prefix}-unit`,
        indivisible: false,
      });
    }
    // Existing Rust parsers receive Persian input directly, including the decimal separator.
    const purchase = await api<{ record_id: string }>("purchase_post", {
      ...context("order"),
      supplier_id: `${prefix}-vendor`,
      discount: "۰",
      lines: [
        {
          id: `${prefix}-line`,
          item_id: `${prefix}-item`,
          warehouse_id: `${prefix}-warehouse`,
          unit_id: `${prefix}-unit`,
          quantity: "۳٫۱۲۵",
          unit_price: "۸",
          line_discount: "۰",
        },
      ],
    });
    const options = await api<{ lines: { id: string }[] }>(
      "operation_options",
      {
        company_id: created.company_id,
        record_id: purchase.record_id,
        operation: "receive",
      },
    );
    const receipt = {
      ...context("receipt"),
      record_id: purchase.record_id,
      order_id: purchase.record_id,
      lines: options.lines.map((l) => ({ id: l.id, quantity: "۳٫۱۲۵" })),
    };
    const receiptPreview = await api<Preview>("receive_preview", receipt);
    await api("receive_post", {
      ...receipt,
      expected_generation: receiptPreview._generation,
    });
    const reportBefore = await api<Report>("report", {
      company_id: created.company_id,
      from: "2026-01-01",
      to: "2026-12-31",
    });
    const workspaceBefore = await api<Workspace>("workspace", {
      company_id: created.company_id,
    });
    await page.addInitScript((company) => {
      localStorage.setItem("sarv.company", company);
      if (!localStorage.getItem("sarv.language"))
        localStorage.setItem("sarv.language", "fa");
      localStorage.setItem("sarv.sidebar", "expanded");
    }, created.company_id);
    await page.goto("/");
    await expect(page).toHaveTitle(/Sarv/i);
    await expect(page.locator("html")).toHaveAttribute("lang", "fa");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator(".topbar h1")).toHaveText("نمای کلی");
    await expect(page.locator("vite-error-overlay")).toHaveCount(0);
    const nav = (label: string) =>
      page
        .locator(".sidebar")
        .getByRole("button", { name: label, exact: true })
        .click();
    await nav("تنظیمات");
    await expect(page.getByLabel("زبان برنامه")).toHaveValue("fa");
    await expect(page.getByLabel("تقویم نمایشی")).toHaveValue("Jalali");
    await expect(page.getByLabel("ارقام نمایشی")).toHaveValue("Persian");
    await expect(
      page.getByRole("button", { name: "ذخیرهٔ تنظیمات", exact: true }),
    ).toBeVisible();
    await expect(
      page.locator("#settings-general [data-slot=native-select]"),
    ).toHaveCount(6);
    await page.screenshot({
      path: `${evidence}/01-settings.png`,
      animations: "disabled",
    });
    // Left goes forward through the logical tab order in RTL; right goes back.
    await page.getByRole("tab", { name: "عمومی", exact: true }).focus();
    await page.keyboard.press("ArrowLeft");
    await expect(
      page.getByRole("tab", { name: "عامل‌ها", exact: true }),
    ).toBeFocused();
    await expect(
      page.getByRole("switch", { name: "دسترسی عامل‌ها", exact: true }),
    ).toBeVisible();
    await expect(page.getByLabel("سطح دسترسی")).toHaveValue("read_only");
    await expect(
      page.getByRole("button", { name: "کپی فرمان CLI" }),
    ).toBeDisabled();
    await page.screenshot({
      path: `${evidence}/02-local-agents.png`,
      animations: "disabled",
    });
    await page.keyboard.press("ArrowRight");
    await expect(
      page.getByRole("tab", { name: "عمومی", exact: true }),
    ).toBeFocused();
    await nav("حسابداری");
    const faAmount = new Intl.NumberFormat("fa-IR").format(BigInt(amount));
    await expect(
      page.locator(".page tbody .money").filter({ hasText: faAmount }),
    ).toHaveCount(2);
    await expect(
      page.getByRole("columnheader", { name: "بدهکار · IRR", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("cell", { name: "۱۸ مهر ۱۴۰۵", exact: true }).first(),
    ).toBeVisible();
    await expect(page.locator(".sync-time")).toContainText("آخرین همگام‌سازی");
    await expect(page.locator(".status[data-slot=badge]").first()).toHaveText(
      "ثبت‌شده",
    );
    await page.evaluate(() => document.fonts.ready);
    const numeric = await page
      .locator(".page tbody .money")
      .first()
      .evaluate((el) => ({
        font: getComputedStyle(el).fontFamily,
        variant: getComputedStyle(el).fontVariantNumeric,
        align: getComputedStyle(el.parentElement!).textAlign,
        dir: getComputedStyle(el).direction,
      }));
    expect(numeric).toMatchObject({
      variant: "tabular-nums",
      align: "right",
      dir: "ltr",
    });
    expect(numeric.font).toContain("Vazirmatn");
    const widths = await page
      .locator(".money")
      .first()
      .evaluate(() => {
        const probe = document.createElement("span");
        probe.className = "money tabular-nums";
        document.body.appendChild(probe);
        const result = Array.from("۰۱۲۳۴۵۶۷۸۹", (d) => {
          probe.textContent = d;
          return probe.getBoundingClientRect().width;
        });
        probe.remove();
        return result;
      });
    expect(Math.max(...widths) - Math.min(...widths)).toBeLessThan(0.2);
    await page.screenshot({
      path: `${evidence}/03-journals.png`,
      animations: "disabled",
    });
    await page.getByLabel("جست‌وجوی رکوردها", { exact: true }).fill("INV-001");
    await expect(page.locator(".page tbody tr")).toHaveCount(1);
    await page.getByLabel("جست‌وجوی رکوردها", { exact: true }).fill("ناموجود");
    await expect(
      page.getByText("رکورد مطابقی پیدا نشد.", { exact: true }),
    ).toBeVisible();
    await page.getByLabel("جست‌وجوی رکوردها", { exact: true }).clear();
    await page.getByRole("button", { name: "سند جدید", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog
      .getByLabel("دلیل", { exact: true })
      .fill("بررسی سند · TEST-001");
    await dialog
      .getByLabel(/حساب ردیف.*۱/)
      .selectOption(initial.accounts.find((a) => a.code === "5201")!.id);
    await dialog.getByLabel(/بدهکار ردیف.*۱/).fill("۷");
    await dialog
      .getByLabel(/حساب ردیف.*۲/)
      .selectOption(initial.accounts.find((a) => a.code === "3101")!.id);
    await dialog.getByLabel(/بستانکار ردیف.*۲/).fill("۷");
    await dialog
      .getByRole("button", { name: "اعتبارسنجی سند", exact: true })
      .click();
    await expect(
      dialog.getByRole("button", { name: "تأیید و ثبت سند" }),
    ).toBeEnabled();
    await expect(dialog.locator(".confirmation-text")).toContainText(
      "غیرقابل‌تغییر",
    );
    await expect(dialog.locator(".confirmation-text")).toContainText(
      "TEST-001",
    );
    await page.screenshot({
      path: `${evidence}/04-posting-review.png`,
      animations: "disabled",
    });
    await dialog.getByRole("button", { name: "انصراف", exact: true }).click();
    // Persian commands and their English aliases both resolve the same page.
    await page.keyboard.press("Control+k");
    await page.getByLabel("جست‌وجوی فرمان‌ها", { exact: true }).fill("گزارش");
    await expect(page.locator("[cmdk-item]")).toHaveCount(1);
    await page.keyboard.press("Enter");
    await expect(page.locator(".topbar h1")).toHaveText("گزارش‌ها");
    await page.getByRole("tab", { name: "تراز آزمایشی", exact: true }).click();
    await expect(page.locator("tfoot")).toBeVisible();
    await expect(page.locator("tfoot .money")).toHaveCount(2);
    await expect(page.locator("thead").first()).toHaveCSS("position", "sticky");
    await page.screenshot({
      path: `${evidence}/05-trial-balance.png`,
      animations: "disabled",
    });
    await nav("موجودی");
    await expect(
      page.getByRole("cell", { name: "۳٫۱۲۵", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("cell", { name: "کالای نمونه · SKU-001", exact: true }),
    ).toBeVisible();
    await nav("تنظیمات");
    await page.getByLabel("ارقام نمایشی").selectOption("Latin");
    await page
      .getByRole("button", { name: "ذخیرهٔ تنظیمات", exact: true })
      .click();
    await expect(
      page.getByText("تنظیمات ذخیره شد", { exact: true }),
    ).toBeVisible();
    await nav("حسابداری");
    await expect(
      page.getByRole("cell", { name: "18 مهر 1405", exact: true }).first(),
    ).toBeVisible();
    await expect(
      page
        .locator(".page tbody .money")
        .filter({ hasText: "9,007,199,254,740,993" }),
    ).toHaveCount(2);
    await page.setViewportSize({ width: 390, height: 844 });
    await page
      .getByRole("button", { name: "صفحه‌های بیشتر", exact: true })
      .click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "تنظیمات", exact: true })
      .click();
    await expect(page.locator(".topbar h1")).toHaveText("تنظیمات");
    await page.screenshot({
      path: `${evidence}/06-mobile-settings.png`,
      animations: "disabled",
    });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await page.getByLabel("زبان برنامه").selectOption("en");
    await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
    await expect(page.locator(".topbar h1")).toHaveText("Settings");
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
    expect(errors).toEqual([]);
    const workspaceAfter = await api<Workspace>("workspace", {
      company_id: created.company_id,
    });
    expect(workspaceAfter.journals).toEqual(workspaceBefore.journals);
    expect(workspaceAfter.sales).toEqual(workspaceBefore.sales);
    expect(workspaceAfter.purchases).toEqual(workspaceBefore.purchases);
    expect(workspaceAfter.resources).toEqual(workspaceBefore.resources);
    const reportAfter = await api<Report>("report", {
      company_id: created.company_id,
      from: "2026-01-01",
      to: "2026-12-31",
    });
    // Saving display preferences advances the workspace revision; financial report values remain exact.
    expect({ ...reportAfter, _generation: undefined }).toEqual({
      ...reportBefore,
      _generation: undefined,
    });
  } finally {
    await api("settings", initial.settings);
  }
});

test("a Persian system locale is respected before the first screen", async ({
  browser,
}) => {
  const context = await browser.newContext({ locale: "fa-IR" });
  const page = await context.newPage();
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("lang", "fa");
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  await expect(page.locator(".topbar h1")).toHaveText("نمای کلی");
  await context.close();
});

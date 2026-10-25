import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

mkdirSync("/tmp/sarv-contact-bank", { recursive: true });
for (const flow of ["contacts", "banking"])
  test(`${flow}: editable profiles and real ledger movements`, async ({
    page,
    request,
  }) => {
    const profile = mkdtempSync(join(tmpdir(), "sarv-contact-bank-"));
    const token = "sarv-onboarding-isolated-qa-2026",
      backend = "http://127.0.0.1:1423";
    const server = spawn(resolve("../target/debug/examples/desktop_qa"), [], {
      cwd: resolve(".."),
      env: {
        ...process.env,
        SARV_QA_DATA_DIR: profile,
        SARV_QA_TOKEN: token,
        SARV_QA_PORT: "1423",
        SARV_QA_SEED: "1",
      },
      stdio: "ignore",
    });
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const call = async (action: string, payload: unknown = {}) => {
      const r = await request.post(backend, {
        headers: { "x-sarv-qa-token": token },
        data: { action, payload },
      });
      const body = await r.json();
      expect(body.error).toBeUndefined();
      return body.result;
    };
    const nav = async (name: string) =>
      page
        .locator("[data-slot=sidebar]")
        .getByRole("button", { name, exact: true })
        .click();
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
      await page.addInitScript(() =>
        localStorage.setItem("sarv.language", "en"),
      );
      await page.clock.setFixedTime(new Date("2026-10-10T09:00:00Z"));
      await page.goto("/");
      await expect(
        page.getByRole("heading", { name: "Overview", exact: true }),
      ).toBeVisible();
      const initial = await call("workspace");
      const c = initial.companies[0].id;
      const w = await call("workspace", { company_id: c });
      const period = w.periods[0].id;
      if (flow === "contacts") {
        await nav("Contacts");
        await page
          .getByRole("button", { name: "New customer", exact: true })
          .click();
        const dialog = page.getByRole("dialog");
        for (const [label, value] of [
          ["First name", "Sara"],
          ["Last name", "Ahmadi"],
          ["Company / organization", "Corner shop"],
          ["Phone", "02112345678"],
          ["Mobile", "09121234567"],
          ["Email", "sara@example.test"],
          ["Address", "Street 12, Tehran"],
          ["City", "Tehran"],
          ["Postal code", "1234567890"],
          ["National / registration ID", "ID-123"],
          ["Tax / economic ID", "TAX-123"],
          ["Website", "https://example.test"],
          ["Contact notes", "Morning deliveries"],
        ])
          await dialog.getByLabel(label, { exact: true }).fill(value);
        await page
          .getByRole("button", { name: "Create record", exact: true })
          .click();
        await expect(
          page.getByRole("button", { name: "Sara Ahmadi", exact: true }),
        ).toBeVisible();
        const updated = await call("workspace", { company_id: c });
        const customer = updated.resources.parties.find(
          (p: { name: string }) => p.name === "Sara Ahmadi",
        );
        const setup = w.setups[0].id;
        const draft = {
          id: "profile-sale",
          revision: "0",
          company_id: c,
          period_id: period,
          setup_id: setup,
          customer_id: customer.id,
          reference: "PROFILE-ORDER",
          document_date: "2026-10-08",
          financial_date: "2026-10-08",
          date_display: "Iso",
          display_unit: "Rial",
          document_discount: "0",
          supplied_total: "20",
          reason: "Profile analytics test",
          quotation: null,
          lines: [
            {
              id: "profile-line",
              item_id: "ITEM-A",
              warehouse_id: "W1",
              unit_id: "PIECE",
              quantity: "1",
              unit_price: "20",
              line_discount: "0",
              price_reference: "Manual",
              price_date: "2026-10-08",
              override_reason: "",
            },
          ],
        };
        await call("draft_save", { draft, expected_revision: null });
        const order = await call("draft_post", {
          id: draft.id,
          revision: "1",
          request_key: "profile-order",
        });
        await call("fulfill_post", {
          company_id: c,
          period_id: period,
          source_id: "profile-invoice",
          request_key: "profile-invoice",
          date: "2026-10-08",
          reason: "Profile invoice",
          reference: "PROFILE-INVOICE",
          order_id: order.record_id,
          due_date: "2026-10-09",
        });
        await page
          .getByRole("button", { name: "Sara Ahmadi", exact: true })
          .click();
        await expect(dialog.getByLabel("Address", { exact: true })).toHaveValue(
          "Street 12, Tehran",
        );
        await dialog.getByLabel("Mobile", { exact: true }).fill("09120000000");
        await dialog
          .getByRole("button", { name: "Save contact", exact: true })
          .click();
        await expect(
          page.getByText("Contact updated", { exact: true }),
        ).toBeVisible();
        await dialog
          .getByRole("tab", { name: "Performance", exact: true })
          .click();
        await expect(
          dialog.getByRole("heading", {
            name: "Monthly net sales",
            exact: true,
          }),
        ).toBeVisible();
        await expect(
          dialog.getByText(
            w.resources.items.find((i: { id: string }) => i.id === "ITEM-A")
              .name,
            { exact: true },
          ),
        ).toBeVisible();
        await page.screenshot({
          path: "/tmp/sarv-contact-bank/performance.png",
        });
        const detail = await call("contact_detail", {
          company_id: c,
          party_id: customer.id,
          as_of: "2026-10-10",
        });
        expect(detail.stats.net_sales).toBe("20");
        expect(detail.stats.gross_profit).toBe("10");
        expect(detail.stats.outstanding).toBe("20");
        await dialog
          .getByRole("tab", { name: "Linked documents", exact: true })
          .click();
        await expect(
          dialog.getByRole("button", { name: "PROFILE-INVOICE", exact: true }),
        ).toBeVisible();
        await dialog
          .getByRole("button", { name: "PROFILE-INVOICE", exact: true })
          .click();
        await expect(dialog).toContainText("PROFILE-INVOICE");
        await dialog
          .getByRole("button", { name: "View contact profile", exact: true })
          .click();
        await expect(
          page.getByRole("dialog").last().getByLabel("Mobile", { exact: true }),
        ).toHaveValue("09120000000");
        await page
          .getByRole("dialog")
          .last()
          .getByRole("button", { name: "Close dialog", exact: true })
          .click();
        await page
          .getByRole("dialog")
          .getByRole("button", { name: "Close dialog", exact: true })
          .click();
        await page.reload();
        await nav("Contacts");
        await page
          .getByRole("button", { name: "Sara Ahmadi", exact: true })
          .click();
        await expect(dialog.getByLabel("Mobile", { exact: true })).toHaveValue(
          "09120000000",
        );
        await dialog
          .getByRole("button", { name: "Close dialog", exact: true })
          .click();
      } else {
        const equity = w.accounts.find(
          (a: {
            classification: string;
            postable: boolean;
            control: boolean;
          }) => a.classification === "Equity" && a.postable && !a.control,
        );
        const expense = w.accounts.find(
          (a: {
            classification: string;
            postable: boolean;
            control: boolean;
          }) => a.classification === "Expense" && a.postable && !a.control,
        );
        const before = await call("banking", {
          company_id: c,
          from: "2026-01-01",
          to: "2026-10-10",
        });
        const bank = before.accounts.find(
          (a: { role: string }) => a.role === "Bank",
        );
        const cash = before.accounts.find(
          (a: { role: string }) => a.role === "Cash",
        );
        await nav("Banking");
        await expect(
          page.getByRole("heading", {
            name: "Cash and bank statement",
            exact: true,
          }),
        ).toBeVisible();
        await page
          .getByRole("button", { name: "Money in", exact: true })
          .click();
        await page
          .getByRole("dialog")
          .getByLabel("Amount · IRR", { exact: true })
          .fill("10");
        await page
          .getByRole("dialog")
          .getByRole("button", { name: "Close dialog", exact: true })
          .click();
        await expect(page.getByRole("dialog").last()).toContainText(
          "Leave unsaved changes?",
        );
        await page
          .getByRole("button", { name: "Keep editing", exact: true })
          .click();
        await expect(
          page.getByRole("dialog").getByLabel("Amount · IRR", { exact: true }),
        ).toHaveValue("10");
        await page
          .getByRole("dialog")
          .getByRole("button", { name: "Close dialog", exact: true })
          .click();
        await page
          .getByRole("button", { name: "Discard changes", exact: true })
          .click();
        for (const [button, target, amount, reference] of [
          ["Money in", equity.id, "1000", "BANK-FUNDING"],
          ["Transfer money", cash.account.id, "300", "BANK-TRANSFER"],
          ["Money out", expense.id, "100", "BANK-EXPENSE"],
        ]) {
          await page.getByRole("button", { name: button, exact: true }).click();
          const dialog = page.getByRole("dialog");
          await dialog
            .getByLabel(
              button === "Transfer money"
                ? "To account"
                : "Category / counterpart account",
              { exact: true },
            )
            .selectOption(target);
          await dialog.getByLabel("Amount · IRR", { exact: true }).fill(amount);
          await dialog.getByLabel("Reference", { exact: true }).fill(reference);
          await dialog
            .getByLabel("Description", { exact: true })
            .fill("Reviewed movement");
          await expect(
            dialog.getByRole("button", {
              name: "Confirm and post",
              exact: true,
            }),
          ).toBeDisabled();
          await dialog
            .getByRole("button", { name: "Review movement", exact: true })
            .click();
          await expect(
            dialog.getByRole("heading", {
              name: "Financial effect",
              exact: true,
            }),
          ).toBeVisible();
          await dialog
            .getByRole("button", { name: "Confirm and post", exact: true })
            .click();
          await expect(page.getByRole("dialog")).toHaveCount(0);
          await expect(
            page.getByRole("button", { name: reference, exact: true }),
          ).toBeVisible();
        }
        const after = await call("banking", {
          company_id: c,
          from: "2026-01-01",
          to: "2026-10-10",
        });
        expect(
          BigInt(
            after.accounts.find((a: { role: string }) => a.role === "Bank")
              .balance,
          ),
        ).toBe(BigInt(bank.balance) + 600n);
        expect(
          BigInt(
            after.accounts.find((a: { role: string }) => a.role === "Cash")
              .balance,
          ),
        ).toBe(BigInt(cash.balance) + 300n);
        await page.screenshot({ path: "/tmp/sarv-contact-bank/banking.png" });
        await page
          .getByRole("button", { name: "BANK-FUNDING", exact: true })
          .click();
        await expect(page.getByRole("dialog")).toContainText("BANK-FUNDING");
        await page
          .getByRole("button", { name: "Close dialog", exact: true })
          .click();
      }
      await nav("Settings");
      await page
        .getByLabel("Interface language", { exact: true })
        .selectOption("fa");
      await page.getByRole("heading", { name: "تنظیمات", exact: true }).click();
      if (flow === "contacts") {
        await nav("مشتریان");
        await page
          .getByRole("button", { name: "Sara Ahmadi", exact: true })
          .click();
        await expect(
          page
            .getByRole("dialog")
            .getByRole("tab", { name: "اطلاعات تماس", exact: true }),
        ).toBeVisible();
        await page.screenshot({
          path: "/tmp/sarv-contact-bank/persian-contact.png",
        });
        await page.setViewportSize({ width: 390, height: 844 });
        expect(
          await page
            .getByRole("dialog")
            .evaluate((el) => el.scrollWidth - el.clientWidth),
        ).toBeLessThanOrEqual(1);
        await page.screenshot({
          path: "/tmp/sarv-contact-bank/persian-contact-mobile.png",
        });
      } else {
        await page
          .getByLabel("تقویم نمایشی", { exact: true })
          .selectOption("Jalali");
        await page
          .getByRole("button", { name: "ذخیرهٔ تنظیمات", exact: true })
          .click();
        await expect
          .poll(
            async () =>
              (await call("workspace", { company_id: c })).settings
                .date_display,
          )
          .toBe("Jalali");
        await expect(
          page.getByText("تنظیمات ذخیره شد", { exact: true }),
        ).toBeVisible();
        await nav("بانکداری");
        await page
          .getByRole("button", { name: "دریافت وجه", exact: true })
          .click();
        await page
          .getByRole("dialog")
          .getByLabel("تاریخ ثبت مالی", { exact: true })
          .click();
        await expect(page.getByRole("grid")).toBeVisible();
        await expect(
          page
            .getByRole("dialog")
            .getByLabel("تاریخ ثبت مالی", { exact: true }),
        ).toContainText("۱۴۰۵");
        await page.screenshot({
          path: "/tmp/sarv-contact-bank/persian-bank-calendar.png",
        });
      }
      expect(errors).toEqual([]);
    } finally {
      server.kill("SIGTERM");
      await once(server, "exit");
      rmSync(profile, { recursive: true, force: true });
    }
  });

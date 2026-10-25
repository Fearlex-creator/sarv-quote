import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
mkdirSync("/tmp/sarv-product-pricing", { recursive: true });
test("products store prices and metadata, fill document prices, and calculate actual profit from posted costs", async ({
  page,
  request,
}) => {
  page.setDefaultTimeout(10000);
  const profile = mkdtempSync(join(tmpdir(), "sarv-product-"));
  const token = "sarv-onboarding-isolated-qa-2026",
    backend = "http://127.0.0.1:1424";
  const server = spawn(resolve("../target/debug/examples/desktop_qa"), [], {
    cwd: resolve(".."),
    env: {
      ...process.env,
      SARV_QA_DATA_DIR: profile,
      SARV_QA_TOKEN: token,
      SARV_QA_PORT: "1424",
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
    const b = await r.json();
    expect(b.error).toBeUndefined();
    return b.result;
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
    await page.addInitScript(() => localStorage.setItem("sarv.language", "en"));
    await page.clock.setFixedTime(new Date("2026-10-10T09:00:00Z"));
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: "Overview", exact: true }),
    ).toBeVisible();
    const iw = await call("workspace"),
      c = iw.companies[0].id,
      w = await call("workspace", { company_id: c }),
      period = w.periods[0].id;
    await nav("Inventory");
    await page
      .getByRole("button", { name: "New product", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Name", { exact: true }).fill("Handmade notebook");
    for (const [label, value] of [
      ["Default purchase price · IRR", "12500"],
      ["Default selling price · IRR", "20000"],
      ["Barcode", "978-TEST"],
      ["Category", "Stationery"],
      ["Brand", "Local shop"],
      ["Minimum stock · base units", "2"],
      ["Product notes", "Keep dry"],
    ])
      await dialog.getByLabel(label, { exact: true }).fill(value);
    await dialog
      .getByLabel("Preferred supplier", { exact: true })
      .selectOption("SU-A");
    await expect(dialog.getByText("۳۷٫۵۰٪", { exact: true })).toBeVisible();
    await page.screenshot({
      path: "/tmp/sarv-product-pricing/new-product.png",
    });
    await dialog
      .getByRole("button", { name: "Create record", exact: true })
      .click();
    await page.getByRole("tab", { name: "Products", exact: true }).click();
    await page
      .getByRole("button", { name: "Handmade notebook", exact: true })
      .click();
    await expect(dialog.getByLabel("Barcode", { exact: true })).toHaveValue(
      "978-TEST",
    );
    await expect(dialog).toContainText(
      "Low stock: on-hand quantity is at or below your minimum.",
    );
    await dialog
      .getByLabel("Default purchase price · IRR", { exact: true })
      .fill("25000");
    await dialog
      .getByRole("button", { name: "Save product", exact: true })
      .click();
    await expect(
      page.getByText("Product updated", { exact: true }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Close dialog", exact: true })
      .click();
    await page
      .getByRole("button", {
        name: w.resources.items.find((i: { id: string }) => i.id === "ITEM-A")
          .name,
        exact: true,
      })
      .click();
    await dialog
      .getByLabel("Default purchase price · IRR", { exact: true })
      .fill("125000");
    await dialog
      .getByLabel("Default selling price · IRR", { exact: true })
      .fill("200000");
    await dialog
      .getByLabel("Minimum stock · base units", { exact: true })
      .fill("40");
    await dialog
      .getByRole("button", { name: "Save product", exact: true })
      .click();
    await expect(
      dialog.getByRole("button", { name: "Save product", exact: true }),
    ).toBeDisabled();
    await dialog
      .getByRole("button", { name: "Close dialog", exact: true })
      .click();
    await page.keyboard.press("Control+Alt+O");
    await page.getByLabel("Customer", { exact: true }).selectOption("CU-A");
    await page
      .getByLabel("Order reference", { exact: true })
      .fill("PRICE-ORDER");
    await page.getByLabel("Product 1", { exact: true }).selectOption("ITEM-A");
    await expect(page.getByLabel("Price 1", { exact: true })).toHaveValue(
      "200000",
    );
    await page.getByLabel("Warehouse 1", { exact: true }).selectOption("W1");
    await page
      .getByRole("button", { name: "Review order", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Record order", exact: true })
      .click();
    await page.getByRole("tab", { name: "Orders", exact: true }).click();
    await page
      .getByRole("button", { name: "PRICE-ORDER", exact: true })
      .click();
    await dialog
      .getByLabel("New document reference", { exact: true })
      .fill("PRICE-INVOICE");
    await dialog
      .getByRole("button", { name: "Review financial effect", exact: true })
      .click();
    await dialog
      .getByRole("button", { name: "Confirm and post", exact: true })
      .click();
    await expect(
      page.getByText("Goods operation posted", { exact: true }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Close dialog", exact: true })
      .click();
    await page.keyboard.press("Control+Alt+B");
    await expect(dialog).toContainText("New purchase order");
    await dialog.getByLabel("Supplier", { exact: true }).selectOption("SU-A");
    await dialog
      .getByLabel("Purchase product 1", { exact: true })
      .selectOption("ITEM-A");
    await expect(
      dialog.getByLabel("Purchase price 1", { exact: true }),
    ).toHaveValue("125000");
    await dialog
      .getByLabel("Purchase warehouse 1", { exact: true })
      .selectOption("W1");
    await dialog.getByLabel("Reference", { exact: true }).fill("PRICE-PO");
    await dialog
      .getByRole("button", { name: "Review order", exact: true })
      .click();
    await dialog
      .getByRole("button", { name: "Record purchase order", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    const updated = await call("workspace", { company_id: c });
    const po = updated.purchases.find(
      (r: { command: { Order?: { reference: string } } }) =>
        r.command.Order?.reference === "PRICE-PO",
    );
    expect(po).toBeDefined();
    await call("receive_post", {
      company_id: c,
      period_id: period,
      source_id: "price-receipt",
      request_key: "price-receipt",
      date: "2026-10-10",
      reason: "Price receipt test",
      reference: "PRICE-RECEIPT",
      order_id: po.id,
    });
    const detail = await call("product_detail", {
      company_id: c,
      item_id: "ITEM-A",
      as_of: "2026-10-10",
    });
    expect(detail.stats.net_sales).toBe("200000");
    expect(detail.stats.cost_of_goods_sold).toBe("10");
    expect(detail.stats.gross_profit).toBe("199990");
    expect(detail.stats.last_purchase_price).toBe("125000");
    expect(detail.stats.average_stock_cost).toBe("4176");
    await nav("Inventory");
    await page.getByRole("tab", { name: "Products", exact: true }).click();
    await page
      .getByRole("button", {
        name: w.resources.items.find((i: { id: string }) => i.id === "ITEM-A")
          .name,
        exact: true,
      })
      .click();
    await dialog.getByRole("tab", { name: "Performance", exact: true }).click();
    await expect(
      dialog.getByRole("heading", {
        name: "Monthly product performance",
        exact: true,
      }),
    ).toBeVisible();
    await page.screenshot({
      path: "/tmp/sarv-product-pricing/performance.png",
    });
    await dialog
      .getByRole("tab", { name: "Linked documents", exact: true })
      .click();
    await expect(
      dialog.getByRole("button", { name: "PRICE-INVOICE", exact: true }),
    ).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "PRICE-RECEIPT", exact: true }),
    ).toBeVisible();
    await dialog
      .getByRole("button", { name: "Close dialog", exact: true })
      .click();
    await nav("Settings");
    await page
      .getByLabel("Interface language", { exact: true })
      .selectOption("fa");
    await page.getByRole("heading", { name: "تنظیمات", exact: true }).click();
    await nav("موجودی");
    await page.getByRole("tab", { name: "کالاها", exact: true }).click();
    await page
      .getByRole("button", { name: "Handmade notebook", exact: true })
      .click();
    await expect(
      dialog.getByLabel("قیمت پیش‌فرض خرید · IRR", { exact: true }),
    ).toHaveValue("25000");
    await page.setViewportSize({ width: 390, height: 844 });
    expect(
      await dialog.evaluate((el) => el.scrollWidth - el.clientWidth),
    ).toBeLessThanOrEqual(1);
    await page.screenshot({
      path: "/tmp/sarv-product-pricing/persian-mobile.png",
    });
    expect(errors).toEqual([]);
  } finally {
    server.kill("SIGTERM");
    await once(server, "exit");
    rmSync(profile, { recursive: true, force: true });
  }
});

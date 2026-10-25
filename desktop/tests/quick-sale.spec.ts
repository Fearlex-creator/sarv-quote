import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const token = "sarv-quick-sale-qa-token-2026-xx";
const port = process.env.SARV_SALE_QA_PORT ?? "1433";
const backend = `http://127.0.0.1:${port}`;

test("a sale is completed from one screen with the keyboard, and a new zero-stock item becomes a backorder", async ({
  page,
  request,
}) => {
  const profile = mkdtempSync(join(tmpdir(), "sarv-quick-sale-"));
  const server = spawn(resolve("../target/debug/examples/desktop_qa"), [], {
    cwd: resolve(".."),
    env: {
      ...process.env,
      SARV_QA_DATA_DIR: profile,
      SARV_QA_TOKEN: token,
      SARV_QA_SEED: "1",
      SARV_QA_PORT: port,
    },
    stdio: "ignore",
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
  try {
    await expect
      .poll(async () =>
        request.get(backend).then((r) => r.status()).catch(() => 0),
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
    // Sidebar starts collapsed.
    await expect(page.locator(".brand-text")).toBeHidden();
    const companyId = (await api("workspace")).companies[0].id;
    const sales = async () =>
      (await api("workspace", { company_id: companyId })).sales.length;
    const before = await sales();

    await page.getByRole("button", { name: "New sales invoice" }).first().click();
    // Product 1: keyboard only. Customer stays empty (walk-in).
    const product = page.getByRole("combobox", { name: "Product 1" });
    await product.click();
    await product.fill("الف");
    await page.keyboard.press("Enter"); // pick, jump to quantity
    await expect(page.getByLabel("Quantity 1")).toBeFocused();
    await page.keyboard.press("Enter"); // price
    await expect(page.getByLabel("Price 1")).toBeFocused();
    await page.keyboard.type("5000");
    await page.keyboard.press("Enter"); // discount
    await expect(page.getByLabel("Discount 1")).toBeFocused();
    await page.keyboard.press("Enter"); // next row
    await expect(page.getByRole("combobox", { name: "Product 2" })).toBeFocused();
    await page.keyboard.press("Control+Enter"); // complete
    await expect(page.getByRole("heading", { name: "Sale completed" })).toBeVisible({ timeout: 15000 });
    expect(await sales()).toBeGreaterThanOrEqual(before + 2);
    await page.getByRole("button", { name: "New sale", exact: true }).click();

    // A brand-new item with no stock: created inline, recorded as a backorder.
    const p1 = page.getByRole("combobox", { name: "Product 1" });
    await p1.click();
    await p1.fill("Turbine X1");
    await expect(page.getByRole("option", { name: /Create "Turbine X1"/ })).toBeVisible();
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByLabel(/Your cost/)).toBeFocused();
    await page.keyboard.type("3000");
    await page.keyboard.press("Enter");
    await page.keyboard.type("5000");
    await page.keyboard.press("Enter"); // last field submits
    await expect(dialog).not.toBeVisible();
    await expect(page.getByText(/Out of stock — will be backordered/)).toBeVisible();
    await page.getByRole("button", { name: "Record as backorder" }).click();
    await expect(page.getByRole("heading", { name: "Recorded as a backorder" })).toBeVisible({ timeout: 15000 });
    await page.getByRole("button", { name: "Create purchase order" }).click();
    // The purchase order opens with the missing goods already listed.
    const po = page.getByRole("dialog");
    await expect(po.getByLabel("Purchase product 1", { exact: true })).not.toHaveValue("");
    await expect(po.getByLabel("Purchase price 1", { exact: true })).toHaveValue("30000"); // the purchase editor shows rials
  } finally {
    if (server.exitCode === null && server.signalCode === null) {
      server.kill("SIGTERM");
      await once(server, "exit");
    }
    rmSync(profile, { recursive: true, force: true });
  }
});

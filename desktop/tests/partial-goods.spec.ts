import type { Workspace, Report } from "../src/lib/types";
import { test, expect } from "@playwright/test";
import { mkdirSync } from "node:fs";
const token =
  process.env.SARV_QA_TOKEN ?? "sarv-isolated-browser-qa-token-2026";
const out = "../artifacts/tauri-ui";
mkdirSync(out, { recursive: true });
test("partial receipts, billing, delivery, original-cost returns and transfer reconcile", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  async function api<T = unknown>(
    action: string,
    payload: unknown,
  ): Promise<T> {
    const response = await request.post("http://127.0.0.1:1421", {
      headers: { "x-sarv-qa-token": token },
      data: { action, payload },
    });
    const body = await response.json();
    expect(body.error, JSON.stringify(body)).toBeUndefined();
    return body.result as T;
  }
  const prefix = `partial-${Date.now()}`;
  const created = await api<{ company_id: string; period_id: string }>(
    "setup_company",
    {
      name: prefix,
      fiscal_name: "2026",
      start_date: "2026-01-01",
      end_date: "2026-12-31",
    },
  );
  const preferences = (
    await api<Workspace>("workspace", { company_id: created.company_id })
  ).settings;
  await api("settings", {
    ...preferences,
    display_digits: "Latin",
    default_currency: "Rial",
  });
  try {
    const company = created.company_id;
    const period = created.period_id;
    const context = (key: string) => ({
      company_id: company,
      period_id: period,
      source_id: `${prefix}-${key}`,
      request_key: `${prefix}-${key}`,
      date: "2026-10-10",
      reason: "Independent partial goods browser test",
      reference: `${prefix}-${key}`,
      due_date: "2026-10-30",
    });
    await api("initialize_zero", {
      company_id: company,
      source_id: `${prefix}-opening`,
      request_key: `${prefix}-opening`,
      confirmed: true,
    });
    for (const [kind, id, name] of [
      ["unit", "unit", "Piece"],
      ["warehouse", "main", "Main"],
      ["warehouse", "other", "Other"],
      ["vendor", "vendor", "Supplier"],
      ["customer", "customer", "Customer"],
      ["item", "item", "Partial goods"],
    ])
      await api("resource", {
        ...context(id),
        kind,
        id: `${prefix}-${id}`,
        name,
        sku: prefix,
        unit_id: `${prefix}-unit`,
      });
    const purchase = await api<{ record_id: string }>("purchase_post", {
      ...context("po"),
      supplier_id: `${prefix}-vendor`,
      discount: "2",
      lines: [
        {
          id: `${prefix}-po-line`,
          item_id: `${prefix}-item`,
          warehouse_id: `${prefix}-main`,
          unit_id: `${prefix}-unit`,
          quantity: "3",
          unit_price: "4",
          line_discount: "0",
        },
      ],
    });
    await page.goto("/");
    await page.getByLabel("Organization").selectOption(company);
    const nav = async (name: string) =>
      page
        .locator(".sidebar")
        .getByRole("button", { name, exact: true })
        .click();
    const close = async () =>
      page
        .getByRole("dialog")
        .getByRole("button", { name: "Close dialog" })
        .click();
    const open = async (reference: string) =>
      page.getByRole("button", { name: reference, exact: true }).click();
    async function post(
      reference: string,
      quantity?: string,
      screenshot?: string,
    ) {
      const d = page.getByRole("dialog");
      await expect(
        d.getByLabel("Quantity for Partial goods in Main"),
      ).toBeVisible();
      if (quantity)
        await d.getByLabel("Quantity for Partial goods in Main").fill(quantity);
      await d
        .getByLabel("New document reference")
        .fill(`${prefix}-${reference}`);
      await d.getByRole("button", { name: "Review financial effect" }).click();
      await expect(
        d.getByRole("button", { name: "Confirm and post" }),
      ).toBeEnabled();
      if (screenshot)
        await page.screenshot({ path: `${out}/${screenshot}.png` });
      await d.getByRole("button", { name: "Confirm and post" }).click();
      await expect(d.locator(".operation-result")).toBeVisible();
      await close();
    }
    await nav("Purchases");
    await open(`${prefix}-po`);
    // Over-receiving fails before any posting, then a corrected partial succeeds.
    const d = page.getByRole("dialog");
    await d.getByLabel("Quantity for Partial goods in Main").fill("4");
    await d.getByLabel("New document reference").fill(`${prefix}-gr1`);
    await d.getByRole("button", { name: "Review financial effect" }).click();
    await expect(
      d.getByText("Quantity exceeds the remaining source quantity"),
    ).toBeVisible();
    await expect(
      d.getByRole("button", { name: "Confirm and post" }),
    ).toBeDisabled();
    await post("gr1", "1", "partial-receipt-review");
    await open(`${prefix}-po`);
    await expect(
      page.getByRole("dialog").getByLabel("Quantity for Partial goods in Main"),
    ).toHaveValue("2");
    await post("gr2");
    await page.getByRole("tab", { name: "Receipts", exact: true }).click();
    await open(`${prefix}-gr2`);
    await post("bill", "1", "partial-bill-review");
    let workspace = await api<Workspace>("workspace", { company_id: company });
    const draftId = `${prefix}-draft`;
    await api("draft_save", {
      expected_revision: null,
      draft: {
        id: draftId,
        revision: "0",
        company_id: company,
        period_id: period,
        setup_id: workspace.setups[0].id,
        customer_id: `${prefix}-customer`,
        reference: `${prefix}-so`,
        document_date: "2026-10-10",
        financial_date: "2026-10-10",
        date_display: "Iso",
        display_unit: "Rial",
        document_discount: "1",
        supplied_total: "39",
        reason: "Independent partial sales",
        quotation: null,
        lines: [
          {
            id: `${prefix}-sale-line`,
            item_id: `${prefix}-item`,
            warehouse_id: `${prefix}-main`,
            unit_id: `${prefix}-unit`,
            quantity: "2",
            unit_price: "20",
            line_discount: "0",
            price_reference: "Manual",
            price_date: "2026-10-10",
            override_reason: "",
          },
        ],
      },
    });
    await api("draft_post", {
      id: draftId,
      revision: "1",
      request_key: `${prefix}-order-post`,
    });
    await page.reload();
    await nav("Sales");
    await open(`${prefix}-so`);
    await post("inv1", "1", "partial-invoice-review");
    await open(`${prefix}-so`);
    await expect(
      page.getByRole("dialog").getByLabel("Quantity for Partial goods in Main"),
    ).toHaveValue("1");
    await post("inv2");
    await page.getByRole("tab", { name: "Invoices", exact: true }).click();
    await open(`${prefix}-inv1`);
    await post("sales-return", "1", "customer-return-review");
    await page.getByRole("tab", { name: "Returns", exact: true }).click();
    await expect(
      page.getByRole("button", { name: `${prefix}-sales-return`, exact: true }),
    ).toBeVisible();
    await nav("Purchases");
    await page.getByRole("tab", { name: "Bills", exact: true }).click();
    await open(`${prefix}-bill`);
    await post("supplier-return", "1", "supplier-return-review");
    await page.getByRole("tab", { name: "Returns", exact: true }).click();
    await expect(
      page.getByRole("button", {
        name: `${prefix}-supplier-return`,
        exact: true,
      }),
    ).toBeVisible();
    await nav("Inventory");
    await page
      .getByRole("button", { name: "Transfer stock", exact: true })
      .click();
    const t = page.getByRole("dialog");
    await t.getByLabel("Reason").fill("Move remaining stock to Other");
    await t.getByRole("button", { name: "Review transfer" }).click();
    await expect(t.getByText("Value:", { exact: false })).toContainText("2");
    await page.screenshot({ path: `${out}/warehouse-transfer-review.png` });
    await t.getByRole("button", { name: "Confirm and transfer" }).click();
    await expect(t).toHaveCount(0);
    await page.screenshot({ path: `${out}/partial-stock-result.png` });
    const report = await api<Report>("report", {
      company_id: company,
      from: "2026-01-01",
      to: "2026-10-10",
    });
    expect(report.position.receivables).toBe("19");
    expect(report.position.payables).toBe("0");
    expect(report.profit_loss.profit).toBe("15");
    expect(report.reconciliation.reconciled).toBe(true);
    expect(
      report.reconciliation.stock.find(
        (s) => s.warehouse_id === `${prefix}-other`,
      ),
    ).toMatchObject({ quantity_thousandths: "1000", value: "2" });
    workspace = await api<Workspace>("workspace", { company_id: company });
    expect(workspace.sales_count).toBe("4");
    expect(workspace.purchase_count).toBe("12");
    expect(
      (
        await api<{ lines: unknown[] }>("operation_options", {
          company_id: company,
          record_id: purchase.record_id,
          operation: "receive",
        })
      ).lines,
    ).toEqual([]);
    expect(errors).toEqual([]);
  } finally {
    await api("settings", preferences);
  }
});

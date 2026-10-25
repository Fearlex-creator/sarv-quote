import { test, expect } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
const out = "../artifacts/tauri-ui";
mkdirSync(out, { recursive: true });
test("complete native accounting presentation against the real Rust dispatcher", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  async function nav(label: string) {
    await page
      .locator(".sidebar")
      .getByRole("button", { name: label, exact: true })
      .click();
  }
  async function shot(name: string) {
    await expect(page.locator(".loading")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Working…", exact: true }),
    ).toHaveCount(0);
    await page.screenshot({
      path: `${out}/${name}.png`,
      fullPage: false,
      animations: "disabled",
    });
  }
  async function resource(kind: string, name: string) {
    if (kind === "customer") {
      await nav("Contacts");
      await page.getByRole("tab", { name: "Customers", exact: true }).click();
      await page
        .getByRole("button", { name: "New customer", exact: true })
        .click();
    } else if (kind === "vendor") {
      await nav("Contacts");
      await page.getByRole("tab", { name: "Suppliers", exact: true }).click();
      await page
        .getByRole("button", { name: "New vendor", exact: true })
        .click();
    } else {
      await nav("Inventory");
      await page
        .getByRole("tab", {
          name:
            kind === "unit"
              ? "Units"
              : kind === "warehouse"
                ? "Warehouses"
                : "Products",
          exact: true,
        })
        .click();
      await page
        .getByRole("button", {
          name: `New ${kind === "item" ? "product" : kind}`,
          exact: true,
        })
        .click();
    }
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Name", { exact: true }).fill(name);
    if (kind === "item") {
      await dialog.getByLabel("Product code (optional)").fill("QA-ITEM");
      await dialog
        .getByLabel("Base unit", { exact: true })
        .selectOption({ label: "Piece" });
    }
    await dialog
      .getByRole("button", { name: "Create record", exact: true })
      .click();
    await expect(dialog).toHaveCount(0);
    await expect(page.getByRole("cell", { name, exact: true })).toBeVisible();
  }
  await page.addInitScript(() => {
    if (!localStorage.getItem("sarv.sidebar"))
      localStorage.setItem("sarv.sidebar", "expanded");
  });
  await page.clock.setFixedTime(new Date("2026-10-10T09:00:00Z"));
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Overview", exact: true }),
  ).toBeVisible();
  // Configure display using the existing durable settings service.
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Amount display").selectOption("Rial");
  await page.getByLabel("Display digits").selectOption("Latin");
  await page.getByLabel("Theme", { exact: true }).selectOption("Light");
  await page
    .getByRole("button", { name: "Save preferences", exact: true })
    .click();
  await expect(
    page.getByText("Preferences saved", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Add company", exact: true }).click();
  await shot("onboarding");
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Company name").fill("QA Accounting Studio");
  await dialog
    .getByRole("button", { name: "Create company", exact: true })
    .click();
  await expect(
    dialog.getByRole("heading", { name: "Record opening balances" }),
  ).toBeVisible();
  await shot("opening-balances");
  await dialog.getByRole("checkbox").check();
  await dialog
    .getByRole("button", { name: "Start with zero balances" })
    .click();
  await expect(dialog).toHaveCount(0);
  await page.reload();
  await expect(page.getByLabel("Organization")).toHaveValue(/.+/);
  await expect(
    page
      .locator(".profile-row")
      .getByText("QA Accounting Studio", { exact: true }),
  ).toBeVisible();
  await resource("customer", "QA Customer");
  await shot("customers");
  await resource("vendor", "QA Supplier");
  await resource("unit", "Piece");
  await resource("warehouse", "Main warehouse");
  await resource("item", "QA Goods");
  await nav("Purchases");
  await page
    .getByRole("button", { name: "New purchase order", exact: true })
    .click();
  let d = page.getByRole("dialog");
  await d
    .getByLabel("Supplier", { exact: true })
    .selectOption({ label: "QA Supplier" });
  await d.getByLabel("Reference", { exact: true }).fill("PO-QA-001");
  await d
    .getByLabel("Purchase product 1", { exact: true })
    .selectOption({ label: "QA Goods" });
  await d
    .getByLabel("Purchase warehouse 1", { exact: true })
    .selectOption({ label: "Main warehouse" });
  await d.getByLabel("Purchase quantity 1", { exact: true }).fill("5");
  await d.getByLabel("Purchase price 1", { exact: true }).fill("10");
  await d.getByRole("button", { name: "Review order", exact: true }).click();
  await expect(d.getByText("50", { exact: false }).first()).toBeVisible();
  await d
    .getByRole("button", { name: "Record purchase order", exact: true })
    .click();
  await expect(d).toHaveCount(0);
  await page.getByRole("button", { name: "PO-QA-001", exact: true }).click();
  d = page.getByRole("dialog");
  await d.getByLabel("New document reference").fill("GR-QA-001");
  await d.getByRole("button", { name: "Review financial effect" }).click();
  await expect(
    d.getByRole("button", { name: "Confirm and post" }),
  ).toBeEnabled();
  await d.getByRole("button", { name: "Confirm and post" }).click();
  await expect(d.locator(".operation-result")).toBeVisible();
  await d.getByRole("button", { name: "Close dialog" }).click();
  await page.getByRole("tab", { name: "Receipts", exact: true }).click();
  await page.getByRole("button", { name: "GR-QA-001", exact: true }).click();
  d = page.getByRole("dialog");
  await d.getByLabel("New document reference").fill("BILL-QA-001");
  await d.getByRole("button", { name: "Review financial effect" }).click();
  await d.getByRole("button", { name: "Confirm and post" }).click();
  await expect(d.locator(".operation-result")).toBeVisible();
  await shot("bill-detail");
  await d.getByRole("button", { name: "Close dialog" }).click();
  await nav("Sales");
  await page.getByRole("button", { name: "New order", exact: true }).click();
  await page
    .getByLabel("Customer", { exact: true })
    .selectOption({ label: "QA Customer" });
  await page.getByLabel("Order reference").fill("SO-QA-001");
  await page
    .getByLabel("Product 1", { exact: true })
    .selectOption({ label: "QA-ITEM · QA Goods" });
  await page
    .getByLabel("Warehouse 1", { exact: true })
    .selectOption({ label: "Main warehouse" });
  await page.getByLabel("Quantity 1", { exact: true }).fill("2");
  await page.getByLabel("Price 1", { exact: true }).fill("20");
  await page.getByLabel("Order discount · IRR").fill("1");
  await shot("order-editor");
  await page.getByRole("button", { name: "Save draft", exact: true }).click();
  await expect(page.getByText("Draft saved", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Back to sales", exact: false })
    .click();
  await page.getByRole("tab", { name: "Drafts", exact: false }).click();
  await page.getByRole("button", { name: "SO-QA-001", exact: true }).click();
  await page.getByRole("button", { name: "Review order", exact: true }).click();
  d = page.getByRole("dialog");
  await expect(d.getByText("39", { exact: true })).toBeVisible();
  await d.getByRole("button", { name: "Record order", exact: true }).click();
  await expect(d).toHaveCount(0);
  await page.getByRole("tab", { name: "Orders", exact: true }).click();
  await page.getByRole("button", { name: "SO-QA-001", exact: true }).click();
  d = page.getByRole("dialog");
  await d.getByLabel("New document reference").fill("INV-QA-001");
  await d.getByRole("button", { name: "Review financial effect" }).click();
  await shot("invoice-review");
  await d.getByRole("button", { name: "Confirm and post" }).click();
  await expect(d.locator(".operation-result")).toBeVisible();
  await d.getByRole("button", { name: "Close dialog" }).click();
  await page.getByRole("tab", { name: "Invoices", exact: true }).click();
  await shot("invoices-list");
  await page.getByRole("button", { name: "INV-QA-001", exact: true }).click();
  await shot("invoice-detail");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Close dialog" })
    .click();
  await nav("Reports");
  await expect(
    page.getByRole("heading", { name: "Profit & Loss", exact: true }),
  ).toBeVisible();
  await expect(page.locator("tfoot .money")).toHaveText("19");
  await shot("profit-loss");
  await page
    .getByRole("tab", { name: "Control reconciliation", exact: true })
    .click();
  await expect(
    page.getByText("All controls agree", { exact: true }),
  ).toBeVisible();
  await shot("control-reconciliation");
  await nav("Accounting");
  await page.getByRole("button", { name: "New journal", exact: true }).click();
  d = page.getByRole("dialog");
  await d.getByLabel("Reason", { exact: true }).fill("QA expense allocation");
  await d
    .getByLabel("Journal account 1", { exact: true })
    .selectOption({ label: "5201 · هزینه‌های عملیاتی" });
  await d.getByLabel("Journal debit 1", { exact: true }).fill("5");
  await d
    .getByLabel("Journal account 2", { exact: true })
    .selectOption({ label: "3101 · سرمایه" });
  await d.getByLabel("Journal credit 2", { exact: true }).fill("4");
  await d
    .getByRole("button", { name: "Validate journal", exact: true })
    .click();
  await expect(d.getByRole("alert")).toBeVisible();
  await expect(
    d.getByRole("button", { name: "Confirm and post journal", exact: true }),
  ).toBeDisabled();
  await d.getByLabel("Journal credit 2", { exact: true }).fill("5");
  await d
    .getByRole("button", { name: "Validate journal", exact: true })
    .click();
  await expect(
    d.getByRole("button", { name: "Confirm and post journal", exact: true }),
  ).toBeEnabled();
  await shot("journal-editor");
  await d
    .getByRole("button", { name: "Confirm and post journal", exact: true })
    .click();
  await expect(d).toHaveCount(0);
  await nav("Reports");
  await expect(page.locator("tfoot .money")).toHaveText("14");
  await page.getByRole("tab", { name: "Trial balance", exact: true }).click();
  await expect(page.getByText("Balanced", { exact: true })).toBeVisible();
  const dl = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export CSV", exact: true }).click();
  const download = await dl;
  await download.saveAs(`${out}/trial-balance.csv`);
  await page
    .getByRole("button", { name: "حساب‌های دریافتنی", exact: true })
    .click();
  await shot("general-ledger");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Close dialog" })
    .click();
  await nav("Banking");
  await shot("banking-unsupported");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await shot("settings");
  await page.getByLabel("Theme", { exact: true }).selectOption("Dark");
  await page
    .getByRole("button", { name: "Save preferences", exact: true })
    .click();
  await expect(page.locator("html")).toHaveClass("dark");
  await nav("Overview");
  await shot("dark-mode");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Interface language").selectOption("fa");
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  await shot("persian-rtl");
  await page.getByLabel("زبان برنامه").selectOption("en");
  await page.getByLabel("Theme", { exact: true }).selectOption("Light");
  await page
    .getByRole("button", { name: "Save preferences", exact: true })
    .click();
  await nav("Overview");
  await page.keyboard.press("Control+k");
  await expect(
    page.getByRole("dialog", { name: "Command menu" }),
  ).toBeVisible();
  await expect(page.getByPlaceholder("Type a command…")).toBeFocused();
  await page.keyboard.type("Sales");
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Sales", exact: true }),
  ).toBeVisible();
  await page.keyboard.press("F9");
  await expect(page.locator(".sidebar")).toHaveClass(/sidebar-collapsed/);
  await page.reload();
  await expect(page.locator(".sidebar")).toHaveClass(/sidebar-collapsed/);
  await page.keyboard.press("F9");
  for (const [width, height] of [
    [1440, 900],
    [1920, 1080],
    [1100, 720],
    [900, 640],
  ]) {
    await page.setViewportSize({ width, height });
    await nav("Overview");
    await expect(
      page.getByRole("heading", { name: "Overview", exact: true }),
    ).toBeVisible();
    await shot(`overview-${width}x${height}`);
    await nav("Sales");
    await page.getByRole("tab", { name: "Invoices", exact: true }).click();
    await shot(`invoices-${width}x${height}`);
    for (const route of [
      "Purchases",
      "Inventory",
      "Accounting",
      "Reports",
      "Contacts",
      "Settings",
    ]) {
      await nav(route);
      await shot(`${route.toLowerCase()}-${width}x${height}`);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        ),
      ).toBe(false);
    }
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    );
    expect(overflow).toBe(false);
  }
  await nav("Accounting");
  const journalTab = page.getByRole("tab", { name: "Journals", exact: true });
  await journalTab.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "Chart of accounts", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("End");
  await expect(
    page.getByRole("tab", { name: "Fiscal periods", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  expect(errors).toEqual([]);
});

test("opening CSV presents reconciled controls and persists only after review", async ({
  page,
  request,
}) => {
  const api = async (action: string, payload: unknown) => {
    const response = await request.post("http://127.0.0.1:1421", {
      headers: {
        "x-sarv-qa-token":
          process.env.SARV_QA_TOKEN ?? "sarv-isolated-browser-qa-token-2026",
      },
      data: { action, payload },
    });
    const body = await response.json();
    expect(body.error).toBeUndefined();
    return body.result;
  };
  const created = await api("setup_company", {
    name: "QA CSV Opening",
    fiscal_name: "1405",
    start_date: "2026-03-21",
    end_date: "2027-03-20",
  });
  await page.addInitScript((id) => {
    localStorage.setItem("sarv.company", id);
    localStorage.setItem("sarv.language", "en");
  }, created.company_id);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Record opening balances", exact: true })
    .click();
  const dialog = page.getByRole("dialog");
  const template = readFileSync(
    "../docs/assets/accounting-m3/opening.csv",
    "utf8",
  ).trim();
  // Resource identities are global in the kernel. Give this separate QA company
  // its own identities while preserving all references in the trusted fixture.
  const identities = new Map(
    template
      .split(/\r?\n/)
      .slice(1)
      .map((line) => {
        const id = line.split(",")[2];
        return [id, `${created.company_id}-${id}`];
      }),
  );
  const csv = template
    .split(/\r?\n/)
    .map((line) =>
      line
        .split(",")
        .map((cell) =>
          cell === "F2-core-v1"
            ? created.company_id
            : (identities.get(cell) ?? cell),
        )
        .join(","),
    )
    .join("\n");
  await dialog.getByLabel("Opening CSV", { exact: true }).fill(csv);
  await dialog
    .getByRole("button", { name: "Validate opening", exact: true })
    .click();
  await expect(
    dialog.getByRole("table", { name: "Opening control review" }),
  ).toBeVisible();
  await expect(dialog.locator("[data-slot=alert] .money")).toHaveText(
    /1,251\s*IRR/,
  );
  expect(
    (await api("workspace", { company_id: created.company_id })).initialized,
  ).toBe(false);
  await page.screenshot({ path: `${out}/opening-csv-review.png` });
  await dialog
    .getByRole("button", { name: "Record validated opening", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Overview", exact: true }),
  ).toBeVisible();
  const books = await api("workspace", { company_id: created.company_id });
  expect(books.initialized).toBe(true);
  expect(books.journal_count).toBe("1");
  const report = await api("report", {
    company_id: created.company_id,
    from: "2026-03-21",
    to: "2026-10-10",
  });
  expect(report.trial.total_debit).toBe("1251");
  expect(report.reconciliation.reconciled).toBe(true);
});

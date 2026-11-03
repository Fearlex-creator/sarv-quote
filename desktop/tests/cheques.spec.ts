import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const token = "sarv-cheques-qa-token-2026-xxxx";
const port = process.env.SARV_CHEQUE_QA_PORT ?? "1456";
const backend = `http://127.0.0.1:${port}`;

test("a customer cheque is received, deposited and cleared from the Payments screen", async ({
  page,
  request,
}) => {
  const profile = mkdtempSync(join(tmpdir(), "sarv-cheques-"));
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
    expect(body.error, JSON.stringify(body)).toBeUndefined();
    return body.result;
  };
  try {
    await expect
      .poll(async () => request.get(backend).then((r) => r.status()).catch(() => 0))
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
    const companyId = (await api("workspace")).companies[0].id;
    const cheques = async () =>
      (await api("workspace", { company_id: companyId })).cheques as {
        status: string;
        allowed: string[];
      }[];
    expect(await cheques()).toHaveLength(0);

    await page.locator('[data-slot="sidebar"]').getByRole("button", { name: "Payments", exact: true }).click();
    await page.getByRole("button", { name: "Receive money", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Customer", { exact: true }).selectOption({ index: 1 });
    await dialog.getByLabel("Cash / bank account", { exact: true }).selectOption("cheque");
    // The cheque needs a valid 16-digit Sayad ID before the payment can be reviewed.
    await dialog.getByLabel("Sayad ID", { exact: true }).fill("12345");
    await expect(dialog.getByText("A Sayad ID has exactly 16 digits.")).toBeVisible();
    await dialog.getByLabel("Sayad ID", { exact: true }).fill("1234567890123456");
    await expect(dialog.getByText("A Sayad ID has exactly 16 digits.")).toHaveCount(0);
    await dialog.getByLabel("Cheque number", { exact: true }).fill("778899");
    await dialog.getByLabel("Cheque bank", { exact: true }).fill("Melli");
    await dialog.getByLabel("Reference", { exact: true }).fill("CHQ-E2E-1");
    await dialog.getByRole("button", { name: "Full" }).first().click();
    await dialog.getByRole("button", { name: "Review payment" }).click();
    await expect(dialog.getByRole("heading", { name: "Financial effect" })).toBeVisible();
    await dialog.getByRole("button", { name: "Confirm and post" }).click();
    await expect(dialog).not.toBeVisible({ timeout: 15000 });

    // The cheque is listed, in hand, with only the steps the state machine allows.
    const row = page.getByRole("row").filter({ hasText: "1234 5678 9012 3456" });
    await expect(row).toBeVisible();
    await expect(row.getByText("In hand")).toBeVisible();
    await expect(row.getByRole("button", { name: "Mark as bounced" })).toHaveCount(0);
    expect((await cheques())[0].allowed).toEqual(["deposit", "clear", "spend", "return"]);

    await row.getByRole("button", { name: "Deposit for collection" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "Confirm" }).click();
    await expect(row.getByText("In collection")).toBeVisible({ timeout: 15000 });

    await row.getByRole("button", { name: "Mark as cleared" }).click();
    const clear = page.getByRole("dialog");
    await expect(clear.getByRole("button", { name: "Confirm" })).toBeDisabled();
    await clear.getByLabel("Bank account", { exact: true }).selectOption({ index: 1 });
    await clear.getByRole("button", { name: "Confirm" }).click();
    await expect(row.getByText("Cleared")).toBeVisible({ timeout: 15000 });
    await expect(row.getByRole("button")).toHaveCount(0);

    const final = await cheques();
    expect(final[0].status).toBe("cleared");
    expect(final[0].allowed).toEqual([]);
    // The books stay consistent after the whole lifecycle.
    expect((await api("integrity")).status).not.toBe("failed");
  } finally {
    if (server.exitCode === null && server.signalCode === null) {
      server.kill("SIGTERM");
      await once(server, "exit");
    }
    rmSync(profile, { recursive: true, force: true });
  }
});

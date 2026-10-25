import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const token = "sarv-onboarding-bank-qa-2026-xx";
const port = process.env.SARV_BANK_QA_PORT ?? "1432";
const backend = `http://127.0.0.1:${port}`;

test("owner adds the main bank account and cash while onboarding", async ({
  page,
  request,
}) => {
  const profile = mkdtempSync(join(tmpdir(), "sarv-onboarding-bank-"));
  const server = spawn(resolve("../target/debug/examples/desktop_qa"), [], {
    cwd: resolve(".."),
    env: {
      ...process.env,
      SARV_QA_DATA_DIR: profile,
      SARV_QA_TOKEN: token,
      SARV_QA_SEED: "0",
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
    await page.goto("/");
    await page.getByRole("button", { name: "Get started" }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Company name", { exact: true }).fill("Smile Dental");
    await dialog.getByLabel("Company name", { exact: true }).press("Enter");
    for (let i = 0; i < 2; i++)
      await dialog.getByRole("button", { name: "Continue", exact: true }).click();
    await dialog.getByRole("button", { name: "Create company", exact: true }).click();

    await expect(dialog.getByText("Where is your money today?")).toBeVisible();
    await dialog.getByLabel("Main bank account name").fill("Mellat business, main");
    await dialog.getByLabel("Account or card number (optional)").fill("6104337712345678");
    await dialog.getByLabel(/Bank balance/).fill("125000000");
    await dialog.getByLabel(/Cash on hand/).fill("3000000");
    // Enter on the last field records the opening, like submitting a form.
    await dialog.getByLabel(/Cash on hand/).press("Enter");
    await expect(dialog).not.toBeVisible();

    const companyId = (await api("workspace")).companies[0].id;
    const ws = await api("workspace", { company_id: companyId });
    expect(ws.initialized).toBe(true);
    expect(ws.business_profile.bank_details).toContain("Mellat business, main");
    expect(ws.business_profile.bank_details).toContain("6104337712345678");
  } finally {
    if (server.exitCode === null && server.signalCode === null) {
      server.kill("SIGTERM");
      await once(server, "exit");
    }
    rmSync(profile, { recursive: true, force: true });
  }
});

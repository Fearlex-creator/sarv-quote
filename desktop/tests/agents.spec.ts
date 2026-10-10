import { test, expect } from "@playwright/test";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
const out = "../artifacts/tauri-ui";
mkdirSync(out, { recursive: true });

test("minimal settings grants local CLI and MCP access and revokes it", async ({
  page,
  context,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await request.post("http://127.0.0.1:1421", {
    headers: {
      "x-sarv-qa-token":
        process.env.SARV_QA_TOKEN ?? "sarv-isolated-browser-qa-token-2026",
    },
    data: {
      action: "agent_configure",
      payload: { enabled: false, access: "read_only" },
    },
  });
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/");
  await page
    .locator(".sidebar")
    .getByRole("button", { name: "Settings", exact: true })
    .click();
  const general = page.getByRole("tab", { name: "General", exact: true });
  await general.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "Agents", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  const access = page.getByRole("switch", {
    name: "Agent access",
    exact: true,
  });
  await expect(access).not.toBeChecked();
  await expect(
    page.getByRole("button", { name: "Copy CLI", exact: true }),
  ).toBeDisabled();
  await expect(page.locator(".page-heading p")).toHaveCount(0);
  await expect(
    page.locator(".agents-panel p, .agents-panel small"),
  ).toHaveCount(0);
  await page.screenshot({ path: `${out}/agents-off.png` });
  await page.getByLabel("Access", { exact: true }).selectOption("full");
  await expect(access).toBeEnabled();
  await access.click();
  await expect(access).toBeChecked();
  await expect(
    page.getByRole("button", { name: "Copy MCP", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Copy CLI", exact: true }).click();
  const copiedCLI = await page.evaluate(() => navigator.clipboard.readText());
  expect(copiedCLI).toContain("sarv-agent");
  expect(copiedCLI).toContain("--connection");
  await page.getByRole("button", { name: "Copy MCP", exact: true }).click();
  const config = JSON.parse(
    await page.evaluate(() => navigator.clipboard.readText()),
  ).mcpServers.sarv as { command: string; args: string[] };
  expect(config.args.at(-1)).toBe("mcp");
  const base = config.args.slice(0, -1);
  const cli = (action: string, payload: unknown = {}) =>
    JSON.parse(
      execFileSync(config.command, [...base, "call", action, "--stdin"], {
        input: JSON.stringify(payload),
        encoding: "utf8",
      }),
    );
  const name = `Agent browser ${Date.now()}`;
  const company = cli("setup_company", {
    name,
    fiscal_name: "2026",
    start_date: "2026-01-01",
    end_date: "2026-12-31",
  });
  cli("initialize_zero", {
    company_id: company.company_id,
    source_id: `${name}-opening`,
    request_key: `${name}-opening`,
    confirmed: true,
  });
  await page
    .getByRole("button", { name: "Refresh books", exact: true })
    .click();
  await expect(
    page.getByLabel("Organization").getByRole("option", { name, exact: true }),
  ).toHaveCount(1);
  // Creating the first company selects it and remounts the settings page.
  await expect(page.locator(".loading")).toHaveCount(0);
  await page.getByRole("tab", { name: "Agents", exact: true }).click();
  const mcp = spawnSync(config.command, config.args, {
    encoding: "utf8",
    input:
      [
        JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "initialize",
          params: {
            protocolVersion: "2024-11-05",
            capabilities: {},
            clientInfo: { name: "browser-agent", version: "1" },
          },
        }),
        JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }),
        JSON.stringify({
          jsonrpc: "2.0",
          id: 2,
          method: "tools/call",
          params: {
            name: "sarv_workspace",
            arguments: { company_id: company.company_id },
          },
        }),
      ].join("\n") + "\n",
  });
  expect(mcp.status).toBe(0);
  const responses = mcp.stdout
    .trim()
    .split("\n")
    .map((v) => JSON.parse(v));
  expect(responses).toHaveLength(2);
  expect(JSON.parse(responses[1].result.content[0].text).initialized).toBe(
    true,
  );
  await page.screenshot({ path: `${out}/agents-on.png` });
  await page.getByLabel("Access", { exact: true }).selectOption("read_only");
  await expect(page.getByLabel("Access", { exact: true })).toBeEnabled();
  const readonly = spawnSync(
    config.command,
    [...base, "call", "setup_company", "--json", "{}"],
    { encoding: "utf8" },
  );
  expect(readonly.status).toBe(1);
  expect(JSON.parse(readonly.stdout).error.message).toContain("Read-only");
  await page.getByRole("button", { name: "Revoke", exact: true }).click();
  await expect(access).not.toBeChecked();
  const revoked = spawnSync(config.command, [...base, "status"], {
    encoding: "utf8",
  });
  expect(revoked.status).toBe(1);
  await page.screenshot({ path: `${out}/agents-revoked.png` });
  expect(errors).toEqual([]);
});

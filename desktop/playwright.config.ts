import { defineConfig } from "@playwright/test";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Browser to drive: explicit override, then the first installed Chromium-family browser.
const browser =
  process.env.PLAYWRIGHT_CHROMIUM ??
  [
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/google-chrome",
  ].find(existsSync);

// Shared token for the long-running QA adapter on :1421 (specs read the same variable).
const qaToken =
  process.env.SARV_QA_TOKEN ?? "sarv-isolated-browser-qa-token-2026";
const qaProfile = join(tmpdir(), "sarv-playwright-qa-profile");

export default defineConfig({
  testDir: "tests",
  workers: 1,
  timeout: 90000,
  webServer: [
    {
      // Real Rust accounting adapter, seeded with the synthetic fixture, on a fresh profile.
      // Build it first: `pnpm run build:qa` (the `test` script does).
      // Run from the repository root: the agent service finds `target/release/sarv-agent` relative
      // to the working directory, exactly like the adapters the specs spawn themselves.
      command: `rm -rf "${qaProfile}" && mkdir -p "${qaProfile}" && exec target/debug/examples/desktop_qa`,
      cwd: "..",
      port: 1421,
      reuseExistingServer: false,
      env: {
        SARV_QA_DATA_DIR: qaProfile,
        SARV_QA_TOKEN: qaToken,
        SARV_QA_SEED: "1",
        SARV_QA_PORT: "1421",
      },
    },
    {
      // The app only talks to the adapter when the dev server has these two variables.
      command: "pnpm exec vite --host 127.0.0.1 --port 1420",
      url: "http://localhost:1420",
      reuseExistingServer: false,
      env: { VITE_QA_URL: "http://127.0.0.1:1421", VITE_QA_TOKEN: qaToken },
    },
  ],
  use: {
    baseURL: "http://localhost:1420",
    viewport: { width: 1440, height: 900 },
    launchOptions: { executablePath: browser, args: ["--no-sandbox"] },
    trace: "retain-on-failure",
  },
  reporter: [
    ["list"],
    ["json", { outputFile: "../artifacts/tauri-ui/results.json" }],
  ],
  outputDir: "../artifacts/tauri-ui/test-results",
});

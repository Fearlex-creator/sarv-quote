import { defineConfig } from "@playwright/test";
import { existsSync } from "node:fs";

// Browser to drive: explicit override, then the first installed Chromium-family browser.
const browser =
  process.env.PLAYWRIGHT_CHROMIUM ??
  [
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/google-chrome",
  ].find(existsSync);

export default defineConfig({
  testDir: "tests",
  workers: 1,
  timeout: 90000,
  // The app only talks to the Rust QA adapter when the dev server is started with these two
  // variables. Each spec spawns its own adapter and overrides the token per request.
  webServer: {
    command: "pnpm exec vite --host 127.0.0.1 --port 1420",
    url: "http://localhost:1420",
    reuseExistingServer: false,
    env: {
      VITE_QA_URL: "http://127.0.0.1:1421",
      VITE_QA_TOKEN: "playwright-qa-token-placeholder-2026",
    },
  },
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

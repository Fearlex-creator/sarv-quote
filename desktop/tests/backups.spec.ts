import { test, expect } from "@playwright/test";

test("settings backups create and list, and refuse restore without a shop password", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(() =>
    localStorage.setItem("sarv.language", "en"),
  );
  await page.goto("/");
  await page
    .locator(".sidebar")
    .getByRole("button", { name: "Settings", exact: true })
    .click();
  const tabs = page.getByRole("tab", { name: "Backups", exact: true });
  await tabs.focus();
  await page.keyboard.press("Enter");
  await expect(tabs).toHaveAttribute("aria-selected", "true");
  // No backups on a fresh profile beyond migration artifacts.
  await expect(
    page.getByRole("button", { name: "Create backup", exact: true }),
  ).toBeEnabled();
  await page.getByRole("button", { name: "Create backup", exact: true }).click();
  await expect(page.getByText(/Backup created:/)).toBeVisible();
  // The new backup appears with its contents summary.
  const row = page.locator("table tbody tr").first();
  await expect(row).toBeVisible();
  await expect(row).toContainText("Journals");
  // Restoring a backup taken just now is a safe no-op with an explicit review.
  await row.getByRole("button", { name: "Restore…", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(
    dialog.getByRole("heading", { name: "Restore books from backup?" }),
  ).toBeVisible();
  await expect(
    dialog.getByRole("heading", { name: "Backup contains" }),
  ).toBeVisible();
  await expect(
    dialog.getByRole("heading", { name: "Current books" }),
  ).toBeVisible();
  await page.screenshot({ path: "../artifacts/tauri-ui/backup-restore-review.png" });
  await dialog
    .getByRole("button", { name: "Restore backup", exact: true })
    .click();
  // The shared browser profile is the unlocked QA adapter. Restore stays refused
  // until that profile has its own shop password, so this click must not import
  // the books. A packaged profile with its own password is covered separately.
  await expect(dialog.getByRole("alert")).toContainText(
    "Set a shop password before restoring a backup into this profile",
  );
  await expect(page.getByText("Books restored from backup")).toHaveCount(0);
  expect(errors).toEqual([]);
});

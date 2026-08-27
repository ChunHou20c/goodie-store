import { test, expect } from "@playwright/test";

// No server needed: proves the browser binary itself launches and runs.
test("browser launches and renders a page", async ({ page }) => {
  await page.setContent("<title>smoke</title><h1>hello</h1>");
  await expect(page).toHaveTitle("smoke");
  await expect(page.locator("h1")).toHaveText("hello");
});

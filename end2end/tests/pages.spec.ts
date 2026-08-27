import { test, expect } from "@playwright/test";

// "Does it render" and nothing more: each route answers 200 signed out. No
// markup assertions — the screens are still being built, and any heading
// pinned here would only need rewriting when they change.

const routes = [
  "/",
  "/search",
  "/bag",
  "/orders",
  "/checkout",
  "/login",
  "/admin",
];

for (const path of routes) {
  test(`${path} renders`, async ({ page }) => {
    const response = await page.goto(path);
    expect(response?.status()).toBe(200);
  });
}

// The router's fallback screen still renders, but the response carries 404.
test("/not-a-real-page renders", async ({ page }) => {
  const response = await page.goto("/not-a-real-page");
  expect(response?.status()).toBe(404);
});

// Product URLs carry a slug from the database, so take one off the shelf
// rather than hard-coding a path that reseeding would invalidate.
test("/p/:slug renders", async ({ page }) => {
  await page.goto("/");

  const href = await page.locator('a[href^="/p/"]').first().getAttribute("href");
  expect(href).toBeTruthy();

  const response = await page.goto(href!);
  expect(response?.status()).toBe(200);
});

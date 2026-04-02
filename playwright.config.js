import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/browser",
  testMatch: /.*\.spec\.js/,
  timeout: 30_000,
  fullyParallel: true,
  retries: 0,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "on-first-retry",
    ...(process.env.CI ? {} : { channel: "msedge" })
  },
  webServer: {
    command: "node tests/browser/server.mjs",
    port: 4173,
    reuseExistingServer: true
  }
});

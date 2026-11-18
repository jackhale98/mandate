import { defineConfig, devices } from "@playwright/test";

// The site base (DEC-064). The landing page is served there and the game at
// play/ (DEC-083); app tests navigate relative to the game, the landing-page
// test to the site root.
const base = process.env.MANDATE_BASE ?? "/mandate/";
const executablePath = process.env.PW_CHROMIUM_PATH;

export default defineConfig({
  testDir: "e2e",
  timeout: 60_000,
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL: `http://localhost:4173${base}play/`,
    trace: "off",
    ...(executablePath ? { launchOptions: { executablePath } } : {}),
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] } },
    { name: "phone", use: { ...devices["Pixel 7"] } },
  ],
  webServer: {
    // Serves the whole site: the landing page, play/, and spike-hub/ when built.
    command: "npx vite preview -c landing/vite.config.ts --port 4173 --strictPort",
    url: `http://localhost:4173${base}play/`,
    reuseExistingServer: !process.env.CI,
  },
});

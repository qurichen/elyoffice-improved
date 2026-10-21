import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  use: { baseURL: "http://localhost:47311", viewport: { width: 1280, height: 1000 } },
  webServer: { command: "pnpm build && pnpm preview --port 47311 --strictPort", port: 47311 },
});

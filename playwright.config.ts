import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/e2e',
  workers: 1,
  use: { channel: 'chrome', headless: true, viewport: { width: 1120, height: 760 } },
  webServer: [
    { command: 'npm run dev:fixture', url: 'http://127.0.0.1:1420', reuseExistingServer: false },
    { command: 'npx vite preview --host 127.0.0.1 --port 1421 --strictPort', url: 'http://127.0.0.1:1421', reuseExistingServer: false },
  ],
});

import { defineConfig } from "@playwright/test";

// Rust 版（axum + Leptos）を起動して E2E を実行する設定。
// 使い方: cd e2e && npx playwright test
export default defineConfig({
  testDir: "./tests",
  timeout: 30000,
  use: {
    baseURL: "http://127.0.0.1:5173",
    headless: true,
    // ブラウザのバイナリを差し替えたい場合（Playwright 版とキャッシュ済みブラウザの版が違うとき等）
    launchOptions: process.env.PW_CHROMIUM_PATH
      ? { executablePath: process.env.PW_CHROMIUM_PATH }
      : {},
  },
  webServer: [
    {
      command:
        "cd .. && cargo run -p meetup-backend",
      port: 3000,
      reuseExistingServer: true,
      // 初回は cargo のビルドに時間がかかる
      timeout: 600000,
      env: {
        DATABASE_URL: "file:./e2e.db",
        JWT_SECRET: "dev-secret",
        RATE_LIMIT_DISABLED: "true",
      },
    },
    {
      command: "cd ../frontend && trunk serve",
      port: 5173,
      reuseExistingServer: true,
      timeout: 600000,
    },
  ],
});

import { defineConfig, devices } from '@playwright/test';

/**
 * BASE_URL が渡されていれば、そこで動いているアプリを相手にする (compose の e2e サービス)。
 * 渡されていなければ Playwright が `dotnet run` でサーバを起動する (ローカル開発)。
 */
const externalBaseUrl = process.env.BASE_URL;
const baseURL = externalBaseUrl ?? 'http://127.0.0.1:5210';

export default defineConfig({
  testDir: './tests',

  // 倒立を一定時間維持できるかを見るテストがあるため、既定より長めに取る
  timeout: 120_000,
  expect: { timeout: 30_000 },

  /*
   * プラントは実時間で動く。テストを並列にすると CPU の奪い合いが制御ループの
   * 追従を乱し、「遅延のせいで倒れた」のか「テスト環境が遅かった」のか区別できなくなる。
   * そのため必ず 1 ワーカーで直列に流す。
   */
  workers: 1,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,

  reporter: process.env.CI
    ? [['list'], ['html', { open: 'never' }], ['junit', { outputFile: 'test-results/junit.xml' }]]
    : [['list'], ['html', { open: 'never' }]],

  use: {
    baseURL,
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    viewport: { width: 1280, height: 1400 },
    locale: 'ja-JP',
  },

  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],

  webServer: externalBaseUrl
    ? undefined
    : {
        command: 'dotnet run --project ../src/Ip.Server --no-launch-profile -c Release',
        url: 'http://127.0.0.1:5210/healthz',
        reuseExistingServer: !process.env.CI,
        timeout: 240_000,
        stdout: 'ignore',
        stderr: 'pipe',
        env: { ASPNETCORE_URLS: 'http://127.0.0.1:5210' },
      },
});

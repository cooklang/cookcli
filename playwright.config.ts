import { defineConfig, devices } from '@playwright/test';
import path from 'node:path';

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// import dotenv from 'dotenv';
// dotenv.config({ path: path.resolve(__dirname, '.env') });

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: './tests/e2e',
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: [
    ['html'],
    ['list'],
    ['json', { outputFile: 'test-results/results.json' }]
  ],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: 'http://localhost:9080',

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',

    /* Take screenshots on failure */
    screenshot: 'only-on-failure',

    /* Record video on failure */
    video: 'retain-on-failure',
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Increase timeout for slower operations
        actionTimeout: 10000,
      },
    },

    /* Other browsers disabled for now to speed up CI */
    // {
    //   name: 'firefox',
    //   use: {
    //     ...devices['Desktop Firefox'],
    //     actionTimeout: 10000,
    //   },
    // },

    // {
    //   name: 'webkit',
    //   use: {
    //     ...devices['Desktop Safari'],
    //     actionTimeout: 10000,
    //   },
    // },

    /* Test against mobile viewports. */
    // {
    //   name: 'Mobile Chrome',
    //   use: {
    //     ...devices['Pixel 5'],
    //     actionTimeout: 10000,
    //   },
    // },
    // {
    //   name: 'Mobile Safari',
    //   use: {
    //     ...devices['iPhone 12'],
    //     actionTimeout: 10000,
    //   },
    // },

    /* Test against branded browsers. */
    // {
    //   name: 'Microsoft Edge',
    //   use: { ...devices['Desktop Edge'], channel: 'msedge' },
    // },
    // {
    //   name: 'Google Chrome',
    //   use: { ...devices['Desktop Chrome'], channel: 'chrome' },
    // },
  ],

  /* Run your local dev server before starting the tests */
  webServer: {
    command: process.env.CI
      ? './target/debug/cook server ./seed --port 9080'  // In CI, use pre-built binary
      : 'npm run build-css && npm run build-js && cargo build && ./target/debug/cook server ./seed --port 9080',  // Local dev
    url: 'http://localhost:9080',
    env: {
      // Keep the server under test away from the developer's real CookCloud
      // session. Without this, `npm test` on a machine where someone has run
      // `cook login` starts syncing ./seed to their account, against the same
      // sync.db that tracks their own recipe folder. `test-results/` is
      // gitignored. See tests/config_dir_env_test.rs.
      COOK_CONFIG_DIR: path.join(__dirname, 'test-results', 'cook-config'),
    },
    reuseExistingServer: !process.env.CI,
    timeout: 60 * 1000, // 1 minute should be enough with pre-built binary
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
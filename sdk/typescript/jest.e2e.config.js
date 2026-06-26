/** @type {import('jest').Config} */
module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  testMatch: ["**/e2e/**/*.e2e.test.ts"],
  // E2e tests hit a real node — give each test file plenty of time.
  testTimeout: 120_000,
  globals: {
    "ts-jest": {
      tsconfig: {
        // Allow dynamic import() inside tests.
        module: "commonjs",
        esModuleInterop: true,
      },
    },
  },
};

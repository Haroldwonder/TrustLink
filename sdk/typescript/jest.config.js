/** @type {import('jest').Config} */
export default {
  preset: "ts-jest/presets/default-esm",
  testEnvironment: "node",
  extensionsToTreatAsEsm: [".ts"],
  moduleNameMapper: {
    "^(\\.{1,2}/.*)\\.js$": "$1",
  },
  transform: {
    "^.+\\.tsx?$": [
      "ts-jest",
      {
        useESM: true,
        tsconfig: { module: "node16", moduleResolution: "node16", target: "es2020", esModuleInterop: true },
      },
    ],
  },
  testMatch: ["**/__tests__/**/*.test.ts"],
  testPathIgnorePatterns: ["/node_modules/", "/e2e/"],
};

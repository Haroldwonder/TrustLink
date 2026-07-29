import {
  getInfoSampleRate,
  logger,
  logProcessedEvent,
  resetRandomFn,
  setRandomFn,
  shouldSampleInfo,
} from "./logger";

describe("info-level log sampling", () => {
  const originalSampleRate = process.env.LOG_INFO_SAMPLE_RATE;

  afterEach(() => {
    resetRandomFn();
    if (originalSampleRate === undefined) {
      delete process.env.LOG_INFO_SAMPLE_RATE;
    } else {
      process.env.LOG_INFO_SAMPLE_RATE = originalSampleRate;
    }
  });

  it("defaults to 100% sampling", () => {
    delete process.env.LOG_INFO_SAMPLE_RATE;
    expect(getInfoSampleRate()).toBe(1);
    expect(shouldSampleInfo()).toBe(true);
  });

  it("samples info logs below 100%", () => {
    process.env.LOG_INFO_SAMPLE_RATE = "0.1";
    setRandomFn(() => 0.5);
    expect(shouldSampleInfo()).toBe(false);

    setRandomFn(() => 0.05);
    expect(shouldSampleInfo()).toBe(true);
  });

  it("reduces emitted per-event info logs under synthetic high throughput", () => {
    process.env.LOG_INFO_SAMPLE_RATE = "0.1";
    let randomValue = 0;
    setRandomFn(() => {
      randomValue = (randomValue + 0.11) % 1;
      return randomValue;
    });

    const infoSpy = jest.spyOn(logger, "info").mockImplementation(() => logger);
    const errorSpy = jest.spyOn(logger, "error").mockImplementation(() => logger);
    let infoCount = 0;

    for (let index = 0; index < 1_000; index += 1) {
      logProcessedEvent({ eventType: "created", ledger: index });
      logger.error({ ledger: index }, "synthetic failure");
    }

    infoCount = infoSpy.mock.calls.length;
    expect(infoCount).toBeGreaterThan(0);
    expect(infoCount).toBeLessThan(1_000);
    expect(errorSpy).toHaveBeenCalledTimes(1_000);

    infoSpy.mockRestore();
    errorSpy.mockRestore();
  });
});

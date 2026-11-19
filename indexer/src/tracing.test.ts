/**
 * #1139 – Smoke tests for indexer/src/tracing.ts (OpenTelemetry setup)
 *
 * Verifies that:
 *  - tracing initialisation is opt-in via OTEL_EXPORTER_OTLP_ENDPOINT and is a
 *    no-op (no SDK constructed) when the endpoint is unset
 *  - when the endpoint is set, the NodeSDK is constructed with an OTLP trace
 *    exporter pointed at "<endpoint>/v1/traces" and started, without throwing
 *  - shutdownTracing is safe whether or not the SDK was started
 *  - getTracer() returns a working tracer and a span can be created/ended for
 *    a sample operation
 *
 * The heavy @opentelemetry/sdk-node bits are mocked so the test is hermetic
 * and fast; @opentelemetry/api is left real so span creation is exercised
 * for real. Modules are reset between tests so tracing.ts's internal `sdk`
 * handle never leaks across cases.
 */

jest.mock("@opentelemetry/sdk-node", () => {
  const start = jest.fn();
  const shutdown = jest.fn().mockResolvedValue(undefined);
  const NodeSDK = jest.fn().mockImplementation((config: unknown) => ({
    start,
    shutdown,
    config,
  }));
  (NodeSDK as unknown as { __start: jest.Mock; __shutdown: jest.Mock }).__start = start;
  (NodeSDK as unknown as { __start: jest.Mock; __shutdown: jest.Mock }).__shutdown = shutdown;
  return { NodeSDK };
});

jest.mock("@opentelemetry/exporter-trace-otlp-http", () => ({
  OTLPTraceExporter: jest.fn().mockImplementation((opts: unknown) => ({ opts })),
}));

jest.mock("@opentelemetry/resources", () => ({
  Resource: jest.fn().mockImplementation((attrs: unknown) => ({ attrs })),
}));

jest.mock("@opentelemetry/auto-instrumentations-node", () => ({
  getNodeAutoInstrumentations: jest.fn().mockReturnValue([]),
}));

type TracingModule = typeof import("./tracing");

const ORIGINAL_ENDPOINT = process.env.OTEL_EXPORTER_OTLP_ENDPOINT;

let tracing: TracingModule;
let NodeSDKMock: jest.Mock & { __start: jest.Mock; __shutdown: jest.Mock };
let OTLPTraceExporterMock: jest.Mock;

beforeEach(() => {
  jest.resetModules();
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  tracing = require("./tracing");
  NodeSDKMock = require("@opentelemetry/sdk-node").NodeSDK;
  OTLPTraceExporterMock = require("@opentelemetry/exporter-trace-otlp-http").OTLPTraceExporter;
});

afterEach(() => {
  if (ORIGINAL_ENDPOINT === undefined) {
    delete process.env.OTEL_EXPORTER_OTLP_ENDPOINT;
  } else {
    process.env.OTEL_EXPORTER_OTLP_ENDPOINT = ORIGINAL_ENDPOINT;
  }
});

describe("getTracer", () => {
  it("returns a tracer that can start and end a span for a sample operation", () => {
    const tracer = tracing.getTracer();
    expect(typeof tracer.startSpan).toBe("function");

    const span = tracer.startSpan("sample-operation");
    expect(() => {
      span.setAttribute("sample.attr", "value");
      span.end();
    }).not.toThrow();
    expect(typeof span.spanContext().traceId).toBe("string");
  });

  it("returns a usable tracer on repeated calls", () => {
    expect(() => tracing.getTracer().startSpan("op").end()).not.toThrow();
    expect(() => tracing.getTracer().startSpan("op2").end()).not.toThrow();
  });
});

describe("initTracing", () => {
  it("is a no-op when OTEL_EXPORTER_OTLP_ENDPOINT is unset", () => {
    delete process.env.OTEL_EXPORTER_OTLP_ENDPOINT;
    expect(() => tracing.initTracing()).not.toThrow();
    expect(NodeSDKMock).not.toHaveBeenCalled();
  });

  it("initialises the NodeSDK without throwing when an endpoint is configured", () => {
    process.env.OTEL_EXPORTER_OTLP_ENDPOINT = "http://otel-collector:4318";
    expect(() => tracing.initTracing()).not.toThrow();

    expect(NodeSDKMock).toHaveBeenCalledTimes(1);
    expect(NodeSDKMock.__start).toHaveBeenCalledTimes(1);
    expect(OTLPTraceExporterMock).toHaveBeenCalledWith({
      url: "http://otel-collector:4318/v1/traces",
    });
  });
});

describe("shutdownTracing", () => {
  it("resolves without throwing when tracing was never started", async () => {
    delete process.env.OTEL_EXPORTER_OTLP_ENDPOINT;
    tracing.initTracing();
    await expect(tracing.shutdownTracing()).resolves.toBeUndefined();
    expect(NodeSDKMock.__shutdown).not.toHaveBeenCalled();
  });

  it("shuts the SDK down after it has been started", async () => {
    process.env.OTEL_EXPORTER_OTLP_ENDPOINT = "http://otel-collector:4318";
    tracing.initTracing();
    await expect(tracing.shutdownTracing()).resolves.toBeUndefined();
    expect(NodeSDKMock.__shutdown).toHaveBeenCalledTimes(1);
  });
});

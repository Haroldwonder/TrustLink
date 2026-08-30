/**
 * #1136 – Dedicated tests for the shared REST + GraphQL API-key auth module.
 *
 * auth.ts gates access to the indexer's REST and GraphQL surfaces, so its
 * authentication / authorization paths are exercised directly here:
 *   - key configuration parsing from GRAPHQL_API_KEYS
 *   - header extraction (string / array / missing)
 *   - the "auth disabled when no keys configured" fail-open behaviour
 *   - the Fastify `requireApiKey` guard and its 401 response
 */
import type { FastifyRequest } from "fastify";
import type { IncomingMessage } from "http";
import {
  extractApiKey,
  getConfiguredApiKeys,
  isApiKeyAuthorized,
  isAuthorized,
  requireApiKey,
} from "./auth";

const ORIGINAL_KEYS = process.env.GRAPHQL_API_KEYS;

afterEach(() => {
  if (ORIGINAL_KEYS === undefined) {
    delete process.env.GRAPHQL_API_KEYS;
  } else {
    process.env.GRAPHQL_API_KEYS = ORIGINAL_KEYS;
  }
});

describe("getConfiguredApiKeys", () => {
  it("returns an empty list when GRAPHQL_API_KEYS is unset", () => {
    delete process.env.GRAPHQL_API_KEYS;
    expect(getConfiguredApiKeys()).toEqual([]);
  });

  it("returns an empty list when GRAPHQL_API_KEYS is blank", () => {
    process.env.GRAPHQL_API_KEYS = "   ";
    expect(getConfiguredApiKeys()).toEqual([]);
  });

  it("splits a comma-separated list", () => {
    process.env.GRAPHQL_API_KEYS = "alpha,beta,gamma";
    expect(getConfiguredApiKeys()).toEqual(["alpha", "beta", "gamma"]);
  });

  it("trims surrounding whitespace and drops empty entries", () => {
    process.env.GRAPHQL_API_KEYS = " alpha , , beta ,,";
    expect(getConfiguredApiKeys()).toEqual(["alpha", "beta"]);
  });
});

describe("extractApiKey", () => {
  it("reads a plain string x-api-key header", () => {
    expect(extractApiKey({ "x-api-key": "secret" })).toBe("secret");
  });

  it("reads the first value when the header is repeated", () => {
    expect(extractApiKey({ "x-api-key": ["first", "second"] })).toBe("first");
  });

  it("returns undefined when the header is absent", () => {
    expect(extractApiKey({})).toBeUndefined();
  });
});

describe("isApiKeyAuthorized", () => {
  it("fails open when no keys are configured (auth disabled)", () => {
    delete process.env.GRAPHQL_API_KEYS;
    expect(isApiKeyAuthorized(undefined)).toBe(true);
    expect(isApiKeyAuthorized("anything")).toBe(true);
  });

  it("authorizes a key that is in the configured set", () => {
    process.env.GRAPHQL_API_KEYS = "alpha,beta";
    expect(isApiKeyAuthorized("beta")).toBe(true);
  });

  it("rejects a key that is not in the configured set", () => {
    process.env.GRAPHQL_API_KEYS = "alpha,beta";
    expect(isApiKeyAuthorized("gamma")).toBe(false);
  });

  it("rejects a missing key when keys are configured", () => {
    process.env.GRAPHQL_API_KEYS = "alpha";
    expect(isApiKeyAuthorized(undefined)).toBe(false);
    expect(isApiKeyAuthorized("")).toBe(false);
  });
});

describe("isAuthorized", () => {
  it("derives authorization from the request headers", () => {
    process.env.GRAPHQL_API_KEYS = "alpha";
    const ok = { headers: { "x-api-key": "alpha" } } as unknown as IncomingMessage;
    const bad = { headers: { "x-api-key": "nope" } } as unknown as IncomingMessage;
    expect(isAuthorized(ok)).toBe(true);
    expect(isAuthorized(bad)).toBe(false);
  });
});

describe("requireApiKey", () => {
  function makeReply() {
    const send = jest.fn();
    const code = jest.fn().mockReturnValue({ send });
    return { reply: { code } as unknown as Parameters<typeof requireApiKey>[1], code, send };
  }

  it("returns true and does not touch the reply when authorized", () => {
    process.env.GRAPHQL_API_KEYS = "alpha";
    const { reply, code } = makeReply();
    const req = { headers: { "x-api-key": "alpha" } } as unknown as FastifyRequest;
    expect(requireApiKey(req, reply)).toBe(true);
    expect(code).not.toHaveBeenCalled();
  });

  it("returns false and sends a 401 when unauthorized", () => {
    process.env.GRAPHQL_API_KEYS = "alpha";
    const { reply, code, send } = makeReply();
    const req = { headers: {} } as unknown as FastifyRequest;
    expect(requireApiKey(req, reply)).toBe(false);
    expect(code).toHaveBeenCalledWith(401);
    expect(send).toHaveBeenCalledWith({
      error: "Unauthorized: valid x-api-key header required",
    });
  });

  it("returns true when auth is disabled", () => {
    delete process.env.GRAPHQL_API_KEYS;
    const { reply, code } = makeReply();
    const req = { headers: {} } as unknown as FastifyRequest;
    expect(requireApiKey(req, reply)).toBe(true);
    expect(code).not.toHaveBeenCalled();
  });
});

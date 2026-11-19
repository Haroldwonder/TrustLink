import type { FastifyRequest } from "fastify";
import type { IncomingHttpHeaders, IncomingMessage } from "http";

/** Parse configured API keys from GRAPHQL_API_KEYS (shared REST + GraphQL auth). */
export function getConfiguredApiKeys(): string[] {
  const raw = process.env.GRAPHQL_API_KEYS ?? "";
  return raw
    .split(",")
    .map((key) => key.trim())
    .filter(Boolean);
}

export function extractApiKey(
  headers: IncomingHttpHeaders | IncomingMessage["headers"],
): string | undefined {
  const value = headers["x-api-key"];
  return Array.isArray(value) ? value[0] : value;
}

/** Returns true when auth is disabled (no keys configured) or the key matches. */
export function isApiKeyAuthorized(apiKey: string | undefined): boolean {
  const keys = getConfiguredApiKeys();
  if (keys.length === 0) {
    return true;
  }
  return !!apiKey && keys.includes(apiKey);
}

export function isAuthorized(req: IncomingMessage): boolean {
  return isApiKeyAuthorized(extractApiKey(req.headers));
}

export function requireApiKey(
  req: FastifyRequest,
  reply: { code: (status: number) => { send: (body: unknown) => unknown } },
): boolean {
  if (isApiKeyAuthorized(extractApiKey(req.headers))) {
    return true;
  }
  reply.code(401).send({ error: "Unauthorized: valid x-api-key header required" });
  return false;
}

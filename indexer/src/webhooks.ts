import { PrismaClient } from "@prisma/client";
import { createHmac } from "crypto";

export const MAX_ATTEMPTS = 5;

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

function sign(secret: string, body: string): string {
  return createHmac("sha256", secret).update(body).digest("hex");
}

export async function dispatchWebhooks(
  db: PrismaClient,
  eventType: string,
  payload: unknown
): Promise<void> {
  const webhooks = await db.webhook.findMany({ where: { active: true } });
  if (webhooks.length === 0) return;

  const body = JSON.stringify({ event: eventType, data: payload, ts: Date.now() });

  await Promise.allSettled(
    webhooks.map((wh) =>
      deliverWithRetry(db, wh.id, wh.url, wh.secret, eventType, body)
    )
  );
}

export async function deliverWithRetry(
  db: PrismaClient,
  webhookId: string,
  url: string,
  secret: string,
  eventType: string,
  body: string
): Promise<void> {
  const sig = sign(secret, body);
  let lastStatusCode: number | undefined;
  let lastError: string | undefined;

  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    try {
      const res = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-TrustLink-Signature": sig,
        },
        body,
        signal: AbortSignal.timeout(10_000),
      });

      if (res.ok) return;

      lastStatusCode = res.status;
      lastError = `HTTP ${res.status}`;

      // 4xx errors are not retried (client misconfiguration)
      if (res.status >= 400 && res.status < 500) {
        console.warn(`Webhook ${url} returned ${res.status} — not retrying`);
        await persistFailure(db, webhookId, url, eventType, body, res.status, lastError, attempt);
        return;
      }

      throw new Error(`HTTP ${res.status}`);
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err);
      if (attempt === MAX_ATTEMPTS) {
        console.error(`Webhook delivery to ${url} failed after ${MAX_ATTEMPTS} attempts:`, err);
        await persistFailure(db, webhookId, url, eventType, body, lastStatusCode, lastError, MAX_ATTEMPTS);
        return;
      }
      const delay = Math.min(200 * Math.pow(2, attempt - 1), 10_000);
      await sleep(delay);
    }
  }
}

async function persistFailure(
  db: PrismaClient,
  webhookId: string,
  url: string,
  eventType: string,
  payload: string,
  statusCode: number | undefined,
  errorMessage: string | undefined,
  attemptCount: number
): Promise<void> {
  try {
    await db.webhookFailure.create({
      data: {
        webhookId,
        url,
        eventType,
        payload,
        statusCode: statusCode ?? null,
        errorMessage: errorMessage ?? null,
        attemptCount,
        status: "FAILED",
      },
    });
  } catch (err) {
    // Never let persistence errors surface to the caller
    console.error("Failed to persist webhook failure record:", err);
  }
}

/** Replay a single failure record. Returns true on success. */
export async function replayFailure(
  db: PrismaClient,
  failureId: string
): Promise<{ success: boolean; statusCode?: number; error?: string }> {
  const failure = await db.webhookFailure.findUnique({ where: { id: failureId } });
  if (!failure) return { success: false, error: "Not found" };
  if (failure.status === "RECOVERED") return { success: true };
  if (failure.status === "RETRYING") return { success: false, error: "Retry already in progress" };

  // Mark as RETRYING to prevent concurrent retries
  await db.webhookFailure.update({
    where: { id: failureId },
    data: { status: "RETRYING" },
  });

  // Look up current secret for the webhook (may have been rotated)
  const webhook = failure.webhookId
    ? await db.webhook.findUnique({ where: { id: failure.webhookId } })
    : null;
  const secret = webhook?.secret ?? "";
  const sig = sign(secret, failure.payload);

  try {
    const res = await fetch(failure.url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-TrustLink-Signature": sig,
      },
      body: failure.payload,
      signal: AbortSignal.timeout(10_000),
    });

    if (res.ok) {
      await db.webhookFailure.delete({ where: { id: failureId } });
      return { success: true, statusCode: res.status };
    }

    await db.webhookFailure.update({
      where: { id: failureId },
      data: {
        status: "FAILED",
        statusCode: res.status,
        errorMessage: `HTTP ${res.status}`,
        attemptCount: { increment: 1 },
      },
    });
    return { success: false, statusCode: res.status, error: `HTTP ${res.status}` };
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    await db.webhookFailure.update({
      where: { id: failureId },
      data: {
        status: "FAILED",
        errorMessage,
        attemptCount: { increment: 1 },
      },
    });
    return { success: false, error: errorMessage };
  }
}

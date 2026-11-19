import { PrismaClient } from "@prisma/client";
import { createHmac } from "crypto";
import { webhookDeliveriesSuccessTotal, webhookDeliveriesFailureTotal } from "./metrics";
import { validateWebhookUrl } from "./webhook-validation";
import { decryptSecret, signWebhookPayload } from "./webhook-secret";

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
      deliverWithRetry(db, wh.id, wh.url, wh.secret_encrypted, wh.next_secret_encrypted ?? undefined, eventType, body)
    )
  );
}

export async function deliverWithRetry(
  db: PrismaClient,
  webhookId: string,
  url: string,
  secretEncrypted: string,
  nextSecretEncrypted: string | undefined,
  eventType: string,
  body: string
): Promise<void> {
  // Decrypt the secret
  let secret: string;
  try {
    secret = decryptSecret(secretEncrypted);
  } catch (err) {
    const errorMessage = `Failed to decrypt webhook secret: ${err instanceof Error ? err.message : "unknown"}`;
    console.error(`Webhook ${webhookId} decryption failed:`, err);
    webhookDeliveriesFailureTotal.inc();
    await persistFailure(db, webhookId, url, eventType, body, 500, errorMessage, 1);
    return;
  }

  // Validate URL at delivery time to catch DNS rebinding attacks
  try {
    await validateWebhookUrl(url);
  } catch (err) {
    const errorMessage = `webhook url validation failed: ${err instanceof Error ? err.message : "unknown"}`;
    console.error(`Webhook ${url} failed validation:`, err);
    webhookDeliveriesFailureTotal.inc();
    await persistFailure(db, webhookId, url, eventType, body, 400, errorMessage, 1);
    return;
  }

  const sig = sign(secret, body);
  // During grace period, also compute next signature for dual-signing
  let nextSig: string | undefined;
  if (nextSecretEncrypted) {
    try {
      const nextSecret = decryptSecret(nextSecretEncrypted);
      nextSig = sign(nextSecret, body);
    } catch (err) {
      console.warn(`Failed to decrypt next secret for dual-signing:`, err);
      // Continue without next signature
    }
  }

  let lastStatusCode: number | undefined;
  let lastError: string | undefined;

  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    try {
      const headers: Record<string, string> = {
        "Content-Type": "application/json",
        "X-TrustLink-Signature": sig,
      };
      // Include next signature during grace period
      if (nextSig) {
        headers["X-TrustLink-Signature-Next"] = nextSig;
      }

      const res = await fetch(url, {
        method: "POST",
        headers,
        body,
        signal: AbortSignal.timeout(10_000),
        redirect: "manual",
      });

      if (res.ok) {
        webhookDeliveriesSuccessTotal.inc();
        return;
      }

      // Check for redirect to private address
      if (res.status >= 300 && res.status < 400) {
        const location = res.headers.get("location");
        if (location) {
          try {
            await validateWebhookUrl(location);
          } catch (err) {
            const errorMessage = `redirect to invalid url: ${location}`;
            console.warn(`Webhook ${url} redirected to blocked location: ${location}`);
            webhookDeliveriesFailureTotal.inc();
            await persistFailure(db, webhookId, url, eventType, body, res.status, errorMessage, attempt);
            return;
          }
        }
      }

      lastStatusCode = res.status;
      lastError = `HTTP ${res.status}`;

      // 4xx errors are not retried (client misconfiguration)
      if (res.status >= 400 && res.status < 500) {
        console.warn(`Webhook ${url} returned ${res.status} — not retrying`);
        webhookDeliveriesFailureTotal.inc();
        await persistFailure(db, webhookId, url, eventType, body, res.status, lastError, attempt);
        return;
      }

      throw new Error(`HTTP ${res.status}`);
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err);
      if (attempt === MAX_ATTEMPTS) {
        console.error(`Webhook delivery to ${url} failed after ${MAX_ATTEMPTS} attempts:`, err);
        webhookDeliveriesFailureTotal.inc();
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

// Stale RETRYING entries older than this threshold are treated as FAILED
const STALE_RETRY_THRESHOLD_MS = 5 * 60 * 1000; // 5 minutes

/**
 * Reclaim stale RETRYING entries that exceeded the threshold.
 * Call periodically or before listing failures.
 */
export async function reclaimStaleRetries(db: PrismaClient): Promise<number> {
  const threshold = new Date(Date.now() - STALE_RETRY_THRESHOLD_MS);
  const result = await db.webhookFailure.updateMany({
    where: {
      status: "RETRYING",
      updatedAt: { lt: threshold },
    },
    data: {
      status: "FAILED",
      errorMessage: "retry timed out - stale entry reclaimed",
    },
  });
  return result.count;
}

/** Replay a single failure record. Returns true on success. */
export async function replayFailure(
  db: PrismaClient,
  failureId: string
): Promise<{ success: boolean; statusCode?: number; error?: string }> {
  // First, reclaim any stale RETRYING entries to handle edge cases
  await reclaimStaleRetries(db);

  const failure = await db.webhookFailure.findUnique({ where: { id: failureId } });
  if (!failure) return { success: false, error: "Not found" };
  if (failure.status === "RECOVERED") return { success: true };
  if (failure.status === "RETRYING") return { success: false, error: "Retry already in progress" };

  // REFUSE if webhook was deleted or is inactive — don't sign with empty secret
  if (failure.webhookId) {
    const webhook = await db.webhook.findUnique({ where: { id: failure.webhookId } });
    if (!webhook || !webhook.active) {
      await db.webhookFailure.update({
        where: { id: failureId },
        data: {
          status: "FAILED",
          errorMessage: "webhook no longer exists or is inactive",
          attemptCount: { increment: 1 },
        },
      });
      return { success: false, error: "webhook no longer exists or is inactive" };
    }
  } else {
    // No webhookId means the webhook was deleted
    await db.webhookFailure.update({
      where: { id: failureId },
      data: {
        status: "FAILED",
        errorMessage: "webhook was deleted",
        attemptCount: { increment: 1 },
      },
    });
    return { success: false, error: "webhook was deleted" };
  }

  // Validate URL before retrying
  try {
    await validateWebhookUrl(failure.url);
  } catch (err) {
    await db.webhookFailure.update({
      where: { id: failureId },
      data: {
        status: "FAILED",
        errorMessage: `url validation failed: ${err instanceof Error ? err.message : "unknown"}`,
        attemptCount: { increment: 1 },
      },
    });
    return { success: false, error: "url validation failed" };
  }

  // Atomic update: only transition from FAILED to RETRYING if current status is FAILED
  // This prevents race conditions with concurrent replays
  const updated = await db.webhookFailure.updateMany({
    where: {
      id: failureId,
      status: "FAILED", // Only update if currently FAILED
    },
    data: {
      status: "RETRYING",
      updatedAt: new Date(),
    },
  });

  if (updated.count === 0) {
    // Either another process already claimed this retry, or status changed
    const current = await db.webhookFailure.findUnique({ where: { id: failureId } });
    if (current?.status === "RETRYING") {
      return { success: false, error: "Retry already in progress" };
    }
    return { success: false, error: "Failed to acquire retry lock" };
  }

  // Fetch the webhook again (it may have been rotated during the check)
  const webhook = failure.webhookId
    ? await db.webhook.findUnique({ where: { id: failure.webhookId } })
    : null;
  if (!webhook) {
    await db.webhookFailure.update({
      where: { id: failureId },
      data: {
        status: "FAILED",
        errorMessage: "webhook no longer exists",
        attemptCount: { increment: 1 },
      },
    });
    return { success: false, error: "webhook no longer exists" };
  }

  // Decrypt the secret
  let secret: string;
  try {
    secret = decryptSecret(webhook.secret_encrypted);
  } catch (err) {
    await db.webhookFailure.update({
      where: { id: failureId },
      data: {
        status: "FAILED",
        errorMessage: `Failed to decrypt webhook secret: ${err instanceof Error ? err.message : "unknown"}`,
        attemptCount: { increment: 1 },
      },
    });
    return { success: false, error: "secret decryption failed" };
  }

  const sig = sign(secret, failure.payload);
  
  // During grace period, also compute next signature for dual-signing
  let nextSig: string | undefined;
  if (webhook.next_secret_encrypted) {
    try {
      const nextSecret = decryptSecret(webhook.next_secret_encrypted);
      nextSig = sign(nextSecret, failure.payload);
    } catch (err) {
      console.warn(`Failed to decrypt next secret for dual-signing:`, err);
      // Continue without next signature
    }
  }

  try {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      "X-TrustLink-Signature": sig,
    };
    if (nextSig) {
      headers["X-TrustLink-Signature-Next"] = nextSig;
    }

    const res = await fetch(failure.url, {
      method: "POST",
      headers,
      body: failure.payload,
      signal: AbortSignal.timeout(10_000),
      redirect: "manual",
    });

    if (res.ok) {
      // Mark as RECOVERED instead of deleting to preserve audit trail
      await db.webhookFailure.update({
        where: { id: failureId },
        data: {
          status: "RECOVERED",
          resolvedAt: new Date(),
        },
      });
      return { success: true, statusCode: res.status };
    }

    // Check for redirect to private address
    if (res.status >= 300 && res.status < 400) {
      const location = res.headers.get("location");
      if (location) {
        try {
          await validateWebhookUrl(location);
        } catch (err) {
          await db.webhookFailure.update({
            where: { id: failureId },
            data: {
              status: "FAILED",
              errorMessage: `redirect blocked: ${location}`,
              attemptCount: { increment: 1 },
            },
          });
          return { success: false, error: "blocked redirect" };
        }
      }
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

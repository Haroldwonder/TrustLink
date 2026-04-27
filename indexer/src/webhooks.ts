import { PrismaClient } from "@prisma/client";
import { createHmac } from "crypto";

const MAX_ATTEMPTS = 5;

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
    webhooks.map((wh) => deliverWithRetry(wh.url, wh.secret, body))
  );
}

async function deliverWithRetry(
  url: string,
  secret: string,
  body: string
): Promise<void> {
  const sig = sign(secret, body);

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

      // 4xx errors are not retried (client misconfiguration)
      if (res.status >= 400 && res.status < 500) {
        console.warn(`Webhook ${url} returned ${res.status} — not retrying`);
        return;
      }

      throw new Error(`HTTP ${res.status}`);
    } catch (err) {
      if (attempt === MAX_ATTEMPTS) {
        console.error(`Webhook delivery to ${url} failed after ${MAX_ATTEMPTS} attempts:`, err);
        return;
      }
      const delay = Math.min(200 * Math.pow(2, attempt - 1), 10_000);
      await sleep(delay);
    }
  }
}

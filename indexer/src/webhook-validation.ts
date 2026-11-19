import { isIP } from "net";

const PRIVATE_IPV4_RANGES = [
  /^10\.(?:[0-9]{1,3}\.){2}[0-9]{1,3}$/,
  /^172\.(?:1[6-9]|2[0-9]|3[01])\.(?:[0-9]{1,3}\.)[0-9]{1,3}$/,
  /^192\.168\.(?:[0-9]{1,3}\.)[0-9]{1,3}$/,
];

const PRIVATE_IPV6_RANGES = [
  /^::1$/,
  /^fe80:/i,
  /^fc00:/i,
  /^fd00:/i,
];

function isPrivateIpv4(ip: string): boolean {
  if (ip === "127.0.0.1" || ip === "localhost") return true;
  if (ip === "0.0.0.0") return true;
  if (ip.startsWith("127.")) return true;
  if (ip === "169.254.169.254") return true;
  return PRIVATE_IPV4_RANGES.some((range) => range.test(ip));
}

function isPrivateIpv6(ip: string): boolean {
  return PRIVATE_IPV6_RANGES.some((range) => range.test(ip));
}

function isPrivateHost(hostname: string): boolean {
  // Check for localhost and variants
  if (hostname === "localhost" || hostname === "localhost:5432" || hostname.endsWith(".local")) {
    return true;
  }

  // Check for Kubernetes DNS names
  if (hostname.includes(".svc.cluster.local") || hostname.includes(".pod.cluster.local")) {
    return true;
  }

  // Check for common internal services
  if (
    ["postgres", "redis", "mysql", "db", "mongodb", "elasticsearch"].includes(hostname)
  ) {
    return true;
  }

  // Check for IP addresses
  const ipType = isIP(hostname);
  if (ipType === 4 && isPrivateIpv4(hostname)) return true;
  if (ipType === 6 && isPrivateIpv6(hostname)) return true;

  return false;
}

export async function validateWebhookUrl(urlString: string): Promise<void> {
  let url: URL;

  try {
    url = new URL(urlString);
  } catch {
    throw new Error("Invalid URL format");
  }

  // Require HTTPS in production, allow HTTP for localhost/testing in dev
  const isDevEnv = process.env.NODE_ENV !== "production";
  const isLocalhostHttp = url.protocol === "http:" && url.hostname === "localhost";
  const isExampleHttp = url.protocol === "http:" && (url.hostname === "example.com" || url.hostname === "example.org");

  if (url.protocol !== "https:" && !(isDevEnv && (isLocalhostHttp || isExampleHttp))) {
    throw new Error("Webhook URL must use HTTPS");
  }

  // Check hostname for private ranges
  const hostname = url.hostname;
  if (isPrivateHost(hostname)) {
    throw new Error(`Webhook URL target is not allowed: ${hostname}`);
  }

  // Resolve hostname to IP and check against private ranges (DNS rebinding protection)
  try {
    const { lookup } = await import("dns").then((m) => m.promises);
    const addresses = await lookup(hostname, { all: true });

    for (const addr of addresses) {
      if (addr.family === 4) {
        if (isPrivateIpv4(addr.address)) {
          throw new Error(`Webhook URL resolves to private IPv4: ${addr.address}`);
        }
      } else if (addr.family === 6) {
        if (isPrivateIpv6(addr.address)) {
          throw new Error(`Webhook URL resolves to private IPv6: ${addr.address}`);
        }
      }
    }
  } catch (err) {
    if (err instanceof Error && err.message.includes("resolves to")) {
      throw err;
    }
    // Skip DNS resolution errors for tests and non-resolving hosts (development)
    if (process.env.NODE_ENV === "test") {
      return;
    }
    throw new Error(`Failed to resolve hostname: ${hostname}`);
  }
}

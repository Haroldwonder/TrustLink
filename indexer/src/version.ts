/**
 * Build and deployment metadata for operator debugging.
 * Exposed via the /version REST endpoint so operators can identify
 * which indexer build and contract ID a running instance uses.
 */

export const buildInfo = {
  version: process.env.npm_package_version || "dev",
  buildTimestamp: process.env.BUILD_TIMESTAMP || new Date().toISOString(),
};

/**
 * Returns version metadata for the /version endpoint.
 * All values fall back to sensible defaults if not configured.
 */
export function getVersionInfo(): {
  version: string;
  buildTimestamp: string;
  contractId: string;
  network: string;
  nodeEnv: string;
  timestamp: string;
} {
  return {
    version: buildInfo.version,
    buildTimestamp: buildInfo.buildTimestamp,
    contractId: process.env.CONTRACT_ID || process.env.STELLAR_CONTRACT_ID || "not configured",
    network: process.env.NETWORK || process.env.STELLAR_NETWORK || "testnet",
    nodeEnv: process.env.NODE_ENV || "development",
    timestamp: new Date().toISOString(),
  };
}
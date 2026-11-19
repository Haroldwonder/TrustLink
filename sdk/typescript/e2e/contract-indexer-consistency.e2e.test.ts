/**
 * End-to-end consistency test: contract state vs indexer GraphQL for one ID.
 *
 * Creates a happy-path attestation on a local Soroban network, waits for the
 * indexer to ingest the corresponding event, then queries:
 *   1. the contract directly (`get_attestation`)
 *   2. the indexer's GraphQL API for the same attestation ID
 * and asserts field-for-field equality on the shared identity fields.
 *
 * Prerequisites (same as trustlink.e2e.test.ts / attestation-request-flow):
 *   - Local Stellar Quickstart running
 *   - Contract deployed (scripts/setup_local.sh)
 *   - Indexer running against the same CONTRACT_ID + Postgres
 *
 * Run:
 *   npm run test:e2e -- contract-indexer-consistency.e2e.test.ts
 *
 * Env:
 *   CONTRACT_ID / .local.contract-id
 *   INDEXER_GRAPHQL_URL (default http://localhost:4000/graphql)
 *   RPC_URL, NETWORK_PASSPHRASE, ADMIN_SECRET, ISSUER_SECRET (optional)
 */

import {
  Keypair,
  TransactionBuilder,
  BASE_FEE,
  Networks,
  rpc as SorobanRpc,
  Contract,
  Address,
  nativeToScVal,
} from "@stellar/stellar-sdk";
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";

// ── Configuration ───────────────────────────────────────────────────────────

const RPC_URL = process.env.RPC_URL ?? "http://localhost:8000/soroban/rpc";
const NETWORK_PASSPHRASE =
  process.env.NETWORK_PASSPHRASE ?? Networks.STANDALONE;
const INDEXER_GRAPHQL_URL =
  process.env.INDEXER_GRAPHQL_URL ?? "http://localhost:4000/graphql";
const INDEXER_POLL_ATTEMPTS = Number(process.env.INDEXER_POLL_ATTEMPTS ?? 45);
const INDEXER_POLL_MS = Number(process.env.INDEXER_POLL_MS ?? 2000);

function resolveContractId(): string {
  if (process.env.CONTRACT_ID) return process.env.CONTRACT_ID;
  const idFile = resolve(__dirname, "../../../../.local.contract-id");
  if (existsSync(idFile)) return readFileSync(idFile, "utf8").trim();
  throw new Error(
    "CONTRACT_ID env var not set and .local.contract-id not found. Run setup_local.sh.",
  );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });

async function fundAccount(keypair: Keypair): Promise<void> {
  const friendbotUrl = `http://localhost:8000/friendbot?addr=${keypair.publicKey()}`;
  const res = await fetch(friendbotUrl);
  if (!res.ok) throw new Error(`Friendbot failed: ${res.status}`);
}

async function invoke(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
  signer: Keypair,
): Promise<string> {
  const contract = new Contract(contractId);
  const account = await server.getAccount(signer.publicKey());

  const tx = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const simResult = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed for ${method}: ${simResult.error}`);
  }

  const prepared = SorobanRpc.assembleTransaction(
    tx,
    simResult as SorobanRpc.Api.SimulateTransactionSuccessResponse,
  ).build();

  prepared.sign(signer);
  const sendResult = await server.sendTransaction(prepared);
  if (sendResult.status === "ERROR") {
    throw new Error(
      `sendTransaction failed for ${method}: ${JSON.stringify(sendResult.errorResult)}`,
    );
  }

  const hash = sendResult.hash;
  for (let i = 0; i < 30; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    const status = await server.getTransaction(hash);
    if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) return hash;
    if (status.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`Transaction ${hash} failed for ${method}`);
    }
  }
  throw new Error(`Transaction ${hash} timed out for ${method}`);
}

async function simulate<T>(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
): Promise<T> {
  const contract = new Contract(contractId);
  const dummySource =
    "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN";
  const account = await server.getAccount(dummySource).catch(
    () =>
      ({
        accountId: () => dummySource,
        sequenceNumber: () => "0",
        incrementSequenceNumber: () => {},
      }) as never,
  );

  const tx = new TransactionBuilder(account as never, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const result = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(result)) {
    throw new Error(`Simulation failed for ${method}: ${result.error}`);
  }
  const success = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
  if (!success.result) throw new Error(`No result from ${method}`);

  const { scValToNative } = await import("@stellar/stellar-sdk");
  return scValToNative(success.result.retval) as T;
}

function addr(address: string) {
  return Address.fromString(address).toScVal();
}

function str(value: string) {
  return nativeToScVal(value, { type: "string" });
}

type GraphQLAttestation = {
  id: string;
  issuer: string;
  subject: string;
  claimType: string;
  timestamp: string;
  expiration: string | null;
  isRevoked: boolean;
  metadata: string | null;
};

async function queryIndexerAttestation(
  attestationId: string,
  subject: string,
  claimType: string,
): Promise<GraphQLAttestation | null> {
  const query = `
    query ConsistencyCheck($subject: String!, $claimType: String) {
      attestations(subject: $subject, claimType: $claimType, first: 50) {
        edges {
          node {
            id
            issuer
            subject
            claimType
            timestamp
            expiration
            isRevoked
            metadata
          }
        }
      }
    }
  `;

  const res = await fetch(INDEXER_GRAPHQL_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      query,
      variables: { subject, claimType },
    }),
  });

  if (!res.ok) {
    throw new Error(`GraphQL HTTP ${res.status}: ${await res.text()}`);
  }

  const body = (await res.json()) as {
    data?: {
      attestations?: { edges: Array<{ node: GraphQLAttestation }> };
    };
    errors?: unknown[];
  };

  if (body.errors?.length) {
    throw new Error(`GraphQL errors: ${JSON.stringify(body.errors)}`);
  }

  const edges = body.data?.attestations?.edges ?? [];
  return edges.map((e) => e.node).find((n) => n.id === attestationId) ?? null;
}

async function waitForIndexedAttestation(
  attestationId: string,
  subject: string,
  claimType: string,
): Promise<GraphQLAttestation> {
  let lastError: unknown;
  for (let i = 0; i < INDEXER_POLL_ATTEMPTS; i++) {
    try {
      const found = await queryIndexerAttestation(
        attestationId,
        subject,
        claimType,
      );
      if (found) return found;
    } catch (err) {
      lastError = err;
    }
    await new Promise((r) => setTimeout(r, INDEXER_POLL_MS));
  }
  throw new Error(
    `Indexer did not surface attestation ${attestationId} after ${INDEXER_POLL_ATTEMPTS} polls. Last error: ${lastError}`,
  );
}

// ── Test Suite ──────────────────────────────────────────────────────────────

describe("Contract ↔ indexer consistency (shared attestation ID)", () => {
  let contractId: string;
  let adminKeypair: Keypair;
  let issuerKeypair: Keypair;
  let subjectKeypair: Keypair;

  beforeAll(async () => {
    contractId = resolveContractId();
    adminKeypair = process.env.ADMIN_SECRET
      ? Keypair.fromSecret(process.env.ADMIN_SECRET)
      : Keypair.random();
    issuerKeypair = process.env.ISSUER_SECRET
      ? Keypair.fromSecret(process.env.ISSUER_SECRET)
      : Keypair.random();
    subjectKeypair = Keypair.random();

    await Promise.all([
      fundAccount(adminKeypair),
      fundAccount(issuerKeypair),
      fundAccount(subjectKeypair),
    ]);
  }, 60_000);

  it("matches live contract fields for a happy-path attestation ID", async () => {
    const claimType = "CONSISTENCY_CHECK";

    // Ensure issuer is registered (idempotent enough for local e2e).
    try {
      await invoke(
        contractId,
        "register_issuer",
        [addr(adminKeypair.publicKey()), addr(issuerKeypair.publicKey())],
        adminKeypair,
      );
    } catch {
      // Already registered from a prior run — continue.
    }

    await invoke(
      contractId,
      "create_attestation",
      [
        addr(issuerKeypair.publicKey()),
        addr(subjectKeypair.publicKey()),
        str(claimType),
        nativeToScVal(null),
        nativeToScVal(null),
      ],
      issuerKeypair,
    );

    const subjectAttestations: Array<{ id: string }> = await simulate(
      contractId,
      "get_subject_attestations",
      [
        addr(subjectKeypair.publicKey()),
        nativeToScVal(0, { type: "u32" }),
        nativeToScVal(10, { type: "u32" }),
      ],
    );
    expect(subjectAttestations.length).toBeGreaterThan(0);
    const attestationId = subjectAttestations[0].id;

    const onChain: {
      id: string;
      issuer: string;
      subject: string;
      claim_type: string;
      timestamp: number | bigint | string;
      expiration: number | bigint | string | null;
      revoked: boolean;
      metadata: string | null;
    } = await simulate(contractId, "get_attestation", [str(attestationId)]);

    expect(onChain.id).toBe(attestationId);

    const indexed = await waitForIndexedAttestation(
      attestationId,
      subjectKeypair.publicKey(),
      claimType,
    );

    // Field-for-field equality on the shared identity / status surface.
    expect(indexed.id).toBe(onChain.id);
    expect(indexed.issuer).toBe(String(onChain.issuer));
    expect(indexed.subject).toBe(String(onChain.subject));
    expect(indexed.claimType).toBe(onChain.claim_type);
    expect(indexed.timestamp).toBe(String(onChain.timestamp));
    expect(indexed.isRevoked).toBe(Boolean(onChain.revoked));
    expect(indexed.metadata ?? null).toBe(onChain.metadata ?? null);
    expect(indexed.expiration ?? null).toBe(
      onChain.expiration == null ? null : String(onChain.expiration),
    );
  }, 180_000);
});

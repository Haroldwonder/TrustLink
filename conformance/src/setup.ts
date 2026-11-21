import {
  Address,
  BASE_FEE,
  Contract,
  Keypair,
  nativeToScVal,
  Networks,
  rpc as SorobanRpc,
  TransactionBuilder,
} from "@stellar/stellar-sdk";
import { existsSync, readFileSync } from "fs";
import { resolve } from "path";
import type { ConformanceContext } from "./types";

const RPC_URL = process.env.RPC_URL ?? "http://localhost:8000/soroban/rpc";
const NETWORK_PASSPHRASE = process.env.NETWORK_PASSPHRASE ?? Networks.STANDALONE;

export function resolveContractId(): string {
  if (process.env.CONTRACT_ID) {
    return process.env.CONTRACT_ID;
  }
  const idFile = resolve(__dirname, "../../.local.contract-id");
  if (existsSync(idFile)) {
    return readFileSync(idFile, "utf8").trim();
  }
  throw new Error(
    "CONTRACT_ID not set and .local.contract-id not found. Run scripts/setup_local.sh first.",
  );
}

export async function isRpcAvailable(): Promise<boolean> {
  try {
    const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
    await server.getLatestLedger();
    return true;
  } catch {
    return false;
  }
}

async function fundAccount(keypair: Keypair): Promise<void> {
  const response = await fetch(`http://localhost:8000/friendbot?addr=${keypair.publicKey()}`);
  if (!response.ok) {
    throw new Error(`Friendbot failed: ${response.status}`);
  }
}

async function invoke(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
  signer: Keypair,
): Promise<void> {
  const server = new SorobanRpc.Server(RPC_URL, { allowHttp: true });
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
    throw new Error(`sendTransaction failed for ${method}`);
  }

  for (let attempt = 0; attempt < 30; attempt += 1) {
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 1_000));
    const status = await server.getTransaction(sendResult.hash);
    if (status.status === SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
      return;
    }
    if (status.status === SorobanRpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`Transaction failed for ${method}`);
    }
  }
  throw new Error(`Transaction timed out for ${method}`);
}

export async function prepareConformanceContext(): Promise<ConformanceContext> {
  const contractId = resolveContractId();
  const adminKeypair = Keypair.random();
  const issuerKeypair = Keypair.random();
  const subjectKeypair = Keypair.random();

  await Promise.all([
    fundAccount(adminKeypair),
    fundAccount(issuerKeypair),
    fundAccount(subjectKeypair),
  ]);

  await invoke(
    contractId,
    "register_issuer",
    [Address.fromString(adminKeypair.publicKey()).toScVal(), Address.fromString(issuerKeypair.publicKey()).toScVal()],
    adminKeypair,
  );

  await invoke(
    contractId,
    "create_attestation",
    [
      Address.fromString(issuerKeypair.publicKey()).toScVal(),
      Address.fromString(subjectKeypair.publicKey()).toScVal(),
      nativeToScVal("KYC_PASSED", { type: "string" }),
      nativeToScVal(null),
      nativeToScVal(null),
    ],
    issuerKeypair,
  );

  return {
    contractId,
    rpcUrl: RPC_URL,
    networkPassphrase: NETWORK_PASSPHRASE,
    subject: subjectKeypair.publicKey(),
    issuer: issuerKeypair.publicKey(),
  };
}
import type { ConformanceObservation } from "./types";

const ERROR_NAME_TO_CODE: Record<string, number> = {
  AlreadyInitialized: 1,
  NotInitialized: 2,
  Unauthorized: 3,
  NotFound: 4,
  DuplicateAttestation: 5,
  AlreadyRevoked: 6,
  Expired: 7,
};

export function normalizeAttestationIds(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.map((entry) => {
    if (typeof entry === "string") {
      return entry;
    }
    if (entry && typeof entry === "object" && "id" in entry) {
      return String((entry as { id: unknown }).id);
    }
    return String(entry);
  });
}

export function parseErrorObservation(step: string, error: unknown): ConformanceObservation {
  if (error && typeof error === "object") {
    if ("code" in error && typeof (error as { code: unknown }).code === "number") {
      const code = (error as { code: number }).code;
      const rawName =
        "name" in error && typeof (error as { name: unknown }).name === "string"
          ? (error as { name: string }).name
          : `ContractError(${code})`;
      const name = rawName.replace(/Error$/, "");
      return { step, kind: "error", code, name };
    }
  }

  const message = error instanceof Error ? error.message : String(error);
  const contractMatch = message.match(/Error\(Contract,\s*#(\d+)\)/);
  if (contractMatch) {
    const code = Number.parseInt(contractMatch[1], 10);
    return { step, kind: "error", code, name: `ContractError(${code})` };
  }

  const namedCode = ERROR_NAME_TO_CODE[message];
  if (namedCode !== undefined) {
    return { step, kind: "error", code: namedCode, name: message };
  }

  throw new Error(`Unable to normalize error for step ${step}: ${message}`);
}

export function assertObservationsMatch(
  baseline: ConformanceObservation[],
  actual: ConformanceObservation[],
  clientName: string,
): void {
  const baselineJson = JSON.stringify(baseline, null, 2);
  const actualJson = JSON.stringify(actual, null, 2);
  if (baselineJson !== actualJson) {
    throw new Error(
      `Conformance drift detected for ${clientName}.\nExpected:\n${baselineJson}\nActual:\n${actualJson}`,
    );
  }
}

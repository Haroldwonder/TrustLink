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

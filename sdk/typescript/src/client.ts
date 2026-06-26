import {
  Contract,
  Networks,
  rpc,
  TransactionBuilder,
  BASE_FEE,
  xdr,
} from "@stellar/stellar-sdk";
import type {
  Attestation,
  AttestationStatus,
  CouncilProposal,
  Council,
  ContractMetadata,
  FeeConfig,
  IssuerMetadata,
  MultiSigProposal,
  StorageLimits,
} from "./types.js";

export interface TrustLinkClientOptions {
  /** Stellar RPC server URL. */
  rpcUrl: string;
  /** TrustLink contract ID. */
  contractId: string;
  /** Stellar network passphrase. Defaults to Testnet. */
  networkPassphrase?: string;
}

/**
 * TrustLinkClient — typed wrapper around the TrustLink Soroban contract.
 *
 * All read methods use `simulateTransaction` so they require no signing or fees.
 */
export class TrustLinkClient {
  private readonly server: rpc.Server;
  private readonly contract: Contract;
  private readonly networkPassphrase: string;
  private readonly contractId: string;

  constructor(options: TrustLinkClientOptions) {
    this.rpcUrl = options.rpcUrl;
    this.contractId = options.contractId;
    this.networkPassphrase =
      options.networkPassphrase ?? Networks.TESTNET;
    this.server = new rpc.Server(options.rpcUrl, { allowHttp: true });
    this.contract = new Contract(options.contractId);
  }

  // ---------------------------------------------------------------------------
  // Internal helpers
  // ---------------------------------------------------------------------------

  private rpcUrl: string;

  /** Build and simulate a read-only contract call, returning the decoded value. */
  private async simulate<T>(method: string, args: xdr.ScVal[]): Promise<T> {
    const account = await this.server.getAccount(
      "GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN"
    );
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(this.contract.call(method, ...args))
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);
    if (rpc.Api.isSimulationError(result)) {
      throw new Error(`Contract error in ${method}: ${result.error}`);
    }
    if (!rpc.Api.isSimulationSuccess(result) || !result.result) {
      throw new Error(`Unexpected simulation result for ${method}`);
    }
    return scValToNative(result.result.retval) as T;
  }

  // ---------------------------------------------------------------------------
  // Attestation queries
  // ---------------------------------------------------------------------------

  /** Fetch a single attestation by its ID. */
  async getAttestation(attestationId: string): Promise<Attestation> {
    return this.simulate<Attestation>("get_attestation", [
      xdr.ScVal.scvString(attestationId),
    ]);
  }

  /** Fetch the live status of an attestation. */
  async getAttestationStatus(attestationId: string): Promise<AttestationStatus> {
    return this.simulate<AttestationStatus>("get_attestation_status", [
      xdr.ScVal.scvString(attestationId),
    ]);
  }

  /** All attestation IDs for a subject address. */
  async getSubjectAttestations(subject: string): Promise<string[]> {
    return this.simulate<string[]>("get_subject_attestations", [
      xdr.ScVal.scvAddress(xdr.ScAddress.scAddressTypeAccount(
        xdr.PublicKey.publicKeyTypeEd25519(Buffer.from(subject, "hex"))
      )),
    ]);
  }

  /**
   * Fetch attestations whose `timestamp` falls within [start, end] (inclusive).
   *
   * Maps to the contract's `get_attestations_in_range(start, end)` entry point.
   *
   * @param start - Lower-bound Unix timestamp (seconds).
   * @param end   - Upper-bound Unix timestamp (seconds).
   * @returns Array of matching attestations.
   */
  async getAttestationsInRange(
    start: number,
    end: number
  ): Promise<Attestation[]> {
    return this.simulate<Attestation[]>("get_attestations_in_range", [
      xdr.ScVal.scvU64(xdr.Uint64.fromString(String(start))),
      xdr.ScVal.scvU64(xdr.Uint64.fromString(String(end))),
    ]);
  }

  /**
   * Cursor-based range query — returns up to `limit` attestations created
   * **after** the attestation identified by `cursor`.
   *
   * Maps to the contract's `get_attestations_in_range_after(cursor, limit)`.
   *
   * @param cursor - ID of the last seen attestation (exclusive lower bound).
   * @param limit  - Maximum number of results to return.
   * @returns Array of attestations following the cursor.
   */
  async getAttestationsInRangeAfter(
    cursor: string,
    limit: number
  ): Promise<Attestation[]> {
    return this.simulate<Attestation[]>("get_attestations_in_range_after", [
      xdr.ScVal.scvString(cursor),
      xdr.ScVal.scvU32(limit),
    ]);
  }

  /** Attestations issued by a given issuer address. */
  async getIssuerAttestations(issuer: string): Promise<string[]> {
    return this.simulate<string[]>("get_issuer_attestations", [
      xdr.ScVal.scvAddress(xdr.ScAddress.scAddressTypeAccount(
        xdr.PublicKey.publicKeyTypeEd25519(Buffer.from(issuer, "hex"))
      )),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Admin / Council (issue #742)
  // ---------------------------------------------------------------------------

  /**
   * Fetch the current admin council configuration.
   *
   * Maps to the contract's `get_council()` entry point.
   */
  async getCouncil(): Promise<Council> {
    return this.simulate<Council>("get_council", []);
  }

  /**
   * Fetch a single admin-council proposal by its ID.
   *
   * Maps to the contract's `get_council_proposal(proposal_id)` entry point.
   *
   * @param proposalId - The unique proposal identifier.
   */
  async getCouncilProposal(proposalId: string): Promise<CouncilProposal> {
    return this.simulate<CouncilProposal>("get_council_proposal", [
      xdr.ScVal.scvString(proposalId),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Storage limits (issue #743)
  // ---------------------------------------------------------------------------

  /**
   * Fetch the contract's configured storage limits.
   *
   * Maps to the contract's `get_limits()` entry point.
   */
  async getLimits(): Promise<StorageLimits> {
    return this.simulate<StorageLimits>("get_limits", []);
  }

  // ---------------------------------------------------------------------------
  // Multi-sig proposals
  // ---------------------------------------------------------------------------

  /** Fetch an admin multi-sig proposal by ID. */
  async getMultisigProposal(proposalId: string): Promise<MultiSigProposal> {
    return this.simulate<MultiSigProposal>("get_multisig_proposal", [
      xdr.ScVal.scvString(proposalId),
    ]);
  }

  // ---------------------------------------------------------------------------
  // Misc
  // ---------------------------------------------------------------------------

  async getAdmin(): Promise<string> {
    return this.simulate<string>("get_admin", []);
  }

  async getFeeConfig(): Promise<FeeConfig> {
    return this.simulate<FeeConfig>("get_fee_config", []);
  }

  async getIssuerMetadata(issuer: string): Promise<IssuerMetadata | undefined> {
    return this.simulate<IssuerMetadata | undefined>("get_issuer_metadata", [
      xdr.ScVal.scvAddress(xdr.ScAddress.scAddressTypeAccount(
        xdr.PublicKey.publicKeyTypeEd25519(Buffer.from(issuer, "hex"))
      )),
    ]);
  }

  async getContractMetadata(): Promise<ContractMetadata> {
    return this.simulate<ContractMetadata>("get_contract_metadata", []);
  }

  async getVersion(): Promise<string> {
    return this.simulate<string>("get_version", []);
  }
}

/** Decode a Soroban ScVal to a plain JS value (best-effort). */
function scValToNative(val: xdr.ScVal): unknown {
  switch (val.switch()) {
    case xdr.ScValType.scvString():
      return val.str().toString();
    case xdr.ScValType.scvSymbol():
      return val.sym().toString();
    case xdr.ScValType.scvBool():
      return val.b();
    case xdr.ScValType.scvU32():
      return val.u32();
    case xdr.ScValType.scvI32():
      return val.i32();
    case xdr.ScValType.scvU64():
      return Number(val.u64().toString());
    case xdr.ScValType.scvI64():
      return Number(val.i64().toString());
    case xdr.ScValType.scvVec(): {
      const vec = val.vec();
      return vec ? vec.map(scValToNative) : [];
    }
    case xdr.ScValType.scvMap(): {
      const entries = val.map() ?? [];
      const obj: Record<string, unknown> = {};
      for (const entry of entries) {
        const key = scValToNative(entry.key()) as string;
        obj[key] = scValToNative(entry.val());
      }
      return obj;
    }
    case xdr.ScValType.scvVoid():
      return undefined;
    default:
      return val;
  }
}

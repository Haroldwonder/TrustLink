/**
 * TrustLink Contract — TypeScript client
 *
 * Auto-generated from src/lib.rs.
 * Do NOT edit by hand — run `make bindings` to regenerate.
 */

import {
  Account,
  Contract,
  Keypair,
  Networks,
  rpc,
  Transaction,
  TransactionBuilder,
  xdr,
  BASE_FEE,
  nativeToScVal,
  scValToNative,
} from "@stellar/stellar-sdk";

import {
  Attestation,
  AttestationStatus,
  AuditEntry,
  ClaimTypeInfo,
  ContractConfig,
  ContractMetadata,
  Endorsement,
  FeeConfig,
  GlobalStats,
  HealthStatus,
  IssuerMetadata,
  IssuerStats,
  IssuerTier,
  MultiSigProposal,
  TtlConfig,
  CONTRACT_ERRORS,
} from "./types";

export type { Attestation, AttestationStatus, AuditEntry, ClaimTypeInfo,
  ContractConfig, ContractMetadata, Endorsement, FeeConfig, GlobalStats,
  HealthStatus, IssuerMetadata, IssuerStats, IssuerTier, MultiSigProposal,
  TtlConfig };

// ─── Client options ───────────────────────────────────────────────────────────

export interface TrustLinkClientOptions {
  /** Deployed contract address (C…). */
  contractId: string;
  /** Stellar RPC server URL. */
  rpcUrl: string;
  /** Network passphrase. Defaults to testnet. */
  networkPassphrase?: string;
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

function str(s: string): xdr.ScVal {
  return nativeToScVal(s, { type: "string" });
}

function addr(a: string): xdr.ScVal {
  return nativeToScVal(a, { type: "address" });
}

function u64(n: bigint): xdr.ScVal {
  return nativeToScVal(n, { type: "u64" });
}

function u32(n: number): xdr.ScVal {
  return nativeToScVal(n, { type: "u32" });
}

function bool(b: boolean): xdr.ScVal {
  return nativeToScVal(b, { type: "bool" });
}

function optionVal(v: xdr.ScVal | null): xdr.ScVal {
  if (v === null) return xdr.ScVal.scvVoid();
  return xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("Some"), v]);
}

function vecStr(env: string[]): xdr.ScVal {
  return xdr.ScVal.scvVec(env.map(str));
}

function vecAddr(addrs: string[]): xdr.ScVal {
  return xdr.ScVal.scvVec(addrs.map(addr));
}

function parseError(result: rpc.Api.SimulateTransactionResponse): string {
  if (rpc.Api.isSimulationError(result)) {
    const match = result.error.match(/Error\(Contract, #(\d+)\)/);
    if (match) {
      const code = parseInt(match[1], 10);
      return CONTRACT_ERRORS[code] ?? `ContractError(${code})`;
    }
    return result.error;
  }
  return "Unknown error";
}

// ─── Client ───────────────────────────────────────────────────────────────────

export class TrustLinkClient {
  private readonly contract: Contract;
  private readonly server: rpc.Server;
  private readonly networkPassphrase: string;
  private readonly contractId: string;

  constructor(opts: TrustLinkClientOptions) {
    this.contractId = opts.contractId;
    this.contract = new Contract(opts.contractId);
    this.server = new rpc.Server(opts.rpcUrl, { allowHttp: opts.rpcUrl.startsWith("http://") });
    this.networkPassphrase = opts.networkPassphrase ?? Networks.TESTNET;
  }

  // ─── Low-level helpers ──────────────────────────────────────────────────────

  /** Simulate a read-only call and return the decoded native value. */
  private async simulate(method: string, args: xdr.ScVal[]): Promise<unknown> {
    const op = this.contract.call(method, ...args);
    const account = new Account("GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN", "0");
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(op)
      .setTimeout(30)
      .build();

    const result = await this.server.simulateTransaction(tx);
    if (rpc.Api.isSimulationError(result)) {
      throw new Error(parseError(result));
    }
    const simSuccess = result as rpc.Api.SimulateTransactionSuccessResponse;
    if (!simSuccess.result) throw new Error("No result returned from simulation");
    return scValToNative(simSuccess.result.retval);
  }

  /**
   * Build, simulate, sign, and submit a state-changing transaction.
   * Returns the transaction hash on success.
   */
  async invoke(
    method: string,
    args: xdr.ScVal[],
    signer: Keypair,
  ): Promise<string> {
    const account = await this.server.getAccount(signer.publicKey());
    const op = this.contract.call(method, ...args);
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(op)
      .setTimeout(30)
      .build();

    const simResult = await this.server.simulateTransaction(tx);
    if (rpc.Api.isSimulationError(simResult)) {
      throw new Error(parseError(simResult));
    }

    const prepared = rpc.assembleTransaction(tx, simResult).build() as Transaction;
    prepared.sign(signer);

    const sendResult = await this.server.sendTransaction(prepared);
    if (sendResult.status === "ERROR") {
      throw new Error(`Transaction failed: ${JSON.stringify(sendResult.errorResult)}`);
    }

    let getResult = await this.server.getTransaction(sendResult.hash);
    for (let i = 0; i < 20 && getResult.status === "NOT_FOUND"; i++) {
      await new Promise((r) => setTimeout(r, 1500));
      getResult = await this.server.getTransaction(sendResult.hash);
    }

    if (getResult.status !== "SUCCESS") {
      throw new Error(`Transaction status: ${getResult.status}`);
    }

    return sendResult.hash;
  }

  // ─── Admin ──────────────────────────────────────────────────────────────────

  /** Initialize the contract. Must be called once after deployment. */
  async initialize(signer: Keypair, ttlDays?: number): Promise<string> {
    return this.invoke("initialize", [
      addr(signer.publicKey()),
      optionVal(ttlDays !== undefined ? u32(ttlDays) : null),
    ], signer);
  }

  /** Transfer admin rights to a new address. */
  async transferAdmin(currentAdmin: Keypair, newAdmin: string): Promise<string> {
    return this.invoke("transfer_admin", [
      addr(currentAdmin.publicKey()),
      addr(newAdmin),
    ], currentAdmin);
  }

  /** Return the current admin address. */
  async getAdmin(): Promise<string> {
    return this.simulate("get_admin", []) as Promise<string>;
  }

  /** Return the contract version string. */
  async getVersion(): Promise<string> {
    return this.simulate("get_version", []) as Promise<string>;
  }

  /** Return the full contract configuration snapshot. */
  async getConfig(): Promise<ContractConfig> {
    return this.simulate("get_config", []) as Promise<ContractConfig>;
  }

  /** Return contract name, version, and description. */
  async getContractMetadata(): Promise<ContractMetadata> {
    return this.simulate("get_contract_metadata", []) as Promise<ContractMetadata>;
  }

  // ─── Pause ──────────────────────────────────────────────────────────────────

  /** Pause the contract, disabling all write operations. */
  async pause(admin: Keypair): Promise<string> {
    return this.invoke("pause", [addr(admin.publicKey())], admin);
  }

  /** Unpause the contract, re-enabling write operations. */
  async unpause(admin: Keypair): Promise<string> {
    return this.invoke("unpause", [addr(admin.publicKey())], admin);
  }

  /** Return true if the contract is currently paused. */
  async isPaused(): Promise<boolean> {
    return this.simulate("is_paused", []) as Promise<boolean>;
  }

  // ─── Fees ───────────────────────────────────────────────────────────────────

  /** Configure the attestation fee. Pass fee=0 to disable. */
  async setFee(
    admin: Keypair,
    fee: bigint,
    collector: string,
    feeToken?: string,
  ): Promise<string> {
    return this.invoke("set_fee", [
      addr(admin.publicKey()),
      nativeToScVal(fee, { type: "i128" }),
      addr(collector),
      optionVal(feeToken ? addr(feeToken) : null),
    ], admin);
  }

  /** Return the current fee configuration. */
  async getFeeConfig(): Promise<FeeConfig> {
    return this.simulate("get_fee_config", []) as Promise<FeeConfig>;
  }

  // ─── Issuers ─────────────────────────────────────────────────────────────────

  /** Register a trusted issuer. Admin only. */
  async registerIssuer(admin: Keypair, issuer: string): Promise<string> {
    return this.invoke("register_issuer", [addr(admin.publicKey()), addr(issuer)], admin);
  }

  /** Remove a registered issuer. Admin only. */
  async removeIssuer(admin: Keypair, issuer: string): Promise<string> {
    return this.invoke("remove_issuer", [addr(admin.publicKey()), addr(issuer)], admin);
  }

  /** Return true if the address is a registered issuer. */
  async isIssuer(address: string): Promise<boolean> {
    return this.simulate("is_issuer", [addr(address)]) as Promise<boolean>;
  }

  /** Update the trust tier of a registered issuer. Admin only. */
  async updateIssuerTier(admin: Keypair, issuer: string, tier: IssuerTier): Promise<string> {
    return this.invoke("update_issuer_tier", [
      addr(admin.publicKey()),
      addr(issuer),
      nativeToScVal(tier, { type: "u32" }),
    ], admin);
  }

  /** Return the trust tier of an issuer, or null if not set. */
  async getIssuerTier(issuer: string): Promise<IssuerTier | null> {
    return this.simulate("get_issuer_tier", [addr(issuer)]) as Promise<IssuerTier | null>;
  }

  /** Set display metadata for a registered issuer. */
  async setIssuerMetadata(issuer: Keypair, metadata: IssuerMetadata): Promise<string> {
    return this.invoke("set_issuer_metadata", [
      addr(issuer.publicKey()),
      nativeToScVal(metadata),
    ], issuer);
  }

  /** Return metadata for an issuer, or null if not set. */
  async getIssuerMetadata(issuer: string): Promise<IssuerMetadata | null> {
    return this.simulate("get_issuer_metadata", [addr(issuer)]) as Promise<IssuerMetadata | null>;
  }

  /** Return per-issuer statistics. */
  async getIssuerStats(issuer: string): Promise<IssuerStats> {
    return this.simulate("get_issuer_stats", [addr(issuer)]) as Promise<IssuerStats>;
  }

  // ─── Bridges ─────────────────────────────────────────────────────────────────

  /** Register a trusted bridge contract. Admin only. */
  async registerBridge(admin: Keypair, bridgeContract: string): Promise<string> {
    return this.invoke("register_bridge", [addr(admin.publicKey()), addr(bridgeContract)], admin);
  }

  /** Return true if the address is a registered bridge contract. */
  async isBridge(address: string): Promise<boolean> {
    return this.simulate("is_bridge", [addr(address)]) as Promise<boolean>;
  }

  // ─── Claim types ─────────────────────────────────────────────────────────────

  /** Register a claim type with a description. Admin only. */
  async registerClaimType(admin: Keypair, claimType: string, description: string): Promise<string> {
    return this.invoke("register_claim_type", [
      addr(admin.publicKey()),
      str(claimType),
      str(description),
    ], admin);
  }

  /** Return the description for a claim type, or null if not registered. */
  async getClaimTypeDescription(claimType: string): Promise<string | null> {
    return this.simulate("get_claim_type_description", [str(claimType)]) as Promise<string | null>;
  }

  /** Return a paginated list of registered claim type identifiers. */
  async listClaimTypes(start: number, limit: number): Promise<string[]> {
    return this.simulate("list_claim_types", [u32(start), u32(limit)]) as Promise<string[]>;
  }

  // ─── Attestations ────────────────────────────────────────────────────────────

  /** Create a native attestation. Returns the attestation ID. */
  async createAttestation(
    issuer: Keypair,
    subject: string,
    claimType: string,
    expiration?: bigint,
    metadata?: string,
    tags?: string[],
  ): Promise<string> {
    return this.invoke("create_attestation", [
      addr(issuer.publicKey()),
      addr(subject),
      str(claimType),
      optionVal(expiration !== undefined ? u64(expiration) : null),
      optionVal(metadata !== undefined ? str(metadata) : null),
      optionVal(tags !== undefined ? vecStr(tags) : null),
    ], issuer);
  }

  /** Import a historical attestation. Admin only. Returns the attestation ID. */
  async importAttestation(
    admin: Keypair,
    issuer: string,
    subject: string,
    claimType: string,
    timestamp: bigint,
    expiration?: bigint,
  ): Promise<string> {
    return this.invoke("import_attestation", [
      addr(admin.publicKey()),
      addr(issuer),
      addr(subject),
      str(claimType),
      u64(timestamp),
      optionVal(expiration !== undefined ? u64(expiration) : null),
    ], admin);
  }

  /** Bridge an attestation from another chain. Returns the attestation ID. */
  async bridgeAttestation(
    bridge: Keypair,
    subject: string,
    claimType: string,
    sourceChain: string,
    sourceTx: string,
  ): Promise<string> {
    return this.invoke("bridge_attestation", [
      addr(bridge.publicKey()),
      addr(subject),
      str(claimType),
      str(sourceChain),
      str(sourceTx),
    ], bridge);
  }

  /**
   * Create attestations for multiple subjects in one transaction.
   * Returns the list of attestation IDs in subject order.
   */
  async createAttestationsBatch(
    issuer: Keypair,
    subjects: string[],
    claimType: string,
    expiration?: bigint,
  ): Promise<string[]> {
    return this.invoke("create_attestations_batch", [
      addr(issuer.publicKey()),
      vecAddr(subjects),
      str(claimType),
      optionVal(expiration !== undefined ? u64(expiration) : null),
    ], issuer) as unknown as Promise<string[]>;
  }

  /** Revoke an attestation. Issuer only. */
  async revokeAttestation(issuer: Keypair, attestationId: string, reason?: string): Promise<string> {
    return this.invoke("revoke_attestation", [
      addr(issuer.publicKey()),
      str(attestationId),
      optionVal(reason !== undefined ? str(reason) : null),
    ], issuer);
  }

  /** Revoke multiple attestations in one transaction. Returns the count revoked. */
  async revokeAttestationsBatch(
    issuer: Keypair,
    attestationIds: string[],
    reason?: string,
  ): Promise<number> {
    return this.invoke("revoke_attestations_batch", [
      addr(issuer.publicKey()),
      vecStr(attestationIds),
      optionVal(reason !== undefined ? str(reason) : null),
    ], issuer) as unknown as Promise<number>;
  }

  /** Renew (extend or clear) the expiration of an attestation. Issuer only. */
  async renewAttestation(issuer: Keypair, attestationId: string, newExpiration?: bigint): Promise<string> {
    return this.invoke("renew_attestation", [
      addr(issuer.publicKey()),
      str(attestationId),
      optionVal(newExpiration !== undefined ? u64(newExpiration) : null),
    ], issuer);
  }

  /** Update the expiration of an attestation. Issuer only. */
  async updateExpiration(issuer: Keypair, attestationId: string, newExpiration?: bigint): Promise<string> {
    return this.invoke("update_expiration", [
      addr(issuer.publicKey()),
      str(attestationId),
      optionVal(newExpiration !== undefined ? u64(newExpiration) : null),
    ], issuer);
  }

  /** Request GDPR deletion of an attestation. Subject only. */
  async requestDeletion(subject: Keypair, attestationId: string): Promise<string> {
    return this.invoke("request_deletion", [
      addr(subject.publicKey()),
      str(attestationId),
    ], subject);
  }

  // ─── Queries ─────────────────────────────────────────────────────────────────

  /** Return a full attestation record by ID. */
  async getAttestation(attestationId: string): Promise<Attestation> {
    return this.simulate("get_attestation", [str(attestationId)]) as Promise<Attestation>;
  }

  /** Return the status of an attestation (Valid, Expired, Revoked, Pending). */
  async getAttestationStatus(attestationId: string): Promise<AttestationStatus> {
    return this.simulate("get_attestation_status", [str(attestationId)]) as Promise<AttestationStatus>;
  }

  /**
   * Return the most recent valid attestation for a subject + claim type.
   * Throws NotFound if none exists.
   */
  async getAttestationByType(subject: string, claimType: string): Promise<Attestation> {
    return this.simulate("get_attestation_by_type", [addr(subject), str(claimType)]) as Promise<Attestation>;
  }

  /** Return a paginated list of attestation IDs for a subject. */
  async getSubjectAttestations(subject: string, start: number, limit: number): Promise<string[]> {
    return this.simulate("get_subject_attestations", [addr(subject), u32(start), u32(limit)]) as Promise<string[]>;
  }

  /** Return a paginated list of attestation IDs created by an issuer. */
  async getIssuerAttestations(issuer: string, start: number, limit: number): Promise<string[]> {
    return this.simulate("get_issuer_attestations", [addr(issuer), u32(start), u32(limit)]) as Promise<string[]>;
  }

  /** Return all attestation IDs for a subject that carry a specific tag. */
  async getAttestationsByTag(subject: string, tag: string): Promise<string[]> {
    return this.simulate("get_attestations_by_tag", [addr(subject), str(tag)]) as Promise<string[]>;
  }

  /** Return the distinct claim types for which a subject holds a valid attestation. */
  async getValidClaims(subject: string): Promise<string[]> {
    return this.simulate("get_valid_claims", [addr(subject)]) as Promise<string[]>;
  }

  /** Return true if the subject holds a valid attestation of the given claim type. */
  async hasValidClaim(subject: string, claimType: string): Promise<boolean> {
    return this.simulate("has_valid_claim", [addr(subject), str(claimType)]) as Promise<boolean>;
  }

  /** Return true if the subject holds a valid attestation of the given claim type from a specific issuer. */
  async hasValidClaimFromIssuer(subject: string, claimType: string, issuer: string): Promise<boolean> {
    return this.simulate("has_valid_claim_from_issuer", [addr(subject), str(claimType), addr(issuer)]) as Promise<boolean>;
  }

  /** Return true if the subject holds a valid attestation of the given claim type from an issuer at or above min_tier. */
  async hasValidClaimFromTier(subject: string, claimType: string, minTier: IssuerTier): Promise<boolean> {
    return this.simulate("has_valid_claim_from_tier", [
      addr(subject),
      str(claimType),
      nativeToScVal(minTier, { type: "u32" }),
    ]) as Promise<boolean>;
  }

  /** Return true if the subject holds a valid attestation for ANY of the given claim types (OR logic). */
  async hasAnyClaim(subject: string, claimTypes: string[]): Promise<boolean> {
    return this.simulate("has_any_claim", [addr(subject), vecStr(claimTypes)]) as Promise<boolean>;
  }

  /** Return true if the subject holds a valid attestation for ALL of the given claim types (AND logic). */
  async hasAllClaims(subject: string, claimTypes: string[]): Promise<boolean> {
    return this.simulate("has_all_claims", [addr(subject), vecStr(claimTypes)]) as Promise<boolean>;
  }

  // ─── Audit log ───────────────────────────────────────────────────────────────

  /** Return the full append-only audit log for an attestation. */
  async getAuditLog(attestationId: string): Promise<AuditEntry[]> {
    return this.simulate("get_audit_log", [str(attestationId)]) as Promise<AuditEntry[]>;
  }

  // ─── Multi-sig ───────────────────────────────────────────────────────────────

  /**
   * Propose a multi-sig attestation. The proposer auto-signs.
   * Returns the proposal ID.
   */
  async proposeAttestation(
    proposer: Keypair,
    subject: string,
    claimType: string,
    requiredSigners: string[],
    threshold: number,
  ): Promise<string> {
    return this.invoke("propose_attestation", [
      addr(proposer.publicKey()),
      addr(subject),
      str(claimType),
      vecAddr(requiredSigners),
      u32(threshold),
    ], proposer);
  }

  /** Co-sign an existing multi-sig proposal. Finalizes when threshold is reached. */
  async cosignAttestation(issuer: Keypair, proposalId: string): Promise<string> {
    return this.invoke("cosign_attestation", [addr(issuer.publicKey()), str(proposalId)], issuer);
  }

  /** Return a multi-sig proposal by ID. */
  async getMultisigProposal(proposalId: string): Promise<MultiSigProposal> {
    return this.simulate("get_multisig_proposal", [str(proposalId)]) as Promise<MultiSigProposal>;
  }

  // ─── Endorsements ────────────────────────────────────────────────────────────

  /** Endorse an existing attestation. Issuer only. */
  async endorseAttestation(endorser: Keypair, attestationId: string): Promise<string> {
    return this.invoke("endorse_attestation", [addr(endorser.publicKey()), str(attestationId)], endorser);
  }

  /** Return all endorsements for an attestation. */
  async getEndorsements(attestationId: string): Promise<Endorsement[]> {
    return this.simulate("get_endorsements", [str(attestationId)]) as Promise<Endorsement[]>;
  }

  /** Return the number of endorsements for an attestation. */
  async getEndorsementCount(attestationId: string): Promise<number> {
    return this.simulate("get_endorsement_count", [str(attestationId)]) as Promise<number>;
  }

  // ─── Stats & health ──────────────────────────────────────────────────────────

  /** Return global contract statistics. */
  async getGlobalStats(): Promise<GlobalStats> {
    return this.simulate("get_global_stats", []) as Promise<GlobalStats>;
  }

  /** Return a lightweight health status for monitoring dashboards. */
  async healthCheck(): Promise<HealthStatus> {
    return this.simulate("health_check", []) as Promise<HealthStatus>;
  }
}

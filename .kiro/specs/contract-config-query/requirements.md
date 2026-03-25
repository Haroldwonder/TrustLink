# Requirements Document

## Introduction

Integrators building on TrustLink need a single, convenient query to inspect the contract's current configuration — TTL settings, fee parameters, and contract identity metadata — without having to call multiple separate query functions. This feature adds a `get_config` function that aggregates all existing configurable parameters into a single `ContractConfig` response. The implementation is purely additive: it exposes configuration that already exists in contract storage (`TtlConfig`, `FeeConfig`, and contract metadata) and introduces no new enforcement mechanisms.

## Glossary

- **TrustLink**: The Soroban smart contract on the Stellar blockchain that manages attestations.
- **ContractConfig**: A composite struct returned by `get_config` that aggregates all current contract configuration values.
- **TtlConfig**: The existing struct storing `ttl_days: u32`, which controls how long attestation storage entries live before expiring from the ledger.
- **FeeConfig**: The existing struct storing `attestation_fee: i128`, `fee_collector: Address`, and `fee_token: Option<Address>`.
- **ContractMetadata**: The existing name, version, and description values returned by `get_contract_metadata`.
- **ConfigQuery**: The logical component responsible for reading and assembling the `ContractConfig` response.
- **Query**: A read-only contract function that does not mutate state.
- **Initialized**: The state of the contract after `initialize` has been called at least once.
- **Uninitialized**: The state of the contract before `initialize` has been called.

---

## Requirements

### Requirement 1: Define the ContractConfig Type

**User Story:** As a developer integrating TrustLink, I want a well-defined `ContractConfig` struct, so that I can reason about all contract configuration in a type-safe way.

#### Acceptance Criteria

1. THE TrustLink SHALL expose a `ContractConfig` type with the following fields: `ttl_config: TtlConfig`, `fee_config: FeeConfig`, `contract_name: String`, `contract_version: String`, and `contract_description: String`.
2. THE ContractConfig SHALL use the same `TtlConfig` and `FeeConfig` types already defined in the contract, with no duplication or redefinition.

---

### Requirement 2: Query the Full Contract Configuration

**User Story:** As an integrator, I want to call a single `get_config` function and receive all current contract configuration values, so that I can understand the contract's constraints without making multiple separate calls.

#### Acceptance Criteria

1. WHEN `get_config(env)` is called, THE ConfigQuery SHALL return a `ContractConfig` containing the current `TtlConfig`, `FeeConfig`, `contract_name`, `contract_version`, and `contract_description`.
2. THE ConfigQuery SHALL NOT mutate any contract state when executing `get_config`.
3. WHEN `get_config(env)` is called and the contract has been initialized, THE ConfigQuery SHALL return the values that were set during initialization.
4. WHEN `get_config(env)` is called and the contract has not been initialized, THE ConfigQuery SHALL return the contract's default values for each field.

---

### Requirement 3: Consistency with Existing Query Functions

**User Story:** As a developer, I want `get_config` to return values consistent with the existing individual query functions, so that I can trust the aggregated result.

#### Acceptance Criteria

1. FOR ALL states of the contract, the `ttl_config` field returned by `get_config` SHALL be equal to the value returned by the existing `get_ttl_config` function (or its equivalent).
2. FOR ALL states of the contract, the `fee_config` field returned by `get_config` SHALL be equal to the value returned by the existing `get_fee_config` function.
3. FOR ALL states of the contract, the `contract_name`, `contract_version`, and `contract_description` fields returned by `get_config` SHALL be equal to the values returned by the existing `get_contract_metadata` function.

---

### Requirement 4: Idempotence of the Query

**User Story:** As a developer, I want repeated calls to `get_config` with no intervening state changes to return identical results, so that I can rely on stable query behavior.

#### Acceptance Criteria

1. WHEN `get_config` is called twice in succession with no state changes between calls, THE ConfigQuery SHALL return identical `ContractConfig` values both times.
2. WHEN `get_config` is called before and after a read-only operation (such as another query), THE ConfigQuery SHALL return identical `ContractConfig` values both times.


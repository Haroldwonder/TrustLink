# Requirements Document

## Introduction

The TrustLink contract currently stores a single admin address set at initialization with no mechanism to change it. This creates a single point of failure: if the admin key is compromised or lost, the contract becomes permanently unmanageable. The admin transfer feature adds a `transfer_admin` function that allows the current admin to atomically hand off admin rights to a new address, emitting an auditable on-chain event and immediately revoking the old admin's privileges.

## Glossary

- **Contract**: The TrustLink Soroban smart contract deployed on the Stellar blockchain.
- **Admin**: The privileged address stored in contract instance storage under `StorageKey::Admin`, authorized to register/remove issuers, set fees, import attestations, and register claim types.
- **Caller**: The address passed as `current_admin` to `transfer_admin` and whose Soroban authorization is required.
- **New_Admin**: The address that will become the sole admin after a successful transfer.
- **Validation**: The `Validation` struct in `src/validation.rs` that provides authorization guard functions.
- **Storage**: The `Storage` struct in `src/storage.rs` that is the single point of contact with on-chain storage.
- **Events**: The `Events` struct in `src/events.rs` responsible for publishing on-chain events.

## Requirements

### Requirement 1: Admin Transfer Function

**User Story:** As the current admin, I want to transfer admin rights to a new address, so that I can rotate keys or hand off control without redeploying the contract.

#### Acceptance Criteria

1. WHEN `transfer_admin(env, current_admin, new_admin)` is called, THE Contract SHALL require Soroban authorization from `current_admin` via `current_admin.require_auth()`.
2. WHEN `transfer_admin` is called, THE Contract SHALL validate that `current_admin` matches the stored admin address using `Validation::require_admin`.
3. WHEN `transfer_admin` is called with a valid `current_admin`, THE Contract SHALL overwrite the stored admin address with `new_admin` using `Storage::set_admin`.
4. WHEN `transfer_admin` completes successfully, THE Contract SHALL emit an `admin_transferred` event containing the old admin address and the new admin address.
5. WHEN `transfer_admin` completes successfully, THE Contract SHALL return `Ok(())`.

### Requirement 2: Authorization Enforcement

**User Story:** As a contract user, I want only the current admin to be able to transfer admin rights, so that unauthorized parties cannot seize control of the contract.

#### Acceptance Criteria

1. IF `transfer_admin` is called by an address that is not the current admin, THEN THE Contract SHALL return `Error::Unauthorized`.
2. IF `transfer_admin` is called before the contract is initialized, THEN THE Contract SHALL return `Error::NotInitialized`.
3. WHILE the contract is initialized, THE Contract SHALL reject any `transfer_admin` call where `current_admin` does not match the stored admin address.

### Requirement 3: Privilege Revocation After Transfer

**User Story:** As a contract operator, I want the old admin to immediately lose all admin privileges after a transfer, so that a compromised key cannot be used after rotation.

#### Acceptance Criteria

1. WHEN `transfer_admin` completes successfully, THE Contract SHALL store only `new_admin` as the admin, such that any subsequent admin-gated call using the old admin address returns `Error::Unauthorized`.
2. WHEN `transfer_admin` completes successfully, THE Contract SHALL allow `new_admin` to immediately call `register_issuer` without error.

### Requirement 4: Event Emission

**User Story:** As an off-chain observer, I want an on-chain event emitted when admin rights are transferred, so that I can audit admin key rotations.

#### Acceptance Criteria

1. WHEN `transfer_admin` completes successfully, THE Events module SHALL publish an event with topic symbol `adm_xfer` containing the old admin address and the new admin address as event data.
2. THE Contract SHALL emit exactly one `admin_transferred` event per successful `transfer_admin` invocation.

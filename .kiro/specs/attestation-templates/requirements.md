# Requirements Document

## Introduction

Issuers on TrustLink frequently create attestations with the same parameters (claim type, expiration, metadata). This feature introduces **Attestation Templates** — reusable, per-issuer blueprints that capture default values for attestation creation. Issuers can create a template once and instantiate attestations from it, optionally overriding individual fields at creation time. This reduces repetition, lowers the chance of parameter errors, and makes bulk issuance workflows more ergonomic.

## Glossary

- **TrustLink**: The Soroban smart contract that manages on-chain attestations.
- **Issuer**: A registered address authorized to create attestations.
- **Subject**: The address that an attestation is about.
- **AttestationTemplate**: A named, per-issuer record that stores default values for `claim_type`, `default_expiration_days`, and `metadata_template`.
- **Template_ID**: A caller-supplied `String` identifier that uniquely identifies a template within an issuer's namespace.
- **Template_Store**: The on-chain storage layer that persists `AttestationTemplate` records keyed by `(Issuer, Template_ID)`.
- **Template_Registry**: The per-issuer ordered list of `Template_ID` values used for enumeration.
- **Claim_Type**: A `String` identifier for the kind of claim being attested (e.g. `"KYC"`, `"AML"`).
- **Default_Expiration_Days**: An optional `u32` number of days from attestation creation time after which the attestation expires. `None` means no expiration.
- **Metadata_Template**: An optional `String` (max 256 bytes) used as the default metadata value when creating an attestation from a template.
- **Override**: A caller-supplied value that replaces the template default for a single `create_attestation_from_template` call.

---

## Requirements

### Requirement 1: Define the AttestationTemplate Type

**User Story:** As a developer integrating TrustLink, I want a well-defined `AttestationTemplate` struct, so that I can reason about template data in a type-safe way.

#### Acceptance Criteria

1. THE TrustLink SHALL expose an `AttestationTemplate` type with the fields: `claim_type: String`, `default_expiration_days: Option<u32>`, and `metadata_template: Option<String>`.
2. THE TrustLink SHALL enforce that `metadata_template`, when present, does not exceed 256 bytes in length.
3. THE TrustLink SHALL enforce that `claim_type` is a non-empty string.

---

### Requirement 2: Create a Template

**User Story:** As an issuer, I want to create a named attestation template, so that I can reuse default parameters across multiple attestation issuances.

#### Acceptance Criteria

1. WHEN an issuer calls `create_template` with a valid `template_id` and `AttestationTemplate`, THE TrustLink SHALL persist the template in the Template_Store keyed by `(issuer, template_id)`.
2. WHEN an issuer calls `create_template` with a `template_id` that already exists for that issuer, THE TrustLink SHALL overwrite the existing template with the new values.
3. WHEN a non-issuer address calls `create_template`, THE TrustLink SHALL return `Error::Unauthorized`.
4. WHEN `create_template` is called with a `metadata_template` exceeding 256 bytes, THE TrustLink SHALL return `Error::MetadataTooLong`.
5. WHEN `create_template` is called with an empty `claim_type`, THE TrustLink SHALL return `Error::InvalidClaimType`.
6. WHEN `create_template` succeeds, THE TrustLink SHALL add `template_id` to the issuer's Template_Registry if it is not already present.

---

### Requirement 3: Create an Attestation from a Template

**User Story:** As an issuer, I want to create an attestation using a template's defaults, so that I can issue consistent attestations without repeating parameters.

#### Acceptance Criteria

1. WHEN an issuer calls `create_attestation_from_template` with a valid `template_id` and `subject`, THE TrustLink SHALL create an attestation using the template's `claim_type` and `metadata_template` as defaults.
2. WHEN the template's `default_expiration_days` is `Some(n)`, THE TrustLink SHALL set the attestation's `expiration` to `current_ledger_timestamp + (n * 86400)`.
3. WHEN the template's `default_expiration_days` is `None`, THE TrustLink SHALL create the attestation with no expiration.
4. WHEN an issuer calls `create_attestation_from_template` with an `expiration_override` of `Some(timestamp)`, THE TrustLink SHALL use that timestamp as the attestation's expiration instead of the template default.
5. WHEN an issuer calls `create_attestation_from_template` with a `metadata_override` of `Some(value)`, THE TrustLink SHALL use that value as the attestation's metadata instead of the template default.
6. WHEN `create_attestation_from_template` is called with a `template_id` that does not exist for the issuer, THE TrustLink SHALL return `Error::NotFound`.
7. WHEN a non-issuer address calls `create_attestation_from_template`, THE TrustLink SHALL return `Error::Unauthorized`.
8. WHEN `create_attestation_from_template` is called with an `expiration_override` whose timestamp is not greater than the current ledger timestamp, THE TrustLink SHALL return `Error::InvalidExpiration`.
9. WHEN `create_attestation_from_template` is called with a `metadata_override` exceeding 256 bytes, THE TrustLink SHALL return `Error::MetadataTooLong`.
10. WHEN `create_attestation_from_template` succeeds, THE TrustLink SHALL store the resulting attestation using the same storage and indexing rules as `create_attestation`.

---

### Requirement 4: List Templates for an Issuer

**User Story:** As an issuer or verifier, I want to list all templates belonging to an issuer, so that I can discover available templates programmatically.

#### Acceptance Criteria

1. WHEN `list_templates` is called with a valid `issuer` address, THE TrustLink SHALL return the ordered `Vec<Template_ID>` of all template IDs registered for that issuer.
2. WHEN `list_templates` is called for an issuer with no templates, THE TrustLink SHALL return an empty `Vec`.
3. THE TrustLink SHALL return template IDs in the order they were first created.

---

### Requirement 5: Retrieve a Single Template

**User Story:** As an issuer or verifier, I want to retrieve a specific template by ID, so that I can inspect its default values before using it.

#### Acceptance Criteria

1. WHEN `get_template` is called with a valid `issuer` and `template_id`, THE TrustLink SHALL return the corresponding `AttestationTemplate`.
2. WHEN `get_template` is called with a `template_id` that does not exist for the issuer, THE TrustLink SHALL return `Error::NotFound`.

---

### Requirement 6: Template Storage Isolation

**User Story:** As an issuer, I want my templates to be isolated from other issuers' templates, so that template IDs do not collide across issuers.

#### Acceptance Criteria

1. THE Template_Store SHALL key each template by the combination of `(issuer, template_id)`, ensuring that two different issuers can each hold a template with the same `template_id` without conflict.
2. WHEN issuer A creates a template with `template_id` `"T1"` and issuer B creates a template with `template_id` `"T1"`, THE TrustLink SHALL store and retrieve each template independently.

---

### Requirement 7: Template Lifecycle Events

**User Story:** As an off-chain indexer, I want contract events emitted for template operations, so that I can track template creation and usage without polling storage.

#### Acceptance Criteria

1. WHEN `create_template` succeeds, THE TrustLink SHALL emit a `template_created` event containing `issuer` and `template_id`.
2. WHEN `create_attestation_from_template` succeeds, THE TrustLink SHALL emit the standard `attestation_created` event (same as `create_attestation`) for the resulting attestation.

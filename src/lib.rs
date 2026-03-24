#![no_std]

mod events;
mod storage;
pub mod types;
mod validation;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, String, Vec};

use crate::events::Events;
use crate::storage::Storage;
use crate::types::{
    Attestation, AttestationStatus, ClaimTypeInfo, ContractMetadata, Error, FeeConfig,
    IssuerMetadata,
};
use crate::validation::Validation;

fn validate_metadata(metadata: &Option<String>) -> Result<(), Error> {
    if let Some(value) = metadata {
        if value.len() > 256 {
            return Err(Error::MetadataTooLong);
        }
    }
    Ok(())
}

fn validate_tags(tags: &Option<Vec<String>>) -> Result<(), Error> {
    if let Some(t) = tags {
        if t.len() > 5 {
            return Err(Error::TooManyTags);
        }
        for tag in t.iter() {
            if tag.len() > 32 {
                return Err(Error::TagTooLong);
            }
        }
    }
    Ok(())
}

fn validate_native_expiration(env: &Env, expiration: Option<u64>) -> Result<(), Error> {
    if let Some(value) = expiration {
        if value <= env.ledger().timestamp() {
            return Err(Error::InvalidExpiration);
        }
    }
    Ok(())
}

fn validate_import_timestamps(
    env: &Env,
    timestamp: u64,
    expiration: Option<u64>,
) -> Result<(), Error> {
    if timestamp > env.ledger().timestamp() {
        return Err(Error::InvalidTimestamp);
    }

    if let Some(value) = expiration {
        if value <= timestamp {
            return Err(Error::InvalidExpiration);
        }
    }

    Ok(())
}

fn validate_fee_config(fee: i128, fee_token: &Option<Address>) -> Result<(), Error> {
    if fee < 0 {
        return Err(Error::InvalidFee);
    }

    if fee > 0 && fee_token.is_none() {
        return Err(Error::FeeTokenRequired);
    }

    Ok(())
}

fn default_fee_config(admin: &Address) -> FeeConfig {
    FeeConfig {
        attestation_fee: 0,
        fee_collector: admin.clone(),
        fee_token: None,
    }
}

fn load_fee_config(env: &Env) -> Result<FeeConfig, Error> {
    Storage::get_fee_config(env).ok_or(Error::NotInitialized)
}

fn charge_attestation_fee(env: &Env, issuer: &Address) -> Result<(), Error> {
    let fee_config = load_fee_config(env)?;

    if fee_config.attestation_fee < 0 {
        return Err(Error::InvalidFee);
    }

    if fee_config.attestation_fee == 0 {
        return Ok(());
    }

    let fee_token = fee_config.fee_token.ok_or(Error::FeeTokenRequired)?;
    TokenClient::new(env, &fee_token).transfer(
        issuer,
        &fee_config.fee_collector,
        &fee_config.attestation_fee,
    );

    Ok(())
}

fn store_attestation(env: &Env, attestation: &Attestation) {
    Storage::set_attestation(env, attestation);
    Storage::add_subject_attestation(env, &attestation.subject, &attestation.id);
    Storage::add_issuer_attestation(env, &attestation.issuer, &attestation.id);
}

fn paginate_strings(env: &Env, values: Vec<String>, start: u32, limit: u32) -> Vec<String> {
    let total = values.len();
    let end = (start + limit).min(total);
    let mut result = Vec::new(env);

    for index in start..end {
        if let Some(value) = values.get(index) {
            result.push_back(value);
        }
    }

    result
}

#[contract]
pub struct TrustLinkContract;

#[contractimpl]
impl TrustLinkContract {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();
        Storage::set_admin(&env, &admin);
        Storage::set_version(&env, &String::from_str(&env, "1.0.0"));
        Storage::set_fee_config(&env, &default_fee_config(&admin));
        Events::admin_initialized(&env, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn register_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::add_issuer(&env, &issuer);
        Events::issuer_registered(&env, &issuer, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn remove_issuer(env: Env, admin: Address, issuer: Address) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::remove_issuer(&env, &issuer);
        Events::issuer_removed(&env, &issuer, &admin, env.ledger().timestamp());
        Ok(())
    }

    pub fn register_bridge(
        env: Env,
        admin: Address,
        bridge_contract: Address,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Storage::add_bridge(&env, &bridge_contract);
        Ok(())
    }

    pub fn set_fee(
        env: Env,
        admin: Address,
        fee: i128,
        collector: Address,
        fee_token: Option<Address>,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        validate_fee_config(fee, &fee_token)?;

        Storage::set_fee_config(
            &env,
            &FeeConfig {
                attestation_fee: fee,
                fee_collector: collector,
                fee_token,
            },
        );

        Ok(())
    }

    pub fn create_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        claim_type: String,
        expiration: Option<u64>,
        metadata: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<String, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_metadata(&metadata)?;
        validate_tags(&tags)?;
        validate_native_expiration(&env, expiration)?;

        let timestamp = env.ledger().timestamp();
        let attestation_id =
            Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        charge_attestation_fee(&env, &issuer)?;

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer,
            subject,
            claim_type,
            timestamp,
            expiration,
            revoked: false,
            metadata,
            valid_from: None,
            imported: false,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags,
        };

        store_attestation(&env, &attestation);
        Events::attestation_created(&env, &attestation);
        Ok(attestation_id)
    }

    pub fn import_attestation(
        env: Env,
        admin: Address,
        issuer: Address,
        subject: Address,
        claim_type: String,
        timestamp: u64,
        expiration: Option<u64>,
    ) -> Result<String, Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;
        Validation::require_issuer(&env, &issuer)?;
        validate_import_timestamps(&env, timestamp, expiration)?;

        let attestation_id =
            Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer,
            subject,
            claim_type,
            timestamp,
            expiration,
            revoked: false,
            metadata: None,
            valid_from: None,
            imported: true,
            bridged: false,
            source_chain: None,
            source_tx: None,
            tags: None,
        };

        store_attestation(&env, &attestation);
        Events::attestation_imported(&env, &attestation);
        Ok(attestation_id)
    }

    pub fn bridge_attestation(
        env: Env,
        bridge: Address,
        subject: Address,
        claim_type: String,
        source_chain: String,
        source_tx: String,
    ) -> Result<String, Error> {
        bridge.require_auth();
        Validation::require_bridge(&env, &bridge)?;

        let timestamp = env.ledger().timestamp();
        let attestation_id = Attestation::generate_bridge_id(
            &env,
            &bridge,
            &subject,
            &claim_type,
            &source_chain,
            &source_tx,
            timestamp,
        );

        if Storage::has_attestation(&env, &attestation_id) {
            return Err(Error::DuplicateAttestation);
        }

        let attestation = Attestation {
            id: attestation_id.clone(),
            issuer: bridge,
            subject,
            claim_type,
            timestamp,
            expiration: None,
            revoked: false,
            metadata: None,
            valid_from: None,
            imported: false,
            bridged: true,
            source_chain: Some(source_chain),
            source_tx: Some(source_tx),
            tags: None,
        };

        store_attestation(&env, &attestation);
        Events::attestation_bridged(&env, &attestation);
        Ok(attestation_id)
    }

    pub fn create_attestations_batch(
        env: Env,
        issuer: Address,
        subjects: Vec<Address>,
        claim_type: String,
        expiration: Option<u64>,
    ) -> Result<Vec<String>, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_native_expiration(&env, expiration)?;

        let timestamp = env.ledger().timestamp();
        let mut ids = Vec::new(&env);

        for subject in subjects.iter() {
            let attestation_id =
                Attestation::generate_id(&env, &issuer, &subject, &claim_type, timestamp);

            if Storage::has_attestation(&env, &attestation_id) {
                return Err(Error::DuplicateAttestation);
            }

            let attestation = Attestation {
                id: attestation_id.clone(),
                issuer: issuer.clone(),
                subject: subject.clone(),
                claim_type: claim_type.clone(),
                timestamp,
                expiration,
                revoked: false,
                metadata: None,
                valid_from: None,
                imported: false,
                bridged: false,
                source_chain: None,
                source_tx: None,
                tags: None,
            };

            store_attestation(&env, &attestation);
            Events::attestation_created(&env, &attestation);
            ids.push_back(attestation_id);
        }

        Ok(ids)
    }

    pub fn revoke_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
    ) -> Result<(), Error> {
        issuer.require_auth();
        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }

        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.revoked = true;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_revoked(&env, &attestation_id, &issuer);
        Ok(())
    }

    pub fn revoke_attestations_batch(
        env: Env,
        issuer: Address,
        attestation_ids: Vec<String>,
    ) -> Result<u32, Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;

        let mut count = 0;
        for attestation_id in attestation_ids.iter() {
            let mut attestation = Storage::get_attestation(&env, &attestation_id)?;

            if attestation.issuer != issuer {
                return Err(Error::Unauthorized);
            }

            if attestation.revoked {
                return Err(Error::AlreadyRevoked);
            }

            attestation.revoked = true;
            Storage::set_attestation(&env, &attestation);
            Events::attestation_revoked(&env, &attestation_id, &issuer);
            count += 1;
        }

        Ok(count)
    }

    pub fn renew_attestation(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        validate_native_expiration(&env, new_expiration)?;

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_renewed(&env, &attestation_id, &issuer, new_expiration);
        Ok(())
    }

    pub fn update_expiration(
        env: Env,
        issuer: Address,
        attestation_id: String,
        new_expiration: Option<u64>,
    ) -> Result<(), Error> {
        issuer.require_auth();

        if let Some(value) = new_expiration {
            if value <= env.ledger().timestamp() {
                return Err(Error::InvalidExpiration);
            }
        }

        let mut attestation = Storage::get_attestation(&env, &attestation_id)?;
        if attestation.issuer != issuer {
            return Err(Error::Unauthorized);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        attestation.expiration = new_expiration;
        Storage::set_attestation(&env, &attestation);
        Events::attestation_updated(&env, &attestation_id, &issuer, new_expiration);
        Ok(())
    }

    pub fn has_valid_claim(env: Env, subject: Address, claim_type: String) -> bool {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        for attestation_id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.claim_type == claim_type {
                    match attestation.get_status(current_time) {
                        AttestationStatus::Valid => return true,
                        AttestationStatus::Expired => {
                            Events::attestation_expired(&env, &attestation_id, &subject);
                        }
                        AttestationStatus::Revoked | AttestationStatus::Pending => {}
                    }
                }
            }
        }

        false
    }

    pub fn has_any_claim(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() {
            return false;
        }

        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        for claim_type in claim_types.iter() {
            for attestation_id in attestation_ids.iter() {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                    if attestation.claim_type == claim_type
                        && attestation.get_status(current_time) == AttestationStatus::Valid
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn has_all_claims(env: Env, subject: Address, claim_types: Vec<String>) -> bool {
        if claim_types.is_empty() {
            return true;
        }

        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();

        'claims: for claim_type in claim_types.iter() {
            for attestation_id in attestation_ids.iter() {
                if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                    if attestation.claim_type == claim_type
                        && attestation.get_status(current_time) == AttestationStatus::Valid
                    {
                        continue 'claims;
                    }
                }
            }

            return false;
        }

        true
    }

    pub fn get_attestation(env: Env, attestation_id: String) -> Result<Attestation, Error> {
        Storage::get_attestation(&env, &attestation_id)
    }

    pub fn get_attestation_status(
        env: Env,
        attestation_id: String,
    ) -> Result<AttestationStatus, Error> {
        let attestation = Storage::get_attestation(&env, &attestation_id)?;
        let status = attestation.get_status(env.ledger().timestamp());

        if status == AttestationStatus::Expired {
            Events::attestation_expired(&env, &attestation_id, &attestation.subject);
        }

        Ok(status)
    }

    pub fn get_subject_attestations(
        env: Env,
        subject: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        paginate_strings(
            &env,
            Storage::get_subject_attestations(&env, &subject),
            start,
            limit,
        )
    }

    pub fn get_attestations_by_tag(
        env: Env,
        subject: Address,
        tag: String,
    ) -> Vec<String> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let mut result = Vec::new(&env);

        for id in attestation_ids.iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &id) {
                if let Some(tags) = attestation.tags {
                    for t in tags.iter() {
                        if t == tag {
                            result.push_back(id.clone());
                            break;
                        }
                    }
                }
            }
        }

        result
    }

    pub fn get_issuer_attestations(
        env: Env,
        issuer: Address,
        start: u32,
        limit: u32,
    ) -> Vec<String> {
        paginate_strings(
            &env,
            Storage::get_issuer_attestations(&env, &issuer),
            start,
            limit,
        )
    }

    pub fn get_valid_claims(env: Env, subject: Address) -> Vec<String> {
        let current_time = env.ledger().timestamp();
        let mut result = Vec::new(&env);

        for attestation_id in Storage::get_subject_attestations(&env, &subject).iter() {
            if let Ok(attestation) = Storage::get_attestation(&env, &attestation_id) {
                if attestation.get_status(current_time) == AttestationStatus::Valid {
                    let mut already_present = false;
                    for existing in result.iter() {
                        if existing == attestation.claim_type {
                            already_present = true;
                            break;
                        }
                    }

                    if !already_present {
                        result.push_back(attestation.claim_type);
                    }
                }
            }
        }

        result
    }

    pub fn get_attestation_by_type(
        env: Env,
        subject: Address,
        claim_type: String,
    ) -> Result<Attestation, Error> {
        let attestation_ids = Storage::get_subject_attestations(&env, &subject);
        let current_time = env.ledger().timestamp();
        let mut index = attestation_ids.len();

        while index > 0 {
            index -= 1;
            if let Some(attestation_id) = attestation_ids.get(index) {
                let attestation = Storage::get_attestation(&env, &attestation_id)?;
                if attestation.claim_type == claim_type
                    && attestation.get_status(current_time) == AttestationStatus::Valid
                {
                    return Ok(attestation);
                }
            }
        }

        Err(Error::NotFound)
    }

    pub fn is_issuer(env: Env, address: Address) -> bool {
        Storage::is_issuer(&env, &address)
    }

    pub fn is_bridge(env: Env, address: Address) -> bool {
        Storage::is_bridge(&env, &address)
    }

    pub fn set_issuer_metadata(
        env: Env,
        issuer: Address,
        metadata: IssuerMetadata,
    ) -> Result<(), Error> {
        issuer.require_auth();
        Validation::require_issuer(&env, &issuer)?;
        Storage::set_issuer_metadata(&env, &issuer, &metadata);
        Ok(())
    }

    pub fn get_issuer_metadata(env: Env, issuer: Address) -> Option<IssuerMetadata> {
        Storage::get_issuer_metadata(&env, &issuer)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        load_fee_config(&env)
    }

    pub fn register_claim_type(
        env: Env,
        admin: Address,
        claim_type: String,
        description: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        Validation::require_admin(&env, &admin)?;

        let info = ClaimTypeInfo {
            claim_type: claim_type.clone(),
            description: description.clone(),
        };
        Storage::set_claim_type(&env, &info);
        Events::claim_type_registered(&env, &claim_type, &description);
        Ok(())
    }

    pub fn get_claim_type_description(env: Env, claim_type: String) -> Option<String> {
        Storage::get_claim_type(&env, &claim_type).map(|info| info.description)
    }

    pub fn list_claim_types(env: Env, start: u32, limit: u32) -> Vec<String> {
        paginate_strings(&env, Storage::get_claim_type_list(&env), start, limit)
    }

    pub fn get_version(env: Env) -> Result<String, Error> {
        Storage::get_version(&env).ok_or(Error::NotInitialized)
    }

    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, Error> {
        let version = Storage::get_version(&env).ok_or(Error::NotInitialized)?;
        Ok(ContractMetadata {
            name: String::from_str(&env, "TrustLink"),
            version,
            description: String::from_str(
                &env,
                "On-chain attestation and verification system for the Stellar blockchain.",
            ),
        })
    }
}

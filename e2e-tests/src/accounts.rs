use aleph_client::{
    keypair_from_string, raw_keypair_from_string, AccountId, KeyPair, Pair, RawKeyPair,
};

use crate::config::Config;

pub fn get_validator_seed(seed: u32) -> String {
    format!("//{}", seed)
}

// this should be extracted to common code
pub fn get_validators_seeds(config: &Config) -> Vec<String> {
    match config.validators_seeds {
        Some(ref seeds) => seeds.clone(),
        None => (0..config.validator_count)
            .map(get_validator_seed)
            .collect(),
    }
}

pub fn get_validators_keys(config: &Config) -> Vec<KeyPair> {
    accounts_seeds_to_keys(&get_validators_seeds(config))
}
pub fn get_validators_raw_keys(config: &Config) -> Vec<RawKeyPair> {
    accounts_seeds_to_raw_keys(&get_validators_seeds(config))
}

pub fn accounts_seeds_to_keys(seeds: &[String]) -> Vec<KeyPair> {
    seeds
        .iter()
        .map(String::as_str)
        .map(keypair_from_string)
        .collect()
}
pub fn accounts_seeds_to_raw_keys(seeds: &[String]) -> Vec<RawKeyPair> {
    seeds
        .iter()
        .map(String::as_str)
        .map(raw_keypair_from_string)
        .collect()
}

pub fn get_sudo_key(config: &Config) -> KeyPair {
    keypair_from_string(&config.sudo_seed)
}

pub struct NodeKeys {
    pub validator: KeyPair,
    pub controller: KeyPair,
}

impl From<String> for NodeKeys {
    fn from(seed: String) -> Self {
        Self {
            validator: keypair_from_string(&seed),
            controller: keypair_from_string(&get_validators_controller_seed(&seed)),
        }
    }
}

fn get_validators_controller_seed(seed: &str) -> String {
    format!("{}//Controller", seed)
}

pub fn account_ids_from_keys(keys: &[KeyPair]) -> Vec<AccountId> {
    keys.iter()
        .map(|pair| AccountId::from(pair.signer().public()))
        .collect()
}

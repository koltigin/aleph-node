pub use crate::aleph_zero::api::runtime_types::*;
use crate::{
    aleph_runtime::SessionKeys,
    api::runtime_types::{
        primitives::app::Public as AlephPublic,
        sp_consensus_aura::sr25519::app_sr25519::Public as AuraPublic,
        sp_core::{ed25519::Public as EdPublic, sr25519::Public as SrPublic},
    },
    frame_support::weights::weight_v2::Weight,
    pallet_staking::EraRewardPoints,
};

impl<AccountId> Default for EraRewardPoints<AccountId> {
    fn default() -> Self {
        Self {
            total: 0,
            individual: vec![],
        }
    }
}

// Manually implementing decoding
impl From<Vec<u8>> for SessionKeys {
    fn from(bytes: Vec<u8>) -> Self {
        assert_eq!(bytes.len(), 64);
        Self {
            aura: AuraPublic(SrPublic(bytes[..32].try_into().unwrap())),
            aleph: AlephPublic(EdPublic(bytes[32..64].try_into().unwrap())),
        }
    }
}

impl TryFrom<String> for SessionKeys {
    type Error = ();

    fn try_from(keys: String) -> Result<Self, Self::Error> {
        let bytes: Vec<u8> = match hex::FromHex::from_hex(keys) {
            Ok(bytes) => bytes,
            Err(_) => return Err(()),
        };
        Ok(SessionKeys::from(bytes))
    }
}

impl Weight {
    pub fn new(ref_time: u64) -> Self {
        Self { ref_time }
    }
}

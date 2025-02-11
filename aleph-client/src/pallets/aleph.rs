use codec::Encode;
use primitives::{BlockNumber, SessionIndex};
use subxt::rpc_params;

use crate::{
    api::runtime_types::{
        pallet_aleph::pallet::Call::set_emergency_finalizer, primitives::app::Public,
        sp_core::ed25519::Public as EdPublic,
    },
    pallet_aleph::pallet::Call::schedule_finality_version_change,
    AccountId, AlephKeyPair, BlockHash,
    Call::Aleph,
    Connection, Pair, RootConnection, SudoCall, TxStatus,
};

#[async_trait::async_trait]
pub trait AlephSudoApi {
    async fn set_emergency_finalizer(
        &self,
        finalizer: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;

    async fn schedule_finality_version_change(
        &self,
        version: u32,
        session: SessionIndex,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash>;
}

#[async_trait::async_trait]
pub trait AlephRpc {
    async fn emergency_finalize(
        &self,
        number: BlockNumber,
        hash: BlockHash,
        key_pair: AlephKeyPair,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl AlephSudoApi for RootConnection {
    async fn set_emergency_finalizer(
        &self,
        finalizer: AccountId,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = Aleph(set_emergency_finalizer {
            emergency_finalizer: Public(EdPublic(finalizer.into())),
        });
        self.sudo_unchecked(call, status).await
    }

    async fn schedule_finality_version_change(
        &self,
        version: u32,
        session: SessionIndex,
        status: TxStatus,
    ) -> anyhow::Result<BlockHash> {
        let call = Aleph(schedule_finality_version_change {
            version_incoming: version,
            session,
        });

        self.sudo_unchecked(call, status).await
    }
}

#[async_trait::async_trait]
impl AlephRpc for Connection {
    async fn emergency_finalize(
        &self,
        number: BlockNumber,
        hash: BlockHash,
        key_pair: AlephKeyPair,
    ) -> anyhow::Result<()> {
        let method = "alephNode_emergencyFinalize";
        let signature = key_pair.sign(&hash.encode());
        let raw_signature: &[u8] = signature.as_ref();
        let params = rpc_params![raw_signature, hash, number];

        let _: () = self.rpc_call(method.to_string(), params).await?;

        Ok(())
    }
}

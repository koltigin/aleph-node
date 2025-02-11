use aleph_client::{
    pallets::treasury::{TreasurySudoApi, TreasuryUserApi},
    AccountId, RootConnection, SignedConnection, TxStatus,
};
use primitives::{Balance, TOKEN};
use subxt::ext::sp_core::crypto::Ss58Codec;

/// Delegates to `aleph_client::make_treasury_proposal`.
pub async fn propose(connection: SignedConnection, amount_in_tokens: u64, beneficiary: String) {
    let beneficiary = AccountId::from_ss58check(&beneficiary).expect("Address should be valid");
    let endowment = amount_in_tokens as Balance * TOKEN;

    connection
        .propose_spend(endowment, beneficiary, TxStatus::Finalized)
        .await
        .unwrap();
}

/// Delegates to `aleph_client::approve_treasury_proposal`.
pub async fn approve(connection: RootConnection, proposal_id: u32) {
    connection
        .approve(proposal_id, TxStatus::Finalized)
        .await
        .unwrap();
}

/// Delegates to `aleph_client::reject_treasury_proposal`.
pub async fn reject(connection: RootConnection, proposal_id: u32) {
    connection
        .reject(proposal_id, TxStatus::Finalized)
        .await
        .unwrap();
}

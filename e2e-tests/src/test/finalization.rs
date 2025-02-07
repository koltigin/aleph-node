use aleph_client::{
    utility::BlocksApi,
    waiting::{AlephWaiting, BlockStatus},
};

use crate::config::Config;

pub async fn finalization(config: &Config) -> anyhow::Result<()> {
    let connection = config.create_root_connection().await;

    let finalized = connection.connection.get_finalized_block_hash().await;
    let finalized_number = connection
        .connection
        .get_block_number(finalized)
        .await
        .unwrap();
    connection
        .connection
        .wait_for_block(|n| n > finalized_number, BlockStatus::Finalized)
        .await;
    Ok(())
}

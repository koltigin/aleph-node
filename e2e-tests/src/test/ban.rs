use aleph_client::{
    pallets::{
        elections::{ElectionsApi, ElectionsSudoApi},
        session::SessionApi,
        staking::StakingApi,
    },
    primitives::{BanInfo, BanReason},
    sp_runtime::bounded::bounded_vec::BoundedVec,
    waiting::{BlockStatus, WaitingExt},
    SignedConnection, TxStatus,
};
use log::info;
use primitives::{
    SessionCount, DEFAULT_BAN_MINIMAL_EXPECTED_PERFORMANCE, DEFAULT_BAN_SESSION_COUNT_THRESHOLD,
    DEFAULT_CLEAN_SESSION_COUNTER_DELAY,
};

use crate::{
    accounts::{get_validator_seed, NodeKeys},
    ban::{
        check_ban_config, check_ban_event, check_ban_info_for_validator,
        check_underperformed_count_for_sessions, check_underperformed_validator_reason,
        check_underperformed_validator_session_count, check_validators, setup_test,
    },
    rewards::set_invalid_keys_for_validator,
    Config,
};

const SESSIONS_TO_CHECK: SessionCount = 5;

const VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX: u32 = 0;
const VALIDATOR_TO_DISABLE_OVERALL_INDEX: u32 = 2;
// Address for //2 (Node2). Depends on the infrastructure setup.
const NODE_TO_DISABLE_ADDRESS: &str = "ws://127.0.0.1:9945";
const SESSIONS_TO_MEET_BAN_THRESHOLD: SessionCount = 4;

const VALIDATOR_TO_MANUALLY_BAN_NON_RESERVED_INDEX: u32 = 1;
const MANUAL_BAN_REASON: &str = "Manual ban reason";
const MIN_EXPECTED_PERFORMANCE: u8 = 100;

async fn disable_validator(validator_address: &str, validator_seed: u32) -> anyhow::Result<()> {
    let validator_seed = get_validator_seed(validator_seed);
    let stash_controller = NodeKeys::from(validator_seed);
    let controller_key_to_disable = stash_controller.controller;

    // This connection has to be set up with the controller key.
    let connection_to_disable =
        SignedConnection::new(validator_address.to_string(), controller_key_to_disable).await;

    set_invalid_keys_for_validator(&connection_to_disable).await
}

/// Runs a chain, sets up a committee and validators. Sets an incorrect key for one of the
/// validators. Waits for the offending validator to hit the ban threshold of sessions without
/// producing blocks. Verifies that the offending validator has in fact been banned out for the
/// appropriate reason.
pub async fn ban_automatic(config: &Config) -> anyhow::Result<()> {
    let (root_connection, reserved_validators, non_reserved_validators, _) =
        setup_test(config).await?;

    // Check current era validators.
    check_validators(
        &reserved_validators,
        &non_reserved_validators,
        root_connection
            .connection
            .get_current_era_validators(None)
            .await,
    );

    check_ban_config(
        &root_connection.connection,
        DEFAULT_BAN_MINIMAL_EXPECTED_PERFORMANCE,
        DEFAULT_BAN_SESSION_COUNT_THRESHOLD,
        DEFAULT_CLEAN_SESSION_COUNTER_DELAY,
    )
    .await;

    let validator_to_disable =
        &non_reserved_validators[VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX as usize];

    info!("Validator to disable: {}", validator_to_disable);

    check_underperformed_validator_session_count(
        &root_connection.connection,
        validator_to_disable,
        0,
    )
    .await;
    check_underperformed_validator_reason(&root_connection.connection, validator_to_disable, None)
        .await;

    disable_validator(NODE_TO_DISABLE_ADDRESS, VALIDATOR_TO_DISABLE_OVERALL_INDEX).await?;

    root_connection
        .connection
        .wait_for_n_sessions(SESSIONS_TO_MEET_BAN_THRESHOLD, BlockStatus::Best)
        .await;

    // The session count for underperforming validators is reset to 0 immediately on reaching the
    // threshold.
    check_underperformed_validator_session_count(
        &root_connection.connection,
        validator_to_disable,
        0,
    )
    .await;

    let reason = BanReason::InsufficientUptime(DEFAULT_BAN_SESSION_COUNT_THRESHOLD);
    let start = root_connection.connection.get_current_era(None).await + 1;
    let expected_ban_info = BanInfo { reason, start };

    check_underperformed_validator_reason(
        &root_connection.connection,
        validator_to_disable,
        Some(&expected_ban_info),
    )
    .await;

    let expected_non_reserved =
        &non_reserved_validators[(VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX + 1) as usize..];

    let expected_banned_validators = vec![(validator_to_disable.clone(), expected_ban_info)];
    check_ban_event(&root_connection.connection, &expected_banned_validators).await?;

    // Check current validators.
    check_validators(
        &reserved_validators,
        expected_non_reserved,
        root_connection
            .connection
            .get_current_era_validators(None)
            .await,
    );

    Ok(())
}

/// Runs a chain, sets up a committee and validators. Manually bans one of the validators
/// from the committee with a specific reason. Verifies that validator marked for ban has in
/// fact been banned for the given reason.
pub async fn ban_manual(config: &Config) -> anyhow::Result<()> {
    let (root_connection, reserved_validators, non_reserved_validators, _) =
        setup_test(config).await?;

    // Check current era validators.
    check_validators(
        &reserved_validators,
        &non_reserved_validators,
        root_connection
            .connection
            .get_current_era_validators(None)
            .await,
    );

    check_ban_config(
        &root_connection.connection,
        DEFAULT_BAN_MINIMAL_EXPECTED_PERFORMANCE,
        DEFAULT_BAN_SESSION_COUNT_THRESHOLD,
        DEFAULT_CLEAN_SESSION_COUNTER_DELAY,
    )
    .await;

    let validator_to_manually_ban =
        &non_reserved_validators[VALIDATOR_TO_MANUALLY_BAN_NON_RESERVED_INDEX as usize];

    info!("Validator to manually ban: {}", validator_to_manually_ban);

    check_underperformed_validator_session_count(
        &root_connection.connection,
        validator_to_manually_ban,
        0,
    )
    .await;
    check_ban_info_for_validator(&root_connection.connection, validator_to_manually_ban, None)
        .await;

    let bounded_reason = BoundedVec(MANUAL_BAN_REASON.as_bytes().to_vec());

    root_connection
        .ban_from_committee(
            validator_to_manually_ban.clone(),
            bounded_reason.0.clone(),
            TxStatus::InBlock,
        )
        .await?;

    let reason = BanReason::OtherReason(bounded_reason);
    let start = root_connection.connection.get_current_era(None).await + 1;
    let expected_ban_info = BanInfo { reason, start };
    check_ban_info_for_validator(
        &root_connection.connection,
        validator_to_manually_ban,
        Some(&expected_ban_info),
    )
    .await;

    let expected_banned_validators = vec![(validator_to_manually_ban.clone(), expected_ban_info)];

    check_ban_event(&root_connection.connection, &expected_banned_validators).await?;

    let expected_non_reserved: Vec<_> = non_reserved_validators
        .clone()
        .into_iter()
        .filter(|account_id| account_id != validator_to_manually_ban)
        .collect();

    // Check current validators.
    check_validators(
        &reserved_validators,
        &expected_non_reserved,
        root_connection
            .connection
            .get_current_era_validators(None)
            .await,
    );

    Ok(())
}

/// Setup validators and non_validators. Set ban config clean_session_counter_delay to 2, while
/// underperformed_session_count_threshold to 3.
/// Disable one non_reserved validator. Check if the disabled validator is still in the committee
/// and his underperformed session count is less or equal to 2.
pub async fn clearing_session_count(config: &Config) -> anyhow::Result<()> {
    let (root_connection, reserved_validators, non_reserved_validators, _) =
        setup_test(config).await?;

    root_connection
        .set_ban_config(None, Some(3), Some(2), None, TxStatus::InBlock)
        .await?;

    let validator_to_disable =
        &non_reserved_validators[VALIDATOR_TO_DISABLE_NON_RESERVED_INDEX as usize];
    info!(target: "aleph-client", "Disabling validator {}", validator_to_disable);
    disable_validator(NODE_TO_DISABLE_ADDRESS, VALIDATOR_TO_DISABLE_OVERALL_INDEX).await?;

    root_connection
        .connection
        .wait_for_n_sessions(5, BlockStatus::Best)
        .await;

    let underperformed_validator_session_count = root_connection
        .connection
        .get_underperformed_validator_session_count(validator_to_disable.clone(), None)
        .await
        .unwrap_or_default();

    // it only has to be ge than 0 and should be cleared before reaching values larger than 3.
    assert!(underperformed_validator_session_count <= 2);

    let next_era_reserved_validators = root_connection
        .connection
        .get_next_era_reserved_validators(None)
        .await;
    let next_era_non_reserved_validators = root_connection
        .connection
        .get_next_era_non_reserved_validators(None)
        .await;

    // checks no one was banned
    assert_eq!(next_era_reserved_validators, reserved_validators);
    assert_eq!(next_era_non_reserved_validators, non_reserved_validators);

    Ok(())
}

/// Runs a chain, sets up a committee and validators. Changes the ban config to require 100%
/// performance. Checks that each validator has all the sessions in which they were chosen for the
/// committee marked as ones in which they underperformed.
pub async fn ban_threshold(config: &Config) -> anyhow::Result<()> {
    let (root_connection, reserved_validators, non_reserved_validators, seats) =
        setup_test(config).await?;

    // Check current era validators.
    check_validators(
        &reserved_validators,
        &non_reserved_validators,
        root_connection
            .connection
            .get_current_era_validators(None)
            .await,
    );

    check_ban_config(
        &root_connection.connection,
        DEFAULT_BAN_MINIMAL_EXPECTED_PERFORMANCE,
        DEFAULT_BAN_SESSION_COUNT_THRESHOLD,
        DEFAULT_CLEAN_SESSION_COUNTER_DELAY,
    )
    .await;

    // Change ban config to require prohibitively high performance from all validators.
    root_connection
        .set_ban_config(
            Some(MIN_EXPECTED_PERFORMANCE),
            None,
            None,
            None,
            TxStatus::InBlock,
        )
        .await?;

    let ban_config_change_session = root_connection.connection.get_session(None).await;
    let check_start_session = ban_config_change_session + 1;
    let check_end_session = check_start_session + SESSIONS_TO_CHECK - 1;
    root_connection
        .connection
        .wait_for_n_sessions(SESSIONS_TO_CHECK, BlockStatus::Finalized)
        .await;

    check_underperformed_count_for_sessions(
        &root_connection.connection,
        &reserved_validators,
        &non_reserved_validators,
        &seats,
        check_start_session,
        check_end_session,
        DEFAULT_BAN_SESSION_COUNT_THRESHOLD,
    )
    .await?;

    Ok(())
}

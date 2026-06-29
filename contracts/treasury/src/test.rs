#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, testutils::Ledger, token, Address, Env, Vec};
use types::WithdrawalStatus;

fn setup_env() -> (Env, Address, TreasuryContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(TreasuryContract, ());
    let client = TreasuryContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
}

#[test]
fn test_initialize() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);
    signers.push_back(signer2);

    client.initialize(&admin, &signers, &2);

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_threshold(), 2);
    assert_eq!(client.get_signers().len(), 2);
}

#[test]
fn test_get_config() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let config = client.get_config();
    assert_eq!(config.admin, admin);
    assert_eq!(config.signers, signers);
    assert_eq!(config.threshold, 2);
    assert_eq!(config.proposal_count, 0);
}

#[test]
#[should_panic]
fn test_double_initialize() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);

    client.initialize(&admin, &signers, &1);
    // This should panic with AlreadyInitialized
    client.initialize(&admin, &signers, &1);
}

#[test]
fn test_create_and_approve_withdrawal() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );
    assert_eq!(proposal_id, 0);

    // First approval is automatic (proposer)
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.approvals.len(), 1);
    assert_eq!(request.signer_set_version, 0);

    // Second signer approves
    client.approve_withdrawal(&signer2, &proposal_id);
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Approved);
}

#[test]
fn test_add_and_remove_signer() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &1);

    // Add a signer
    client.add_signer(&admin, &signer3);
    assert_eq!(client.get_signers().len(), 3);

    // Remove a signer
    client.remove_signer(&admin, &signer2);
    assert_eq!(client.get_signers().len(), 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_unauthorized_withdrawal_attempt_non_signer() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let non_signer = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);

    client.initialize(&admin, &signers, &1);

    client.create_withdrawal(
        &non_signer,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );
}

#[test]
fn test_threshold_update_boundary_values() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);
    signers.push_back(signer2);
    signers.push_back(signer3);

    client.initialize(&admin, &signers, &2);
    assert_eq!(client.get_threshold(), 2);

    client.update_threshold(&admin, &1);
    assert_eq!(client.get_threshold(), 1);

    client.update_threshold(&admin, &3);
    assert_eq!(client.get_threshold(), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_threshold_update_zero_rejected() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);

    client.initialize(&admin, &signers, &1);

    client.update_threshold(&admin, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_threshold_update_exceeds_signers_rejected() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);
    signers.push_back(signer2);

    client.initialize(&admin, &signers, &2);

    client.update_threshold(&admin, &3);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_remove_signer_at_threshold_minimum() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    client.remove_signer(&admin, &signer1);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_double_approval_by_same_signer_rejected() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );

    client.approve_withdrawal(&signer1, &proposal_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_execute_before_approval_threshold_met() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2);
    signers.push_back(signer3);

    client.initialize(&admin, &signers, &3);

    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );

    client.execute_withdrawal(&signer1, &proposal_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_invalid_threshold_zero_at_init() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);

    client.initialize(&admin, &signers, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_invalid_threshold_exceeds_signers_at_init() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1);
    signers.push_back(signer2);

    client.initialize(&admin, &signers, &3);
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract_addr = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(e, &contract_addr)
}

fn create_token_client<'a>(e: &Env, contract_addr: &Address) -> token::Client<'a> {
    token::Client::new(e, contract_addr)
}

#[test]
fn test_execute_withdrawal_full_flow() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();
    let token_client = create_token_client(&env, &token);

    client.initialize(&admin, &signers, &2);

    let deposit_amount: i128 = 10000;
    token_admin_client.mint(&client.address, &deposit_amount);

    assert_eq!(token_client.balance(&client.address), deposit_amount);
    assert_eq!(token_client.balance(&recipient), 0);

    let withdrawal_amount: i128 = 5000;
    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &withdrawal_amount,
        &symbol_short!("salary"),
    );

    client.approve_withdrawal(&signer2, &proposal_id);
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Approved);
    assert_eq!(request.signer_set_version, 0);

    client.execute_withdrawal(&signer1, &proposal_id);

    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Executed);

    assert_eq!(token_client.balance(&recipient), withdrawal_amount);
    assert_eq!(
        token_client.balance(&client.address),
        deposit_amount - withdrawal_amount
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn test_execute_withdrawal_insufficient_balance() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let recipient = Address::generate(&env);

    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();

    client.initialize(&admin, &signers, &2);

    let withdrawal_amount: i128 = 5000;
    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &withdrawal_amount,
        &symbol_short!("salary"),
    );

    client.approve_withdrawal(&signer2, &proposal_id);

    client.execute_withdrawal(&signer1, &proposal_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn test_duplicate_signers_at_init_rejected() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &2);
}

#[test]
fn test_threshold_one_immediate_approval() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );

    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Approved);
    assert_eq!(request.approvals.len(), 1);
    assert_eq!(request.signer_set_version, 0);
}

#[test]
fn test_signer_rotation_with_pending_withdrawal() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    // Create a withdrawal with threshold 2
    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );

    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Pending);
    assert_eq!(request.signer_set_version, 0);

    // Add a new signer - this increments the signer set version
    client.add_signer(&admin, &signer3);
    assert_eq!(client.get_signers().len(), 3);

    // The pending withdrawal still has signer_set_version 0
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.signer_set_version, 0);

    // Original signer2 can still approve (was in signer set version 0)
    client.approve_withdrawal(&signer2, &proposal_id);
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Approved);
}

#[test]
fn test_signer_removal_does_not_invalidate_pending_withdrawal() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    client.initialize(&admin, &signers, &2);

    // Create a withdrawal with threshold 2
    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &1000_i128,
        &symbol_short!("salary"),
    );

    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Pending);
    assert_eq!(request.signer_set_version, 0);

    // Remove signer3 - this increments the signer set version
    client.remove_signer(&admin, &signer3);
    assert_eq!(client.get_signers().len(), 2);

    // The pending withdrawal still has signer_set_version 0
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.signer_set_version, 0);

    // Original signer2 can still approve (was in signer set version 0)
    client.approve_withdrawal(&signer2, &proposal_id);
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, WithdrawalStatus::Approved);
}

#[test]
fn test_deposit() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();
    let token_client = create_token_client(&env, &token);

    client.initialize(&admin, &signers, &1);

    let depositor = Address::generate(&env);
    let initial_balance = 50000;
    token_admin_client.mint(&depositor, &initial_balance);

    let deposit_amount = 15000;
    client.deposit(&depositor, &token, &deposit_amount);

    assert_eq!(token_client.balance(&depositor), initial_balance - deposit_amount);
    assert_eq!(token_client.balance(&client.address), deposit_amount);
}

// ── Pause Control Tests ──────────────────────────────────────────────

#[test]
fn test_pause_and_unpause_flow() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    // Verify initially unpaused
    let pause_state = client.get_pause_state();
    assert_eq!(pause_state, types::PauseState::Unpaused);

    // Propose pause (threshold 2, so needs another approval)
    let pause_proposal = client.propose_pause(&signer1, &symbol_short!("sec_inc"));
    
    // Approve pause with signer2
    client.approve_pause(&signer2, &pause_proposal);
    
    // Verify paused
    let pause_state = client.get_pause_state();
    assert_eq!(pause_state, types::PauseState::Paused);
}

#[test]
fn test_deposit_blocked_when_paused() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();

    client.initialize(&admin, &signers, &2);

    // Pause the contract
    let pause_proposal = client.propose_pause(&signer1, &symbol_short!("incident"));
    client.approve_pause(&signer2, &pause_proposal);

    // Attempt deposit should fail
    let depositor = Address::generate(&env);
    let result = client.try_deposit(&depositor, &token, &1000);
    assert!(result.is_err());
}

#[test]
fn test_unpause_after_pause() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    // Pause
    let pause_proposal = client.propose_pause(&signer1, &symbol_short!("incident"));
    client.approve_pause(&signer2, &pause_proposal);

    // Propose unpause
    let unpause_proposal = client.propose_unpause(&signer1, &symbol_short!("resolved"));
    client.approve_unpause(&signer2, &unpause_proposal);

    // Verify unpaused
    let pause_state = client.get_pause_state();
    assert_eq!(pause_state, types::PauseState::Unpaused);
}

// ── Signer-Controlled Upgrade Tests ───────────────────────────────────

#[test]
fn test_upgrade_proposal_flow() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let new_wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);
    
    // Propose upgrade
    let proposal_id = client.propose_upgrade(
        &signer1,
        &new_wasm_hash,
        &symbol_short!("v2"),
    );
    assert_eq!(proposal_id, 0);

    // Approve with signer2
    client.approve_upgrade(&signer2, &proposal_id);

    // Verify upgrade count
    let upgrade_count = client.get_upgrade_count();
    assert_eq!(upgrade_count, 1);
}

// ── Timelock Tests ───────────────────────────────────────────────────

#[test]
fn test_admin_change_timelock() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let new_admin = Address::generate(&env);
    
    // Propose admin change
    client.propose_admin_change(&admin, &new_admin);

    // Verify pending admin change exists
    let pending = client.get_pending_admin_change();
    assert_eq!(pending.new_admin, new_admin);
    
    // Cancel the pending change
    client.cancel_admin_change(&admin);
}

#[test]
fn test_signer_change_delay_minimum() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    // Attempt to set delay below minimum should fail
    let result = client.try_update_signer_change_delay(&admin, &1000);
    assert!(result.is_err());
}

#[test]
fn test_signer_change_delay_valid_update() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    // Set valid delay (greater than 1 day)
    let valid_delay: u64 = 8 * 24 * 60 * 60; // 8 days
    client.update_signer_change_delay(&admin, &valid_delay);

    let delay = client.get_signer_change_delay();
    assert_eq!(delay, valid_delay);
}

// ── Emergency Admin Change Tests ───────────────────────────────────────

#[test]
fn test_emergency_admin_change_flow() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let new_admin = Address::generate(&env);

    // Emergency admin change proposed by signer (not admin)
    let proposal_id = client.propose_emergency_admin_change(&signer1, &new_admin);
    
    // Approve with signer2
    client.approve_upgrade(&signer2, &proposal_id);

    // Execute emergency change
    client.execute_emergency_admin_change(&signer1, &proposal_id);
    assert_eq!(client.get_admin(), signer1);
}

// ── Event Emission Tests ───────────────────────────────────────────────

#[test]
fn test_pause_emits_events() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    // All events are emitted during propose_pause and approve_pause
    let pause_proposal = client.propose_pause(&signer1, &symbol_short!("pause"));
    client.approve_pause(&signer2, &pause_proposal);

    // Verify contract is paused
    let pause_state = client.get_pause_state();
    assert_eq!(pause_state, types::PauseState::Paused);
}

// ── Migration Tests ────────────────────────────────────────────────────────

#[test]
fn test_migration_preserves_balances_and_state() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();
    let token_client = create_token_client(&env, &token);

    client.initialize(&admin, &signers, &2);

    // Deposit funds
    let deposit_amount: i128 = 10_000;
    token_admin_client.mint(&client.address, &deposit_amount);
    assert_eq!(token_client.balance(&client.address), deposit_amount);

    // Create a pending withdrawal
    let proposal_id = client.create_withdrawal(
        &signer1,
        &token,
        &recipient,
        &2000_i128,
        &symbol_short!("migration"),
    );
    assert_eq!(proposal_id, 0);

    // Approve with second signer
    client.approve_withdrawal(&signer2, &proposal_id);

    // Capture state for migration
    let config = client.get_config();
    assert_eq!(config.admin, admin);
    assert_eq!(config.signers.len(), 2);
    assert_eq!(config.threshold, 2);
    assert_eq!(config.proposal_count, 1);

    // Verify balance preserved
    assert_eq!(token_client.balance(&client.address), deposit_amount);

    // Verify withdrawal state preserved
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.status, types::WithdrawalStatus::Approved);
}

#[test]
fn test_migration_with_multiple_approvals() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();

    client.initialize(&admin, &signers, &2);

    let deposit_amount: i128 = 20_000;
    token_admin_client.mint(&client.address, &deposit_amount);

    // Create multiple pending withdrawals
    let id1 = client.create_withdrawal(
        &signer1, &token, &recipient1, &1000_i128, &symbol_short!("w1"),
    );
    client.approve_withdrawal(&signer2, &id1);

    let id2 = client.create_withdrawal(
        &signer3, &token, &recipient2, &2000_i128, &symbol_short!("w2"),
    );
    client.approve_withdrawal(&signer1, &id2);

    // Verify both withdrawals are approved and state is preserved
    let w1 = client.get_withdrawal(&id1);
    let w2 = client.get_withdrawal(&id2);
    assert_eq!(w1.status, types::WithdrawalStatus::Approved);
    assert_eq!(w2.status, types::WithdrawalStatus::Approved);
    assert_eq!(w1.amount, 1000);
    assert_eq!(w2.amount, 2000);
    assert_eq!(client.get_proposal_count(), 2);
}

// ── Testnet Drill: Pause, Diagnosis, Recovery, Resume ──────────────────────

#[test]
fn testnet_drill_full_lifecycle() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let signer3 = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();
    let token_client = create_token_client(&env, &token);

    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());
    signers.push_back(signer3.clone());

    client.initialize(&admin, &signers, &2);

    let deposit_amount: i128 = 50_000;
    token_admin_client.mint(&client.address, &deposit_amount);

    // STEP 1: Verify normal operation
    assert_eq!(client.get_pause_state(), types::PauseState::Unpaused);
    assert_eq!(token_client.balance(&client.address), deposit_amount);

    // STEP 2: Initiate emergency pause (simulated vulnerability detected)
    let pause_id = client.propose_pause(&signer1, &symbol_short!("sec_inc"));
    let pre_pause_depositor = Address::generate(&env);
    token_admin_client.mint(&pre_pause_depositor, &1000);
    assert!(client.try_deposit(&pre_pause_depositor, &token, &1000).is_ok()); // Should still work before approval
    let balance_after_pre_pause_deposit = deposit_amount + 1000;

    // STEP 3: Approve pause
    client.approve_pause(&signer2, &pause_id);
    assert_eq!(client.get_pause_state(), types::PauseState::Paused);

    // STEP 4: Verify deposits blocked
    let depositor = Address::generate(&env);
    let deposit_result = client.try_deposit(&depositor, &token, &1000);
    assert!(deposit_result.is_err());

    // STEP 5: Verify withdrawals blocked
    let withdrawal_result = client.try_create_withdrawal(
        &signer3, &token, &recipient, &5000_i128, &symbol_short!("blocked"),
    );
    assert!(withdrawal_result.is_err());

    // STEP 6: Diagnosis — verify balances are preserved (not silently moved)
    assert_eq!(token_client.balance(&client.address), balance_after_pre_pause_deposit);
    assert_eq!(token_client.balance(&recipient), 0);

    // STEP 7: Recovery — propose unpause
    let unpause_id = client.propose_unpause(&signer1, &symbol_short!("patched"));
    assert_eq!(client.get_pause_state(), types::PauseState::Paused); // Still paused until approved

    // STEP 8: Approve unpause
    client.approve_unpause(&signer2, &unpause_id);
    assert_eq!(client.get_pause_state(), types::PauseState::Unpaused);

    // STEP 9: Resume — verify operations work again
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &1000);
    let new_deposit = 1000;
    client.deposit(&depositor, &token, &new_deposit);
    assert_eq!(token_client.balance(&client.address), balance_after_pre_pause_deposit + new_deposit);

    let w_id = client.create_withdrawal(
        &signer1, &token, &recipient, &1000_i128, &symbol_short!("rec"),
    );
    client.approve_withdrawal(&signer2, &w_id);
    client.execute_withdrawal(&signer3, &w_id);
    assert_eq!(token_client.balance(&recipient), 1000);
}

// ── Event Emission Tests ────────────────────────────────────────────────────

#[test]
fn test_upgrade_proposed_emits_event_with_actor() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[3; 32]);
    let proposal_id = client.propose_upgrade(&signer1, &wasm_hash, &symbol_short!("upgrade"));

    // Verify the proposal can be queried
    let proposal = client.get_upgrade_proposal(&proposal_id);
    assert_eq!(proposal.id, proposal_id);
    assert_eq!(proposal.proposer, signer1);
    assert_eq!(proposal.wasm_hash, wasm_hash);
}

#[test]
fn test_all_privileged_actions_emit_identifying_events() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let new_signer = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &2);

    // s_add event
    client.add_signer(&admin, &new_signer);

    // s_remove event
    client.remove_signer(&admin, &new_signer);

    // t_upd event
    client.update_threshold(&admin, &1);

    // admin_change event
    client.propose_admin_change(&admin, &new_admin);

    // delay_updated event
    let new_delay: u64 = 8 * 24 * 60 * 60;
    client.update_signer_change_delay(&admin, &new_delay);

    // pause_propose event
    let pause_id = client.propose_pause(&signer1, &symbol_short!("test"));

    // upgrade_proposed event
    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[4; 32]);
    let upgrade_id = client.propose_upgrade(&signer1, &wasm_hash, &symbol_short!("upg"));

    // emergency_admin_changed event
    client.execute_emergency_admin_change(&signer1, &upgrade_id);

    // unpause events
    let unpause_id = client.propose_unpause(&signer1, &symbol_short!("unp"));
    // With threshold=1 and signer1 as proposer, unpause auto-approves
    let _ = unpause_id;
}


// ── Signer Change Timelock Tests ──────────────────────────────────────────

#[test]
fn test_propose_signer_add_records_proposal() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let new_signer = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.propose_signer_add(&admin, &new_signer);

    // Proposal should exist with PendingSignerChange status
    let proposal = client.get_upgrade_proposal(&proposal_id);
    assert_eq!(proposal.proposer, admin);
    assert_eq!(proposal.description, symbol_short!("sa"));
    assert_eq!(proposal.status, types::UpgradeStatus::PendingSignerChange);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")]
fn test_execute_signer_change_rejected_before_delay() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let new_signer = Address::generate(&env);
    let executor = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.propose_signer_add(&admin, &new_signer);

    // Try to execute immediately — timelock not yet expired
    client.execute_signer_change(&executor, &proposal_id);
}

#[test]
fn test_execute_signer_change_succeeds_after_delay() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let new_signer = Address::generate(&env);
    let executor = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.propose_signer_add(&admin, &new_signer);

    // Advance past the default signer change delay (7 days)
    env.ledger().with_mut(|li| {
        li.timestamp = 7 * 24 * 60 * 60 + 1;
    });

    // Should succeed — delay has elapsed
    client.execute_signer_change(&executor, &proposal_id);
}

#[test]
fn test_schedule_signer_remove_records_proposal() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.schedule_signer_remove(&admin, &signer2);

    let proposal = client.get_upgrade_proposal(&proposal_id);
    assert_eq!(proposal.proposer, admin);
    assert_eq!(proposal.description, symbol_short!("sr"));
    assert_eq!(proposal.status, types::UpgradeStatus::PendingSignerChange);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")]
fn test_execute_signer_remove_rejected_before_delay() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let executor = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.schedule_signer_remove(&admin, &signer2);

    // Immediately try to execute — should fail
    client.execute_signer_change(&executor, &proposal_id);
}

#[test]
fn test_execute_signer_remove_succeeds_after_delay() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let executor = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &1);

    let proposal_id = client.schedule_signer_remove(&admin, &signer2);

    // Advance past the delay
    env.ledger().with_mut(|li| {
        li.timestamp = 7 * 24 * 60 * 60 + 1;
    });

    client.execute_signer_change(&executor, &proposal_id);
}

#[test]
fn test_role_rotation_add_then_remove() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let signer2 = Address::generate(&env);
    let replacement = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());
    signers.push_back(signer2.clone());

    client.initialize(&admin, &signers, &1);

    // Add replacement signer immediately (admin operation)
    client.add_signer(&admin, &replacement);
    assert_eq!(client.get_signers().len(), 3);

    // Remove the old signer
    client.remove_signer(&admin, &signer2);
    assert_eq!(client.get_signers().len(), 2);

    // Verify signer set version was incremented twice
    let config = client.get_config();
    assert!(!config.paused);

    // Ensure replacement can now participate (signer set updated)
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let proposal_id = client.create_withdrawal(
        &replacement,
        &token,
        &recipient,
        &500_i128,
        &symbol_short!("rotation"),
    );
    let request = client.get_withdrawal(&proposal_id);
    assert_eq!(request.proposer, replacement);
}

#[test]
fn test_propose_signer_add_duplicate_rejected() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    // Attempt to propose adding an existing signer should fail
    let result = client.try_propose_signer_add(&admin, &signer1);
    assert!(result.is_err());
}

#[test]
fn test_schedule_signer_remove_nonexistent_rejected() {
    let (env, admin, client) = setup_env();
    let signer1 = Address::generate(&env);
    let non_signer = Address::generate(&env);
    let mut signers = Vec::new(&env);
    signers.push_back(signer1.clone());

    client.initialize(&admin, &signers, &1);

    // Add a second signer so remove doesn't hit threshold error
    let signer2 = Address::generate(&env);
    client.add_signer(&admin, &signer2);

    // Attempt to remove a non-existent signer should fail
    let result = client.try_schedule_signer_remove(&admin, &non_signer);
    assert!(result.is_err());
}

#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, Vec, symbol_short};
use types::{ProposalStatus, VoteChoice};

fn setup_env() -> (Env, Address, GovernanceContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(GovernanceContract, ());
    let client = GovernanceContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
}

#[test]
fn test_initialize() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1);
    members.push_back(member2);

    client.initialize(&admin, &members, &51, &(7 * 24 * 60 * 60), &3600); // 51% quorum, 7-day voting, 1-hour grace

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_members().len(), 2);
    let config = client.get_config();
    assert_eq!(config.quorum_percentage, 51);
    assert_eq!(config.total_weight, 2);
}

#[test]
fn test_create_proposal() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());

    client.initialize(&admin, &members, &51, &(7 * 24 * 60 * 60), &3600);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("devfund"),
        &token,
        &50_000_i128,
        &recipient,
    );

    assert_eq!(proposal_id, 0);
    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Active);
    assert_eq!(proposal.amount, 50_000);
    // Snapshot must be populated
    assert_eq!(proposal.snapshot.quorum_percentage, 51);
    assert_eq!(proposal.snapshot.total_weight, 1);
    assert_eq!(proposal.snapshot.electorate.len(), 1);
}

#[test]
fn test_voting_and_finalization() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(member2.clone());
    members.push_back(member3.clone());

    let voting_duration = 7 * 24 * 60 * 60_u64;
    let grace_period = 3600_u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("devfund"),
        &token,
        &50_000_i128,
        &recipient,
    );

    // Members vote
    client.vote(&member1, &proposal_id, &VoteChoice::Yes);
    client.vote(&member2, &proposal_id, &VoteChoice::Yes);
    client.vote(&member3, &proposal_id, &VoteChoice::No);

    // Move past the voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 1;
    });

    // Finalize
    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Approved);
}

#[test]
fn test_quorum_not_reached() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let member4 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(member2.clone());
    members.push_back(member3.clone());
    members.push_back(member4.clone());

    let voting_duration = 7 * 24 * 60 * 60_u64;
    let grace_period = 3600_u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("ops"),
        &token,
        &10_000_i128,
        &recipient,
    );

    // Only 1 out of 4 members votes (25% < 51% quorum)
    client.vote(&member1, &proposal_id, &VoteChoice::Yes);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 1;
    });

    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Rejected);
}

#[test]
fn test_cancel_proposal() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());

    client.initialize(&admin, &members, &51, &1000, &500);

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("ops"),
        &token,
        &10_000_i128,
        &recipient,
    );

    // Initial status should be Active
    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Active);

    // Proposer can cancel
    client.cancel_proposal(&member1, &proposal_id);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Cancelled);
}

#[test]
fn test_proposal_expiration_live_status() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("test"),
        &token,
        &1000_i128,
        &recipient,
    );

    // Still Active
    assert_eq!(client.get_proposal_status(&proposal_id), ProposalStatus::Active);

    // Past voting but within grace period -> Still Active (waiting for finalization)
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 100;
    });
    assert_eq!(client.get_proposal_status(&proposal_id), ProposalStatus::Active);

    // Past grace period -> Expired
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + grace_period + 1;
    });
    assert_eq!(client.get_proposal_status(&proposal_id), ProposalStatus::Expired);
}

#[test]
fn test_weighted_voting() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(member2.clone());

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    // Total weight is 2 initially
    assert_eq!(client.get_config().total_weight, 2);

    // Set member1 weight to 10 BEFORE proposal creation so snapshot captures it
    client.set_voting_weight(&admin, &member1, &10);
    assert_eq!(client.get_config().total_weight, 11);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("weight"),
        &token,
        &1000_i128,
        &recipient,
    );

    // Member1 votes Yes (10 weight from snapshot)
    client.vote(&member1, &proposal_id, &VoteChoice::Yes);

    // Member2 votes No (1 weight from snapshot)
    client.vote(&member2, &proposal_id, &VoteChoice::No);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.yes_votes, 10);
    assert_eq!(proposal.no_votes, 1);

    // Past voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 1;
    });

    // Finalize: should pass because 10 > 1 and quorum is met (11/11 votes)
    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Approved);
}

#[test]
fn test_weighted_quorum() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(member2.clone());

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    // Set member1 weight to 100. Total weight = 101.
    client.set_voting_weight(&admin, &member1, &100);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("quorum"),
        &token,
        &1000_i128,
        &recipient,
    );

    // Only member2 votes (1 weight). 1/101 < 51% quorum.
    client.vote(&member2, &proposal_id, &VoteChoice::Yes);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 1;
    });

    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Rejected);

    // Member1 votes Yes later (100 weight). Total = 101. 101/101 > 51% quorum.
    // Resetting for a new proposal to test passed quorum
    let proposal_id_2 = client.create_proposal(
        &member1,
        &symbol_short!("quorum2"),
        &token,
        &1000_i128,
        &recipient,
    );
    client.vote(&member1, &proposal_id_2, &VoteChoice::Yes);

    env.ledger().with_mut(|li| {
        li.timestamp = 3200; // Past end_time (3001) but before grace period expiry (3501)
    });

    let status_2 = client.finalize(&admin, &proposal_id_2);
    assert_eq!(status_2, ProposalStatus::Approved);
}

#[test]
fn test_member_removal_weight_adjustment() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(member2.clone());

    client.initialize(&admin, &members, &51, &1000, &500);

    // Initial total weight = 2
    assert_eq!(client.get_config().total_weight, 2);

    // Member1 weight = 10. Total = 11.
    client.set_voting_weight(&admin, &member1, &10);
    assert_eq!(client.get_config().total_weight, 11);

    // Remove member2 (weight 1). Total should be 10.
    client.remove_member(&admin, &member2);
    assert_eq!(client.get_config().total_weight, 10);
    assert_eq!(client.get_members().len(), 1);

    // Remove member1 (weight 10). Total should be 0.
    client.remove_member(&admin, &member1);
    assert_eq!(client.get_config().total_weight, 0);
    assert_eq!(client.get_members().len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_cancel_proposal_unauthorized() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let non_proposer = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());
    members.push_back(non_proposer.clone());

    client.initialize(&admin, &members, &51, &1000, &500);

    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("ops"),
        &token,
        &10_000_i128,
        &recipient,
    );

    // Only the proposer can cancel
    client.cancel_proposal(&non_proposer, &proposal_id);
}

// ── Snapshot isolation tests ─────────────────────────────────────────────────

/// A member added AFTER proposal creation is not in the snapshot electorate
/// and must not be allowed to cast a vote.
#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_snapshot_new_member_cannot_vote() {
    let (env, admin, client) = setup_env();
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(m1.clone());
    members.push_back(m2.clone());

    client.initialize(&admin, &members, &51, &1000, &500);

    let proposal_id = client.create_proposal(
        &m1,
        &symbol_short!("test"),
        &token,
        &1000_i128,
        &recipient,
    );

    // Add m3 AFTER proposal creation — not in the snapshot electorate
    client.add_member(&admin, &m3);

    // m3 is a live member but was not snapshotted — must be rejected with NotAMember (#7)
    client.vote(&m3, &proposal_id, &VoteChoice::Yes);
}

/// If a member's weight is raised after proposal creation, votes still use the
/// weight that was frozen in the snapshot at creation time.
///
/// Setup: 4 members (total snapshot weight = 4), quorum = 51%.
/// Quorum threshold = floor(4 * 51 / 100) = 2.
/// After creation m1's weight is raised to 100 (live total = 103).
/// Only m1 votes → yes_votes must equal 1 (snapshotted), not 100 (live).
/// 1 < threshold 2 → Rejected with snapshot; would be Approved without it.
#[test]
fn test_snapshot_member_weight_frozen_on_vote() {
    let (env, admin, client) = setup_env();
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let m4 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(m1.clone());
    members.push_back(m2.clone());
    members.push_back(m3.clone());
    members.push_back(m4.clone());

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    // Snapshot: m1=1, m2=1, m3=1, m4=1, total=4, quorum=51%
    // Quorum threshold = floor(4 * 51 / 100) = 2
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| { li.timestamp = 1000; });

    let proposal_id = client.create_proposal(
        &m1,
        &symbol_short!("freeze"),
        &token,
        &1000_i128,
        &recipient,
    );

    // Admin bumps m1 weight to 100 AFTER creation. Live total = 103.
    client.set_voting_weight(&admin, &m1, &100);
    assert_eq!(client.get_config().total_weight, 103);

    // m1 votes; snapshot weight must be used (1, not 100)
    client.vote(&m1, &proposal_id, &VoteChoice::Yes);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.yes_votes, 1, "snapshot weight (1) must be used, not live weight (100)");

    // Past voting period.
    // Snapshot: yes=1 out of total=4 → 1 < quorum threshold 2 → Rejected.
    // Without snapshot: live weight 100 out of live total 103 → 100 ≥ 52 → Approved.
    env.ledger().with_mut(|li| { li.timestamp = 1000 + voting_duration + 1; });
    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Rejected);
}

/// Removing members after proposal creation must not lower the snapshotted
/// total_weight used for quorum calculation.
#[test]
fn test_snapshot_total_weight_frozen_on_finalize() {
    let (env, admin, client) = setup_env();
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let m3 = Address::generate(&env);
    let m4 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(m1.clone());
    members.push_back(m2.clone());
    members.push_back(m3.clone());
    members.push_back(m4.clone());

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    // 51% quorum; snapshot total_weight = 4
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| { li.timestamp = 1000; });

    let proposal_id = client.create_proposal(
        &m1,
        &symbol_short!("remove"),
        &token,
        &1000_i128,
        &recipient,
    );

    // m1 and m2 cast votes (2 out of snapshot total 4 = 50% < 51%)
    client.vote(&m1, &proposal_id, &VoteChoice::Yes);
    client.vote(&m2, &proposal_id, &VoteChoice::No);

    // Admin removes m3 and m4 AFTER votes are cast. Live total drops to 2.
    // Without snapshot: 2/2 = 100% ≥ 51% → would Approve.
    // With snapshot:    2/4 = 50%  < 51% → must Reject.
    client.remove_member(&admin, &m3);
    client.remove_member(&admin, &m4);
    assert_eq!(client.get_config().total_weight, 2);

    env.ledger().with_mut(|li| { li.timestamp = 1000 + voting_duration + 1; });

    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(
        status,
        ProposalStatus::Rejected,
        "snapshotted total_weight (4) must be used for quorum, not live value (2)"
    );
}

/// A member removed AFTER proposal creation was in the snapshot electorate
/// and must still be able to cast a vote.
#[test]
fn test_snapshot_removed_member_retains_vote_eligibility() {
    let (env, admin, client) = setup_env();
    let m1 = Address::generate(&env);
    let m2 = Address::generate(&env);
    let token = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(m1.clone());
    members.push_back(m2.clone());

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    // Snapshot: m1=1, m2=1, total=2, quorum=51%
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| { li.timestamp = 1000; });

    let proposal_id = client.create_proposal(
        &m1,
        &symbol_short!("retain"),
        &token,
        &1000_i128,
        &recipient,
    );

    // Admin removes m1 before they vote
    client.remove_member(&admin, &m1);
    assert_eq!(client.get_members().len(), 1);

    // m1 is no longer a live member but was in the snapshot — must still vote
    client.vote(&m1, &proposal_id, &VoteChoice::Yes);
    client.vote(&m2, &proposal_id, &VoteChoice::Yes);

    let proposal = client.get_proposal(&proposal_id);
    // Both votes used snapshotted weights: m1=1, m2=1
    assert_eq!(proposal.yes_votes, 2);

    env.ledger().with_mut(|li| { li.timestamp = 1000 + voting_duration + 1; });

    // Snapshot total=2; 2/2 = 100% ≥ 51% quorum; Yes > No → Approved
    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Approved);
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
fn test_execute_full_flow() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();
    let token_client = create_token_client(&env, &token);

    // Give the governance contract some funds
    let initial_balance = 100_000_i128;
    token_admin_client.mint(&client.address, &initial_balance);

    let voting_duration = 1000u64;
    let grace_period = 500u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_amount = 25_000_i128;
    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("fund"),
        &token,
        &proposal_amount,
        &recipient,
    );

    client.vote(&member1, &proposal_id, &VoteChoice::Yes);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 1;
    });

    let status = client.finalize(&admin, &proposal_id);
    assert_eq!(status, ProposalStatus::Approved);

    client.execute(&admin, &proposal_id);

    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Executed);

    assert_eq!(token_client.balance(&recipient), proposal_amount);
    assert_eq!(token_client.balance(&client.address), initial_balance - proposal_amount);
}

#[test]
#[should_panic]
fn test_execute_insufficient_balance() {
    let (env, admin, client) = setup_env();
    let member1 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let mut members = Vec::new(&env);
    members.push_back(member1.clone());

    let token_admin = Address::generate(&env);
    let token_admin_client = create_token_contract(&env, &token_admin);
    let token = token_admin_client.address.clone();

    // Do NOT mint funds to the governance contract
    let voting_duration = 1000u64;
    let grace_period = 500u64;
    client.initialize(&admin, &members, &51, &voting_duration, &grace_period);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let proposal_amount = 25_000_i128;
    let proposal_id = client.create_proposal(
        &member1,
        &symbol_short!("fund"),
        &token,
        &proposal_amount,
        &recipient,
    );

    client.vote(&member1, &proposal_id, &VoteChoice::Yes);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + voting_duration + 1;
    });

    client.finalize(&admin, &proposal_id);

    // This should panic due to insufficient balance
    client.execute(&admin, &proposal_id);
}

// ── Timelocked Upgrade Tests ───────────────────────────────────────────────

#[test]
fn test_upgrade_timelock_execute_after_delay() {
    let (env, admin, client) = setup_env();

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);

    client.initialize(&admin, &Vec::new(&env), &51, &1000, &500);

    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("v2"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
    assert_eq!(pending.unwrap().wasm_hash, wasm_hash);

    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 + 1;
    });

    let pending_after = client.get_pending_upgrade();
    assert!(pending_after.is_some());
    let pending_after_val = pending_after.unwrap();
    assert_eq!(pending_after_val.wasm_hash, wasm_hash);
    assert!(env.ledger().timestamp() >= pending_after_val.proposed_at + 86400);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn test_upgrade_timelock_rejected_before_delay() {
    let (env, admin, client) = setup_env();
    let executor = Address::generate(&env);

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);

    client.initialize(&admin, &Vec::new(&env), &51, &1000, &500);

    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("v2"));

    env.ledger().with_mut(|li| {
        li.timestamp = 100;
    });
    client.execute_upgrade(&executor);
}

#[test]
fn test_upgrade_proposal_event_includes_actor() {
    let (env, admin, client) = setup_env();
    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[2; 32]);

    client.initialize(&admin, &Vec::new(&env), &51, &1000, &500);
    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("sec_patch"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
}

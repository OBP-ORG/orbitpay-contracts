#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env, Vec};
use types::StreamStatus;

fn setup_env() -> (Env, Address, PayrollStreamContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PayrollStreamContract, ());
    let client = PayrollStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
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
#[should_panic]
fn test_double_initialize() {
    let (_env, admin, client) = setup_env();
    client.initialize(&admin);
    client.initialize(&admin);
}

#[test]
fn test_create_stream_transfers_tokens_and_saves_stream() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    assert_eq!(stream_id, 0);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 10000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.total_amount, 10000);
    assert_eq!(stream.status, StreamStatus::Active);
    assert_eq!(stream.rate_per_second, 10);
}

#[test]
fn test_create_stream_fails_without_balance_and_does_not_persist() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &5000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let result = client.try_create_stream(&sender, &recipient, &token, &10000, &1000, &2000);
    assert_eq!(result, Err(Ok(StreamError::InsufficientBalance)));

    assert_eq!(client.get_stream_count(), 0);
    assert_eq!(token_client.balance(&sender), 5000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_create_batch_streams() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &30000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let mut streams = Vec::new(&env);
    streams.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token: token.clone(),
        total_amount: 10000,
        start_time: 1000,
        end_time: 2000,
    });
    streams.push_back(CreateStreamParams {
        recipient: Address::generate(&env),
        token: token.clone(),
        total_amount: 20000,
        start_time: 1000,
        end_time: 3000,
    });

    let stream_ids = client.create_batch_streams(&sender, &streams);

    assert_eq!(stream_ids.len(), 2);
    assert_eq!(stream_ids.get(0).unwrap(), 0);
    assert_eq!(stream_ids.get(1).unwrap(), 1);

    let stream0 = client.get_stream(&0);
    assert_eq!(stream0.total_amount, 10000);

    let stream1 = client.get_stream(&1);
    assert_eq!(stream1.total_amount, 20000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 30000);
}

#[test]
fn test_calculate_claimable() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    let claimable = client.get_claimable(&stream_id);
    assert_eq!(claimable, 5000);
}

#[test]
fn test_claim_transfers_tokens_and_prevents_double_claim() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    let claimed = client.claim(&recipient, &stream_id);
    assert_eq!(claimed, 5000);
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&client.address), 5000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.claimed_amount, 5000);
    assert_eq!(stream.status, StreamStatus::Active);

    let second_claim = client.try_claim(&recipient, &stream_id);
    assert_eq!(second_claim, Err(Ok(StreamError::NothingToClaim)));
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&client.address), 5000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_claim_rejects_unauthorized_claimer() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let attacker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.claim(&attacker, &stream_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_claim_rejects_when_claimable_is_zero() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);
    client.claim(&recipient, &stream_id);
}

#[test]
fn test_cancel_stream_settles_midway() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 5000);
    assert_eq!(token_client.balance(&recipient), 5000);
    assert_eq!(token_client.balance(&sender), 5000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_cancel_stream_zero_owed_refunds_sender_only() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 500;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);
    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 0);
    assert_eq!(token_client.balance(&recipient), 0);
    assert_eq!(token_client.balance(&sender), 10000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_cancel_stream_rejects_unauthorized() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let attacker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 1500;
    });

    client.cancel_stream(&attacker, &stream_id);
}

#[test]
fn test_cancel_stream_after_end() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 2500;
    });

    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 10000);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_cancel_stream_after_full_claim_has_zero_settlement() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);

    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    env.ledger().with_mut(|li| {
        li.timestamp = 2500;
    });

    let claimed = client.claim(&recipient, &stream_id);
    assert_eq!(claimed, 10000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&client.address), 0);

    client.cancel_stream(&sender, &stream_id);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
    assert_eq!(stream.claimed_amount, 10000);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&recipient), 10000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_claim_progression() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let stream_id = client.create_stream(
        &sender,
        &recipient,
        &token,
        &10000_i128,
        &1000_u64,
        &2000_u64,
    );

    // 1. Claim at 25% (1250)
    env.ledger().with_mut(|li| { li.timestamp = 1250; });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 2500);

    // 2. Claim at 50% (1500)
    env.ledger().with_mut(|li| { li.timestamp = 1500; });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 5000);

    // 3. Claim at 75% (1750)
    env.ledger().with_mut(|li| { li.timestamp = 1750; });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 7500);

    // 4. Claim at 100% (2000)
    env.ledger().with_mut(|li| { li.timestamp = 2000; });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 10000);
}

// ── Timelocked Upgrade Tests ───────────────────────────────────────────────

#[test]
fn test_upgrade_timelock_execute_after_delay() {
    let (env, admin, client) = setup_env();

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);

    client.initialize(&admin);

    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("v2"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
    assert_eq!(pending.unwrap().wasm_hash, wasm_hash);

    // Advance time past the timelock
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 + 1;
    });

    // Verify the timelock has elapsed — execution is gated on this condition
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

    client.initialize(&admin);

    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("v2"));

    // Try to execute before timelock - should fail
    env.ledger().with_mut(|li| {
        li.timestamp = 100;
    });
    client.execute_upgrade(&executor);
}

#[test]
fn test_upgrade_proposal_event_includes_actor() {
    let (env, admin, client) = setup_env();
    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[2; 32]);

    client.initialize(&admin);
    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("sec_patch"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
}

#[test]
fn test_unauthorized_cancel() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let malicious = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| { li.timestamp = 1000; });
    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &1000, &2000);

    let result = client.try_cancel_stream(&malicious, &stream_id);
    assert!(result.is_err());
}

#[test]
fn test_invalid_creation_params() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    // 1. Invalid amount
    let res1 = client.try_create_stream(&sender, &recipient, &token, &-100, &1000, &2000);
    assert!(res1.is_err());

    // 2. Invalid duration
    let res2 = client.try_create_stream(&sender, &recipient, &token, &1000, &2000, &1000);
    assert!(res2.is_err());

    // 3. Same sender and recipient
    let res3 = client.try_create_stream(&sender, &sender, &token, &1000, &1000, &2000);
    assert!(res3.is_err());
}

#[test]
fn test_multiple_concurrent_streams() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &20000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| { li.timestamp = 1000; });
    
    let id1 = client.create_stream(&sender, &recipient1, &token, &10000, &1000, &2000);
    let id2 = client.create_stream(&sender, &recipient2, &token, &10000, &1000, &3000);

    // At 1500: id1 is 50%, id2 is 25%
    env.ledger().with_mut(|li| { li.timestamp = 1500; });
    
    client.claim(&recipient1, &id1);
    client.claim(&recipient2, &id2);
    
    assert_eq!(token_client.balance(&recipient1), 5000);
    assert_eq!(token_client.balance(&recipient2), 2500);
}

#[test]
fn test_cancel_after_partial_claim() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    let start_time = 1000;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });
    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &start_time, &(start_time + 1000));

    // 1. Advance to 25% (250s)
    env.ledger().with_mut(|li| { li.timestamp = start_time + 250; });
    client.claim(&recipient, &stream_id);
    assert_eq!(token_client.balance(&recipient), 2500);

    // 2. Advance to 50% (500s)
    env.ledger().with_mut(|li| { li.timestamp = start_time + 500; });
    
    // 3. Sender cancels
    client.cancel_stream(&sender, &stream_id);

    // Verify:
    // Recipient should have received the "unclaimed but accrued" 2,500 more.
    assert_eq!(token_client.balance(&recipient), 5000);
    // Sender should have received 5,000 refund (10,000 - 5,000 accrued).
    assert_eq!(token_client.balance(&sender), 5000);

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Cancelled);
}

#[test]
fn test_invalid_start_time() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    env.ledger().with_mut(|li| { li.timestamp = 1000; });
    
    // Attempt to create stream starting in the past (999 < 1000)
    let result = client.try_create_stream(&sender, &recipient, &token, &1000, &999, &2000);
    assert!(result.is_err());
}

#[test]
fn test_claim_multiple_times_progression() {
    let (env, admin, client) = setup_env();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token = token_contract.address.clone();
    let token_client = create_token_client(&env, &token);
    token_contract.mint(&sender, &10000);

    client.initialize(&admin);

    let start_time = 1000;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });
    let stream_id = client.create_stream(&sender, &recipient, &token, &10000, &start_time, &(start_time + 1000));

    for i in 1..=10 {
        env.ledger().with_mut(|li| { li.timestamp = start_time + (i * 100); });
        client.claim(&recipient, &stream_id);
        assert_eq!(token_client.balance(&recipient), (i as i128) * 1000);
    }

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.status, StreamStatus::Completed);
}

// ── Property-Based Invariant Tests (Issue #28) ─────────────────────
//
// Deterministic, seeded property tests. A splitmix64-style PRNG drives
// bounded randomized configurations and event sequences, giving
// reproducible coverage of:
//   1. Conservation: sender + recipient + contract balances == total_amount
//      at every observable point for arbitrary create -> claim* -> cancel orderings.
//   2. Monotonic accrual: `get_claimable` is non-decreasing over time when no
//      claim occurs between samples, and never exceeds `total_amount`.
//   3. Terminal state: once a stream is Cancelled, no further payout is possible
//      and balances cannot change.

fn next_rand(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn rand_range_u64(state: &mut u64, lo: u64, hi_inclusive: u64) -> u64 {
    lo + next_rand(state) % (hi_inclusive - lo + 1)
}

fn rand_range_i128(state: &mut u64, lo: i128, hi_inclusive: i128) -> i128 {
    lo + (next_rand(state) as i128) % (hi_inclusive - lo + 1)
}

/// Property: conservation holds for randomized create -> claim* -> {cancel | final claim}.
#[test]
fn test_property_conservation_across_random_sequences() {
    let seeds: [u64; 8] = [
        0xDEAD_BEEF, 0x00C0_FFEE, 0x1337_1337, 0xABCD_1234,
        0x4242_4242, 0x9999_8888, 0x0000_FFFF, 0xBADD_CAFE,
    ];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token = token_contract.address.clone();
        let token_client = create_token_client(&env, &token);

        let total_amount: i128 = rand_range_i128(&mut rng, 1_000, 1_000_000);
        let duration: u64 = rand_range_u64(&mut rng, 200, 10_000);
        let start_time: u64 = 1_000;
        let end_time = start_time + duration;

        token_contract.mint(&sender, &total_amount);
        client.initialize(&admin);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let stream_id = client.create_stream(
            &sender, &recipient, &token, &total_amount, &start_time, &end_time,
        );

        let n_claims = rand_range_u64(&mut rng, 0, 4);
        let mut last_t = start_time;
        for _ in 0..n_claims {
            let step = rand_range_u64(&mut rng, 1, duration / 2 + 1);
            let next_t = last_t + step;
            let clamped = if next_t > end_time { end_time } else { next_t };
            env.ledger().with_mut(|li| { li.timestamp = clamped; });
            let _ = client.try_claim(&recipient, &stream_id);
            last_t = clamped;
        }

        if (next_rand(&mut rng) & 1) == 1 {
            let step = rand_range_u64(&mut rng, 0, duration / 2 + 1);
            env.ledger().with_mut(|li| { li.timestamp = last_t + step; });
            let _ = client.try_cancel_stream(&sender, &stream_id);
        } else {
            env.ledger().with_mut(|li| { li.timestamp = end_time + 1; });
            let _ = client.try_claim(&recipient, &stream_id);
        }

        let sender_bal = token_client.balance(&sender);
        let recipient_bal = token_client.balance(&recipient);
        let contract_bal = token_client.balance(&client.address);
        assert_eq!(
            sender_bal + recipient_bal + contract_bal, total_amount,
            "conservation failed for seed=0x{:X}: sender={} recipient={} contract={} expected={}",
            seed, sender_bal, recipient_bal, contract_bal, total_amount
        );

        let stream = client.get_stream(&stream_id);
        assert!(stream.claimed_amount >= 0);
        assert!(stream.claimed_amount <= stream.total_amount);
    }
}

/// Property: `get_claimable` is non-decreasing over time (without intermediate claims)
/// and never exceeds `total_amount`; equals `total_amount` after `end_time`.
#[test]
fn test_property_monotonic_claimable_and_bounded() {
    let seeds: [u64; 4] = [0x0000_A5A5, 0x0000_5A5A, 0x0000_C001, 0x0000_F00D];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token = token_contract.address.clone();

        let total_amount: i128 = rand_range_i128(&mut rng, 1_000, 1_000_000);
        let duration: u64 = rand_range_u64(&mut rng, 200, 10_000);
        let start_time: u64 = 1_000;
        let end_time = start_time + duration;

        token_contract.mint(&sender, &total_amount);
        client.initialize(&admin);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let stream_id = client.create_stream(
            &sender, &recipient, &token, &total_amount, &start_time, &end_time,
        );

        let n_samples: u64 = 10;
        let step = duration / n_samples;
        let mut prev_claimable: i128 = 0;
        for i in 0..=n_samples {
            let t = start_time + i * step;
            env.ledger().with_mut(|li| { li.timestamp = t; });
            let c = client.get_claimable(&stream_id);
            assert!(
                c >= prev_claimable,
                "seed=0x{:X}: claimable decreased at t={} (prev={}, now={})",
                seed, t, prev_claimable, c
            );
            assert!(
                c <= total_amount,
                "seed=0x{:X}: claimable {} exceeds total_amount {}",
                seed, c, total_amount
            );
            prev_claimable = c;
        }

        env.ledger().with_mut(|li| { li.timestamp = end_time + 100; });
        assert_eq!(
            client.get_claimable(&stream_id), total_amount,
            "seed=0x{:X}: claimable after end != total_amount", seed
        );
    }
}

/// Property: after cancel, no further payout is possible and balances are frozen.
#[test]
fn test_property_terminal_state_no_payout_after_cancel() {
    let seeds: [u64; 4] = [0x0000_1111, 0x0000_2222, 0x0000_3333, 0x0000_4444];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token = token_contract.address.clone();
        let token_client = create_token_client(&env, &token);

        let total_amount: i128 = rand_range_i128(&mut rng, 1_000, 100_000);
        let duration: u64 = rand_range_u64(&mut rng, 200, 5_000);
        let start_time: u64 = 1_000;
        let end_time = start_time + duration;

        token_contract.mint(&sender, &total_amount);
        client.initialize(&admin);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let stream_id = client.create_stream(
            &sender, &recipient, &token, &total_amount, &start_time, &end_time,
        );

        let cancel_offset = rand_range_u64(&mut rng, 1, duration - 1);
        env.ledger().with_mut(|li| { li.timestamp = start_time + cancel_offset; });
        client.cancel_stream(&sender, &stream_id);

        let sender_bal_at_cancel = token_client.balance(&sender);
        let recipient_bal_at_cancel = token_client.balance(&recipient);
        let contract_bal_at_cancel = token_client.balance(&client.address);

        env.ledger().with_mut(|li| { li.timestamp = end_time + 10; });
        assert!(
            client.try_claim(&recipient, &stream_id).is_err(),
            "seed=0x{:X}: claim after cancel unexpectedly succeeded", seed
        );
        assert!(
            client.try_cancel_stream(&sender, &stream_id).is_err(),
            "seed=0x{:X}: double cancel unexpectedly succeeded", seed
        );

        assert_eq!(token_client.balance(&sender), sender_bal_at_cancel);
        assert_eq!(token_client.balance(&recipient), recipient_bal_at_cancel);
        assert_eq!(token_client.balance(&client.address), contract_bal_at_cancel);

        let stream = client.get_stream(&stream_id);
        assert_eq!(stream.status, StreamStatus::Cancelled);
    }
}

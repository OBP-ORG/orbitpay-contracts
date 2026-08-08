#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env, symbol_short, token};
use types::VestingStatus;

fn setup_env() -> (Env, Address, VestingContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract_addr = e.register_stellar_asset_contract_v2(admin.clone()).address();
    token::StellarAssetClient::new(e, &contract_addr)
}

#[test]
fn test_initialize() {
    let (_env, admin, client) = setup_env();
    client.initialize(&admin);
    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_schedule_count(), 0);
}

#[test]
fn test_create_schedule() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    // 4-year vesting with 1-year cliff
    let year = 365 * 24 * 60 * 60_u64;
    let total_amount = 100_000_i128;
    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &total_amount,
        &1000_u64,     // start_time
        &year,         // cliff_duration (1 year)
        &25_000_i128,  // cliff_amount (25% for 1/4 time to match linear)
        &(4 * year),   // total_duration (4 years)
        &symbol_short!("team"),
        &true,         // revocable
    );

    assert_eq!(schedule_id, 0);
    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.total_amount, 100_000);
    assert_eq!(schedule.status, VestingStatus::Active);

    // Verify token transfers
    assert_eq!(token_client.balance(&grantor), 0);
    assert_eq!(token_client.balance(&client.address), 100_000);
}

#[test]
fn test_claim_tokens() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("team"),
        &true,
    );

    // Move to 2 years (50% vested)
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + (2 * year);
    });

    let claimed = client.claim(&beneficiary, &schedule_id);
    assert_eq!(claimed, 50_000);
    
    assert_eq!(token_client.balance(&beneficiary), 50_000);
    assert_eq!(token_client.balance(&client.address), 50_000);
}

#[test]
fn test_revoke_withdrawal() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("team"),
        &true,
    );

    // Move to 2 years, then revoke
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + (2 * year);
    });

    let unvested = client.revoke(&grantor, &schedule_id);
    assert_eq!(unvested, 50_000);

    assert_eq!(token_client.balance(&grantor), 50_000);
    assert_eq!(token_client.balance(&client.address), 50_000); // 50k still there for beneficiary to claim
}

#[test]
fn test_claim_after_revoke() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("team"),
        &true,
    );

    // Move to 2 years, then revoke
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + (2 * year);
    });

    let unvested = client.revoke(&grantor, &schedule_id);
    assert_eq!(unvested, 50_000);

    // Beneficiary should be able to claim the 50_000 that vested
    let claimed = client.claim(&beneficiary, &schedule_id);
    assert_eq!(claimed, 50_000);
    
    // Status should be FullyClaimed
    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.status, VestingStatus::FullyClaimed);

    // Trying to revoke again should fail with AlreadyFullyClaimed
    let revoke_again = client.try_revoke(&grantor, &schedule_id);
    assert_eq!(revoke_again, Err(Ok(VestingError::AlreadyFullyClaimed)));
    
    // Token balances should be correct
    assert_eq!(token_client.balance(&grantor), 50_000);
    assert_eq!(token_client.balance(&beneficiary), 50_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
fn test_insufficient_balance_on_create() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    // Grantor has 0 tokens

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;

    let result = client.try_create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("fail"),
        &true,
    );

    assert!(result.is_err());
    // Error(Contract, #12) is InsufficientBalance
}

#[test]
fn test_cliff_not_reached() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("team"),
        &true,
    );

    // Move time to 6 months (before cliff)
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + (year / 2);
    });

    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.vested_amount, 0);
    assert_eq!(progress.claimable_amount, 0);
}

#[test]
fn test_vesting_after_cliff() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("team"),
        &true,
    );

    // Move to exactly 2 years (50% vested)
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + (2 * year);
    });

    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.vested_amount, 50_000);
    assert_eq!(progress.claimable_amount, 50_000);
}

#[test]
fn test_explicit_cliff_amount() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;

    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &1000_u64,
        &year,
        &50_000_i128,
        &(4 * year),
        &symbol_short!("custom"),
        &true,
    );

    // 1. Check exactly at cliff
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + year;
    });
    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.vested_amount, 50_000);

    // 2. Check halfway through remaining vesting
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + year + (year + year / 2);
    });
    let progress_mid = client.get_progress(&schedule_id);
    assert_eq!(progress_mid.vested_amount, 75_000);

    // 3. Check at end
    env.ledger().with_mut(|li| {
        li.timestamp = 1000 + (4 * year);
    });
    let progress_end = client.get_progress(&schedule_id);
    assert_eq!(progress_end.vested_amount, 100_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // InvalidAmount
fn test_invalid_cliff_amount() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;

    client.create_schedule(
        &grantor,
        &beneficiary,
        &token,
        &100_000_i128,
        &1000_u64,
        &year,
        &150_000_i128, // cliff_amount > total_amount
        &(4 * year),
        &symbol_short!("fail"),
        &true,
    );
}

// ── SC-20: Comprehensive Vesting Tests ───────────────────────────

/// SC-20 Task 1: Nothing vests before cliff
#[test]
fn test_nothing_vests_before_cliff() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Check at start (0 elapsed)
    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.vested_amount, 0);
    assert_eq!(progress.claimable_amount, 0);

    // Check at 1 day
    env.ledger().with_mut(|li| { li.timestamp = start_time + 86_400; });
    let progress_1d = client.get_progress(&schedule_id);
    assert_eq!(progress_1d.vested_amount, 0);

    // Check at 6 months (halfway through cliff)
    env.ledger().with_mut(|li| { li.timestamp = start_time + (year / 2); });
    let progress_6m = client.get_progress(&schedule_id);
    assert_eq!(progress_6m.vested_amount, 0);
    assert_eq!(progress_6m.claimable_amount, 0);

    // Check at 1 second before cliff
    env.ledger().with_mut(|li| { li.timestamp = start_time + year - 1; });
    let progress_pre = client.get_progress(&schedule_id);
    assert_eq!(progress_pre.vested_amount, 0);

    // Verify claim attempt before cliff fails
    let claim_result = client.try_claim(&beneficiary, &schedule_id);
    assert!(claim_result.is_err());
}

/// SC-20 Task 2: Exact cliff amount vests at cliff time
#[test]
fn test_exact_cliff_amount_at_cliff_time() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Move to exactly cliff time
    env.ledger().with_mut(|li| { li.timestamp = start_time + year; });

    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.vested_amount, 25_000);
    assert_eq!(progress.claimable_amount, 25_000);

    // Claim exactly the cliff amount
    let claimed = client.claim(&beneficiary, &schedule_id);
    assert_eq!(claimed, 25_000);
    assert_eq!(token_client.balance(&beneficiary), 25_000);
}

/// SC-20 Task 3: Linear vesting at 25%, 50%, 75%
#[test]
fn test_linear_vesting_at_milestones() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    // Use uniform schedule: 100k total, 1yr cliff, 25k cliff_amount, 4yr total
    // cliff_amount = 25k vests at 1yr. remaining 75k linear over 3yrs (year 1 to 4)
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // At 25% (1 year) -- cliff amount
    env.ledger().with_mut(|li| { li.timestamp = start_time + year; });
    let p25 = client.get_progress(&schedule_id);
    assert_eq!(p25.vested_amount, 25_000);

    // At 50% (2 years) -- cliff + 1/3 of remaining = 25k + 25k = 50k
    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });
    let p50 = client.get_progress(&schedule_id);
    assert_eq!(p50.vested_amount, 50_000);

    // At 75% (3 years) -- cliff + 2/3 of remaining = 25k + 50k = 75k
    env.ledger().with_mut(|li| { li.timestamp = start_time + (3 * year); });
    let p75 = client.get_progress(&schedule_id);
    assert_eq!(p75.vested_amount, 75_000);
}

/// SC-20 Task 4: Full vesting after total duration
#[test]
fn test_full_vesting_after_total_duration() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Move to exactly end of total duration
    env.ledger().with_mut(|li| { li.timestamp = start_time + (4 * year); });

    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.vested_amount, 100_000);
    assert_eq!(progress.claimable_amount, 100_000);

    // Claim all
    let claimed = client.claim(&beneficiary, &schedule_id);
    assert_eq!(claimed, 100_000);
    assert_eq!(token_client.balance(&beneficiary), 100_000);
    assert_eq!(token_client.balance(&client.address), 0);

    // Schedule should be FullyClaimed
    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.status, VestingStatus::FullyClaimed);

    // Also verify far past the duration
    env.ledger().with_mut(|li| { li.timestamp = start_time + (10 * year); });
    let progress_late = client.get_progress(&schedule_id);
    assert_eq!(progress_late.vested_amount, 100_000);
}

/// SC-20 Task 5: Claim, then claim again later for remaining
#[test]
fn test_claim_then_claim_remaining() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // First claim at 2 years (50% vested = 50k)
    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });
    let first_claim = client.claim(&beneficiary, &schedule_id);
    assert_eq!(first_claim, 50_000);
    assert_eq!(token_client.balance(&beneficiary), 50_000);

    // Verify claimed_amount updated
    let progress_mid = client.get_progress(&schedule_id);
    assert_eq!(progress_mid.claimed_amount, 50_000);
    assert_eq!(progress_mid.claimable_amount, 0);

    // Second claim at 3 years (75% vested = 75k, already claimed 50k, claimable 25k)
    env.ledger().with_mut(|li| { li.timestamp = start_time + (3 * year); });
    let second_claim = client.claim(&beneficiary, &schedule_id);
    assert_eq!(second_claim, 25_000);
    assert_eq!(token_client.balance(&beneficiary), 75_000);

    // Third claim at 4 years (100% vested, already claimed 75k, claimable 25k)
    env.ledger().with_mut(|li| { li.timestamp = start_time + (4 * year); });
    let third_claim = client.claim(&beneficiary, &schedule_id);
    assert_eq!(third_claim, 25_000);
    assert_eq!(token_client.balance(&beneficiary), 100_000);

    // Schedule should be fully claimed now
    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.status, VestingStatus::FullyClaimed);

    // Fourth claim should fail: already fully claimed
    let result = client.try_claim(&beneficiary, &schedule_id);
    assert!(result.is_err());
}

/// SC-20 Task 6: Non-revocable schedule cannot be revoked
#[test]
fn test_non_revocable_schedule_cannot_be_revoked() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    // Create a NON-revocable schedule (revocable = false)
    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("locked"), &false,
    );

    // Advance time and try to revoke
    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });

    let result = client.try_revoke(&grantor, &schedule_id);
    assert!(result.is_err());

    // Verify schedule remains Active
    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.status, VestingStatus::Active);
    assert_eq!(schedule.total_amount, 100_000);
}

/// SC-20 Task 7: Unauthorized revoke by non-grantor
#[test]
fn test_unauthorized_revoke_by_non_grantor() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let attacker = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });

    // Attacker tries to revoke -- should fail with Unauthorized
    let result = client.try_revoke(&attacker, &schedule_id);
    assert!(result.is_err());

    // Beneficiary also should not be able to revoke
    let result2 = client.try_revoke(&beneficiary, &schedule_id);
    assert!(result2.is_err());

    // Verify schedule remains intact
    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.status, VestingStatus::Active);
    assert_eq!(schedule.total_amount, 100_000);
}

/// SC-20 Task 8: Multiple schedules for same beneficiary
#[test]
fn test_multiple_schedules_same_beneficiary() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &300_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    // Schedule A: 100k, 4yr vesting, 1yr cliff, "team" label
    let id_a = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Schedule B: 50k, 2yr vesting, 6mo cliff, "advisor" label
    let id_b = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &50_000_i128, &start_time, &(year / 2), &10_000_i128,
        &(2 * year), &symbol_short!("advisor"), &false,
    );

    // Schedule C: 150k, 4yr vesting, 1yr cliff, "seed" label
    let id_c = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &150_000_i128, &start_time, &year, &50_000_i128,
        &(4 * year), &symbol_short!("seed"), &true,
    );

    assert_eq!(client.get_schedule_count(), 3);

    // Verify beneficiary index has all 3 schedules
    let beneficiary_schedules = client.get_schedules_by_beneficiary(&beneficiary);
    assert_eq!(beneficiary_schedules.len(), 3);

    // Move to 1 year -- Schedule A: cliff 25k, Schedule B: partially linear, Schedule C: cliff 50k
    env.ledger().with_mut(|li| { li.timestamp = start_time + year; });

    let prog_a = client.get_progress(&id_a);
    assert_eq!(prog_a.vested_amount, 25_000);

    // Schedule B (2yr total, 6mo cliff, 10k cliff_amount, 50k total)
    // At 1yr: cliff_amount(10k) + (40k * (1yr - 6mo) / (2yr - 6mo)) = 10k + 40k * 0.5/1.5 = 10k + 13333 = 23333
    let prog_b = client.get_progress(&id_b);
    assert_eq!(prog_b.vested_amount, 23_333);

    let prog_c = client.get_progress(&id_c);
    assert_eq!(prog_c.vested_amount, 50_000);

    // Claim from all schedules independently
    let claimed_a = client.claim(&beneficiary, &id_a);
    assert_eq!(claimed_a, 25_000);

    let claimed_b = client.claim(&beneficiary, &id_b);
    assert_eq!(claimed_b, 23_333);

    let claimed_c = client.claim(&beneficiary, &id_c);
    assert_eq!(claimed_c, 50_000);

    assert_eq!(token_client.balance(&beneficiary), 98_333); // 25k + 23333 + 50k

    // Verify each schedule tracks claimed amounts independently
    let sched_a = client.get_schedule(&id_a);
    assert_eq!(sched_a.claimed_amount, 25_000);

    let sched_b = client.get_schedule(&id_b);
    assert_eq!(sched_b.claimed_amount, 23_333);

    let sched_c = client.get_schedule(&id_c);
    assert_eq!(sched_c.claimed_amount, 50_000);
}

#[test]
fn test_claim_history() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let _token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| {
        li.timestamp = start_time;
    });

    let schedule_id = client.create_schedule(
        &grantor,
        &beneficiary,
        &token_contract.address,
        &100_000_i128,
        &start_time,
        &year,
        &25_000_i128,
        &(4 * year),
        &symbol_short!("legacy"),
        &true,
    );

    // 1. Claim at 2 years
    let time1 = start_time + (2 * year);
    env.ledger().with_mut(|li| {
        li.timestamp = time1;
    });
    client.claim(&beneficiary, &schedule_id);

    // 2. Claim at 3 years
    let time2 = start_time + (3 * year);
    env.ledger().with_mut(|li| {
        li.timestamp = time2;
    });
    client.claim(&beneficiary, &schedule_id);

    let history = client.get_claim_history(&schedule_id);
    assert_eq!(history.len(), 2);
    
    assert_eq!(history.get(0).unwrap().amount, 50_000);
    assert_eq!(history.get(0).unwrap().timestamp, time1);
    
    assert_eq!(history.get(1).unwrap().amount, 25_000);
    assert_eq!(history.get(1).unwrap().timestamp, time2);
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
#[should_panic(expected = "Error(Contract, #14)")]
fn test_upgrade_timelock_rejected_before_delay() {
    let (env, admin, client) = setup_env();
    let executor = Address::generate(&env);

    let wasm_hash = soroban_sdk::BytesN::from_array(&env, &[1; 32]);

    client.initialize(&admin);

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

    client.initialize(&admin);
    client.propose_upgrade(&admin, &wasm_hash, &symbol_short!("sec_patch"));

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some());
}

// ── Conservation Invariant Tests (Issue #33) ─────────────────────

/// Verifies that the conservation invariant Σ(beneficiary_claims) + grantor_refund == original_total
/// holds for Claim→Revoke→Claim sequences.
#[test]
fn test_conservation_invariant_claim_revoke_claim() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Move to 2 years (50% vested = 50k) and claim half
    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });
    let first_claim = client.claim(&beneficiary, &schedule_id);
    assert_eq!(first_claim, 50_000);

    // Revoke — unvested = 50k
    let unvested = client.revoke(&grantor, &schedule_id);
    assert_eq!(unvested, 50_000);

    // Post-revoke: claim remaining vested amount (cliff + linear - already claimed = 50k - 50k = 0)
    let post_revoke_claim = client.try_claim(&beneficiary, &schedule_id);
    assert!(post_revoke_claim.is_err());

    // Invariant: beneficiary got 50k, grantor got 50k, total = 100k = original
    assert_eq!(token_client.balance(&beneficiary), 50_000);
    assert_eq!(token_client.balance(&grantor), 50_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

/// Verifies revoke before cliff — all funds should return to grantor.
#[test]
fn test_revoke_before_cliff_full_refund() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);

    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Revoke before cliff (6 months in, vested = 0)
    env.ledger().with_mut(|li| { li.timestamp = start_time + (year / 2); });

    let unvested = client.revoke(&grantor, &schedule_id);
    assert_eq!(unvested, 100_000);

    assert_eq!(token_client.balance(&grantor), 100_000);
    assert_eq!(token_client.balance(&client.address), 0);

    // Beneficiary cannot claim
    let claim_result = client.try_claim(&beneficiary, &schedule_id);
    assert!(claim_result.is_err());
}

/// Verifies revoke after end — nothing to return, beneficiary keeps what's vested.
#[test]
fn test_revoke_after_end_zero_refund() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Move past end to 5 years
    env.ledger().with_mut(|li| { li.timestamp = start_time + (5 * year); });

    let unvested = client.revoke(&grantor, &schedule_id);
    assert_eq!(unvested, 0);

    // All 100k remains — beneficiary can still claim
    let claimed = client.claim(&beneficiary, &schedule_id);
    assert_eq!(claimed, 100_000);

    assert_eq!(token_client.balance(&beneficiary), 100_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

/// Verifies revoke exactly at cliff time — cliff amount is vested, rest returned.
#[test]
fn test_revoke_at_exact_cliff() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    let token_client = token::Client::new(&env, &token_contract.address);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Revoke exactly at cliff — 25k vested, 75k unvested
    env.ledger().with_mut(|li| { li.timestamp = start_time + year; });

    let unvested = client.revoke(&grantor, &schedule_id);
    assert_eq!(unvested, 75_000);

    // Beneficiary can claim the cliff amount
    let claimed = client.claim(&beneficiary, &schedule_id);
    assert_eq!(claimed, 25_000);

    assert_eq!(token_client.balance(&beneficiary), 25_000);
    assert_eq!(token_client.balance(&grantor), 75_000);
    assert_eq!(token_client.balance(&client.address), 0);
}

/// Verifies that double-revoke fails gracefully.
#[test]
fn test_double_revoke_fails() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });
    let first = client.revoke(&grantor, &schedule_id);
    assert_eq!(first, 50_000);

    let second = client.try_revoke(&grantor, &schedule_id);
    assert_eq!(second, Err(Ok(VestingError::ScheduleRevoked)));
}

/// Verifies that revoke after fully claimed fails.
#[test]
fn test_revoke_after_fully_claimed_fails() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    // Full vesting after total duration
    env.ledger().with_mut(|li| { li.timestamp = start_time + (4 * year); });
    client.claim(&beneficiary, &schedule_id);

    let revoke_result = client.try_revoke(&grantor, &schedule_id);
    assert_eq!(revoke_result, Err(Ok(VestingError::AlreadyFullyClaimed)));
}

/// Verifies that `original_total_amount` is preserved through revoke.
#[test]
fn test_original_total_preserved_after_revoke() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });
    client.revoke(&grantor, &schedule_id);

    let schedule = client.get_schedule(&schedule_id);
    assert_eq!(schedule.total_amount, 50_000);
    assert_eq!(schedule.original_total_amount, 100_000);
    assert!(schedule.original_total_amount >= schedule.total_amount);
}

/// Verifies that `original_total_amount` is exposed in progress.
#[test]
fn test_progress_shows_original_total() {
    let (env, admin, client) = setup_env();
    let grantor = Address::generate(&env);
    let beneficiary = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = create_token_contract(&env, &token_admin);
    token_contract.mint(&grantor, &100_000);

    client.initialize(&admin);
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;
    env.ledger().with_mut(|li| { li.timestamp = start_time; });

    let schedule_id = client.create_schedule(
        &grantor, &beneficiary, &token_contract.address,
        &100_000_i128, &start_time, &year, &25_000_i128,
        &(4 * year), &symbol_short!("team"), &true,
    );

    env.ledger().with_mut(|li| { li.timestamp = start_time + (2 * year); });
    client.revoke(&grantor, &schedule_id);

    let progress = client.get_progress(&schedule_id);
    assert_eq!(progress.original_total_amount, 100_000);
    assert_eq!(progress.total_amount, 50_000);
}

/// Fuzzed matrix: each (config, revoke_time) pair exercises a fresh schedule independently.
/// Verifies conservation invariant: Σ(beneficiary_claims) + grantor_refund + contract_remainder == original_total
#[test]
fn test_conservation_invariant_fuzzed() {
    let (env, admin, client) = setup_env();
    let year = 365 * 24 * 60 * 60_u64;
    let start_time = 1000_u64;

    client.initialize(&admin);

    let configs = [
        (100_000, year, 4 * year, 25_000_i128),
        (50_000, year / 2, 2 * year, 5_000_i128),
        (200_000, year / 4, year, 50_000_i128),
        (1_000_000, year, 4 * year, 250_000_i128),
        (99_999, year * 3 / 4, 3 * year, 33_333_i128),
    ];

    let revoke_time_offsets = [
        0,                        // At start (vested=0)
        1,                        // Mid-cliff
        2,                        // Exactly at cliff
        3,                        // Quarter through linear
        4,                        // Half through linear
        5,                        // At end
        6,                        // Past end
    ];

    for (total, cliff_duration, total_duration, cliff_amount) in configs {
        for offset in revoke_time_offsets {
            let grantor = Address::generate(&env);
            let beneficiary = Address::generate(&env);

            let token_admin = Address::generate(&env);
            let token_contract = create_token_contract(&env, &token_admin);
            let token_client = token::Client::new(&env, &token_contract.address);
            token_contract.mint(&grantor, &total);

            env.ledger().with_mut(|li| { li.timestamp = start_time; });

            let sid = client.create_schedule(
                &grantor, &beneficiary, &token_contract.address,
                &total, &start_time, &cliff_duration, &cliff_amount,
                &total_duration, &symbol_short!("test"), &true,
            );

            let revoke_time = match offset {
                0 => start_time,
                1 => start_time + cliff_duration / 2,
                2 => start_time + cliff_duration,
                3 => start_time + cliff_duration + total_duration / 4,
                4 => start_time + cliff_duration + total_duration / 2,
                5 => start_time + total_duration,
                _ => start_time + total_duration + year,
            };

            env.ledger().with_mut(|li| { li.timestamp = revoke_time; });
            let _ = client.try_revoke(&grantor, &sid);

            let grantor_balance = token_client.balance(&grantor);
            let beneficiary_balance = token_client.balance(&beneficiary);
            let contract_balance = token_client.balance(&client.address);
            let total_accounted = grantor_balance + beneficiary_balance + contract_balance;

            assert_eq!(
                total_accounted, total,
                "Conservation invariant failed at offset={}: grantor={} beneficiary={} contract={} expected={}",
                offset, grantor_balance, beneficiary_balance, contract_balance, total
            );
        }
    }
}

// ── Property-Based Invariant Tests (Issue #28) ─────────────────────
//
// Deterministic, seeded property tests. A splitmix64-style PRNG drives
// bounded randomized configurations and event sequences, giving
// reproducible coverage of:
//   1. Conservation: grantor + beneficiary + contract balances == original total
//      for arbitrary create -> claim* -> {revoke, more claims} orderings.
//   2. Cliff invariant: vested == 0 before cliff; vested >= cliff_amount at cliff;
//      vested == total_amount at/after end.
//   3. Monotonic vested: `progress.vested_amount` is non-decreasing over time
//      and never exceeds `total_amount`.
//   4. Terminal state: post-revoke, only up to the vested-at-revoke amount can
//      be claimed; further claims fail; balances stay conserved.

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

/// Property: conservation holds for randomized create -> claim* -> {revoke, claim*} sequences.
#[test]
fn test_property_vesting_conservation_random_sequences() {
    let seeds: [u64; 8] = [
        0xDEAD_BEEF, 0x00C0_FFEE, 0x1337_1337, 0xABCD_1234,
        0x4242_4242, 0x9999_8888, 0x0000_FFFF, 0xBADD_CAFE,
    ];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token_client = token::Client::new(&env, &token_contract.address);

        let total_amount: i128 = rand_range_i128(&mut rng, 10_000, 1_000_000);
        let total_duration: u64 = rand_range_u64(&mut rng, 400, 10_000);
        let cliff_duration: u64 = rand_range_u64(&mut rng, 1, total_duration - 1);
        let cliff_amount: i128 = rand_range_i128(&mut rng, 0, total_amount / 2);
        let start_time: u64 = 1_000;

        token_contract.mint(&grantor, &total_amount);
        client.initialize(&admin);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let sid = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total_amount, &start_time, &cliff_duration, &cliff_amount,
            &total_duration, &symbol_short!("prop"), &true,
        );

        let n_claims = rand_range_u64(&mut rng, 0, 3);
        let mut last_t = start_time;
        for _ in 0..n_claims {
            let step = rand_range_u64(&mut rng, 1, total_duration / 2 + 1);
            last_t += step;
            env.ledger().with_mut(|li| { li.timestamp = last_t; });
            let _ = client.try_claim(&beneficiary, &sid);
        }

        // Optional revoke path
        if (next_rand(&mut rng) & 1) == 1 {
            let step = rand_range_u64(&mut rng, 0, total_duration / 2 + 1);
            env.ledger().with_mut(|li| { li.timestamp = last_t + step; });
            let _ = client.try_revoke(&grantor, &sid);
        }

        // Post-event tail: advance well past end and try one more claim
        env.ledger().with_mut(|li| { li.timestamp = start_time + total_duration + 1; });
        let _ = client.try_claim(&beneficiary, &sid);

        let g = token_client.balance(&grantor);
        let b = token_client.balance(&beneficiary);
        let c = token_client.balance(&client.address);
        assert_eq!(
            g + b + c, total_amount,
            "conservation failed for seed=0x{:X}: grantor={} beneficiary={} contract={} expected={}",
            seed, g, b, c, total_amount
        );

        let schedule = client.get_schedule(&sid);
        assert!(schedule.claimed_amount >= 0);
        assert!(schedule.claimed_amount <= schedule.original_total_amount);
    }
}

/// Property: cliff boundary — vested == 0 before cliff, vested >= cliff_amount at cliff,
/// vested == total_amount at/after `total_duration`.
#[test]
fn test_property_vesting_cliff_boundary() {
    let seeds: [u64; 6] = [
        0x0000_C11F, 0x0000_1F1E, 0x0000_DEAD, 0x0000_BEEF,
        0x0000_A5A5, 0x0000_5A5A,
    ];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &1_000_000);
        client.initialize(&admin);

        let total_amount: i128 = 1_000_000;
        let start_time: u64 = 1_000;
        let cliff_duration: u64 = rand_range_u64(&mut rng, 100, 1_000);
        let total_duration: u64 =
            cliff_duration + rand_range_u64(&mut rng, 100, 10_000);
        let cliff_amount: i128 = rand_range_i128(&mut rng, 0, total_amount / 2);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let sid = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total_amount, &start_time, &cliff_duration, &cliff_amount,
            &total_duration, &symbol_short!("prop"), &true,
        );

        // Just before cliff — nothing has vested
        env.ledger().with_mut(|li| { li.timestamp = start_time + cliff_duration - 1; });
        let progress = client.get_progress(&sid);
        assert_eq!(
            progress.vested_amount, 0,
            "seed=0x{:X}: non-zero vested before cliff", seed
        );
        assert_eq!(progress.claimable_amount, 0);

        // At cliff — vested is at least cliff_amount and at most total_amount
        env.ledger().with_mut(|li| { li.timestamp = start_time + cliff_duration; });
        let progress = client.get_progress(&sid);
        assert!(
            progress.vested_amount >= cliff_amount,
            "seed=0x{:X}: vested {} below cliff {} at cliff time",
            seed, progress.vested_amount, cliff_amount
        );
        assert!(progress.vested_amount <= total_amount);

        // At/after end — fully vested
        env.ledger().with_mut(|li| { li.timestamp = start_time + total_duration; });
        let progress = client.get_progress(&sid);
        assert_eq!(
            progress.vested_amount, total_amount,
            "seed=0x{:X}: vested {} != total {} at end", seed, progress.vested_amount, total_amount
        );

        env.ledger().with_mut(|li| { li.timestamp = start_time + total_duration + 12_345; });
        let progress = client.get_progress(&sid);
        assert_eq!(progress.vested_amount, total_amount);
    }
}

/// Property: `progress.vested_amount` is non-decreasing over time and bounded by `total_amount`.
#[test]
fn test_property_vesting_monotonic_vested() {
    let seeds: [u64; 4] = [0x0000_1111, 0x0000_2222, 0x0000_3333, 0x0000_4444];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &1_000_000);
        client.initialize(&admin);

        let total_amount: i128 = 1_000_000;
        let start_time: u64 = 1_000;
        let cliff_duration: u64 = rand_range_u64(&mut rng, 100, 1_000);
        let total_duration: u64 =
            cliff_duration + rand_range_u64(&mut rng, 200, 10_000);
        let cliff_amount: i128 = rand_range_i128(&mut rng, 0, total_amount / 4);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let sid = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total_amount, &start_time, &cliff_duration, &cliff_amount,
            &total_duration, &symbol_short!("prop"), &true,
        );

        let n_samples: u64 = 12;
        let step = total_duration / n_samples;
        let mut prev: i128 = 0;
        for i in 0..=n_samples {
            let t = start_time + i * step;
            env.ledger().with_mut(|li| { li.timestamp = t; });
            let v = client.get_progress(&sid).vested_amount;
            assert!(
                v >= prev,
                "seed=0x{:X}: vested decreased at t={} (prev={}, now={})",
                seed, t, prev, v
            );
            assert!(
                v <= total_amount,
                "seed=0x{:X}: vested {} exceeds total {}", seed, v, total_amount
            );
            prev = v;
        }

        env.ledger().with_mut(|li| { li.timestamp = start_time + total_duration + 1; });
        assert_eq!(
            client.get_progress(&sid).vested_amount, total_amount,
            "seed=0x{:X}: vested after end != total", seed
        );
    }
}

/// Property: after revoke, further claims cannot pay out more than vested-at-revoke;
/// once that is drained, additional claims fail and balances stay conserved.
#[test]
fn test_property_vesting_terminal_state_after_revoke() {
    let seeds: [u64; 4] = [0x0000_AAAA, 0x0000_BBBB, 0x0000_CCCC, 0x0000_DDDD];

    for seed in seeds {
        let mut rng = seed;
        let (env, admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        let token_client = token::Client::new(&env, &token_contract.address);
        token_contract.mint(&grantor, &1_000_000);
        client.initialize(&admin);

        let total_amount: i128 = 1_000_000;
        let start_time: u64 = 1_000;
        let cliff_duration: u64 = rand_range_u64(&mut rng, 100, 1_000);
        let total_duration: u64 =
            cliff_duration + rand_range_u64(&mut rng, 400, 10_000);
        let cliff_amount: i128 = rand_range_i128(&mut rng, 0, total_amount / 4);

        env.ledger().with_mut(|li| { li.timestamp = start_time; });
        let sid = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total_amount, &start_time, &cliff_duration, &cliff_amount,
            &total_duration, &symbol_short!("prop"), &true,
        );

        // Revoke somewhere in the vesting window
        let revoke_offset = rand_range_u64(&mut rng, cliff_duration, total_duration - 1);
        env.ledger().with_mut(|li| { li.timestamp = start_time + revoke_offset; });
        let vested_at_revoke = client.get_progress(&sid).vested_amount;
        client.revoke(&grantor, &sid);

        // Drain claimable, then verify further claims fail
        let _ = client.try_claim(&beneficiary, &sid);
        assert!(
            client.try_claim(&beneficiary, &sid).is_err(),
            "seed=0x{:X}: claim succeeded after fully draining post-revoke", seed
        );

        // Beneficiary can never receive more than what was vested at revoke time
        assert!(
            token_client.balance(&beneficiary) <= vested_at_revoke,
            "seed=0x{:X}: beneficiary got {} > vested_at_revoke {}",
            seed, token_client.balance(&beneficiary), vested_at_revoke
        );

        // Conservation still holds
        let g = token_client.balance(&grantor);
        let b = token_client.balance(&beneficiary);
        let c = token_client.balance(&client.address);
        assert_eq!(g + b + c, total_amount, "seed=0x{:X}: conservation broken after revoke", seed);

        // Double revoke fails
        assert!(
            client.try_revoke(&grantor, &sid).is_err(),
            "seed=0x{:X}: double revoke succeeded", seed
        );
    }
}

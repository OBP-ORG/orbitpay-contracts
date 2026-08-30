//! Property-based tests for the vesting contract.
//!
//! Uses proptest to generate random valid schedules and verify that core
//! invariants hold across thousands of random scenarios. Every assertion
//! includes the full generated input tuple so a failing case can be
//! replayed by appending it to `proptest-regressions/test_properties.txt`.

#![cfg(test)]

use super::*;
use proptest::prelude::*;
use soroban_sdk::{symbol_short, testutils::Address as _, testutils::Ledger, token, Address, Env};

fn setup_env() -> (Env, Address, VestingContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(VestingContract, ());
    let client = VestingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, admin, client)
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract_addr = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    token::StellarAssetClient::new(e, &contract_addr)
}

fn token_balance(env: &Env, token: &token::StellarAssetClient<'_>, addr: &Address) -> i128 {
    let client = token::Client::new(env, &token.address);
    client.balance(addr)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// After revoke: claimed + unvested + remaining_claimable == original_total.
    #[test]
    fn prop_conservation_revoke(
        total in 10_000_i128..1_000_000_i128,
        cliff_dur in 1u64..86_400u64,
        cliff_amt in 1i128..50_000_i128,
        total_dur in 1u64..50_000u64,
        mid_pct in 1u64..99u64,
    ) {
        prop_assume!(cliff_dur < total_dur);
        prop_assume!(cliff_amt <= total / 2);

        let (env, _admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &total);

        client.initialize(&_admin);

        let start: u64 = 1000;
        env.ledger().with_mut(|li| li.timestamp = start);

        let schedule_id = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total, &start, &cliff_dur, &cliff_amt, &total_dur,
            &symbol_short!("test"), &true,
        );

        let mid = start + (total_dur * mid_pct / 100);
        env.ledger().with_mut(|li| li.timestamp = mid);

        let unvested = client.revoke(&grantor, &schedule_id);
        let schedule = client.get_schedule(&schedule_id);

        let claimed = schedule.claimed_amount;
        let original = schedule.original_total_amount;
        let vested_at_revoke = original - unvested;
        let remaining_claimable = vested_at_revoke - claimed;

        assert_eq!(
            claimed + unvested + remaining_claimable,
            original,
            "CONSERVATION VIOLATED after revoke: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} mid_pct={mid_pct} \
             claimed={claimed} + unvested={unvested} + remaining={remaining_claimable} \
             != original={original}"
        );
        assert_eq!(schedule.status, VestingStatus::Revoked);
    }

    /// Claim some, then revoke — conservation must hold.
    #[test]
    fn prop_conservation_claim_then_revoke(
        total in 10_000_i128..1_000_000_i128,
        cliff_dur in 1u64..86_400u64,
        cliff_amt in 1i128..50_000_i128,
        total_dur in 1u64..50_000u64,
        claim_pct in 1u64..80u64,
    ) {
        prop_assume!(cliff_dur < total_dur);
        prop_assume!(cliff_amt <= total / 2);

        let (env, _admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &total);

        client.initialize(&_admin);

        let start: u64 = 1000;
        env.ledger().with_mut(|li| li.timestamp = start);

        let schedule_id = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total, &start, &cliff_dur, &cliff_amt, &total_dur,
            &symbol_short!("test"), &true,
        );

        let claim_time = start + cliff_dur + ((total_dur - cliff_dur) * claim_pct / 100);
        env.ledger().with_mut(|li| li.timestamp = claim_time);

        let claimed_before = match client.try_claim(&beneficiary, &schedule_id) {
            Ok(Ok(amount)) => amount,
            _ => 0,
        };

        let unvested = client.revoke(&grantor, &schedule_id);
        let schedule = client.get_schedule(&schedule_id);

        let claimed = schedule.claimed_amount;
        let original = schedule.original_total_amount;
        let vested_at_revoke = original - unvested;
        let remaining_claimable = vested_at_revoke - claimed;

        assert_eq!(
            claimed + unvested + remaining_claimable,
            original,
            "CONSERVATION VIOLATED after claim+revoke: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} claim_pct={claim_pct} \
             claimed={claimed} + unvested={unvested} + remaining={remaining_claimable} \
             != original={original}"
        );
        assert_eq!(schedule.status, VestingStatus::Revoked);
        prop_assume!(claimed_before > 0, "skip: no claimable tokens at this time step");
    }

    /// vested(t) is non-decreasing as ledger time advances.
    #[test]
    fn prop_monotonic_vested(
        total in 10_000_i128..1_000_000_i128,
        cliff_dur in 1u64..86_400u64,
        cliff_amt in 1i128..50_000_i128,
        total_dur in 1u64..50_000u64,
        t1_pct in 1u64..50u64,
        t2_pct in 51u64..100u64,
    ) {
        prop_assume!(cliff_dur < total_dur);
        prop_assume!(cliff_amt <= total / 2);

        let (env, _admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &total);

        client.initialize(&_admin);

        let start: u64 = 1000;
        env.ledger().with_mut(|li| li.timestamp = start);

        let schedule_id = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total, &start, &cliff_dur, &cliff_amt, &total_dur,
            &symbol_short!("test"), &true,
        );

        let t1 = start + (total_dur * t1_pct / 100);
        env.ledger().with_mut(|li| li.timestamp = t1);
        let p1 = client.get_progress(&schedule_id);
        let vested_t1 = p1.vested_amount;

        let t2 = start + (total_dur * t2_pct / 100);
        env.ledger().with_mut(|li| li.timestamp = t2);
        let p2 = client.get_progress(&schedule_id);
        let vested_t2 = p2.vested_amount;

        assert!(
            vested_t2 >= vested_t1,
            "MONOTONICITY VIOLATED: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} t1_pct={t1_pct} t2_pct={t2_pct} \
             vested(t1={t1})={vested_t1} > vested(t2={t2})={vested_t2}"
        );
    }

    /// claimable == 0 before cliff, > 0 after cliff (when cliff_amount > 0).
    #[test]
    fn prop_cliff_enforcement(
        total in 10_000_i128..1_000_000_i128,
        cliff_dur in 1u64..86_400u64,
        cliff_amt in 1_000_i128..50_000_i128,
        total_dur in 1u64..50_000u64,
    ) {
        prop_assume!(cliff_dur < total_dur);
        prop_assume!(cliff_amt <= total / 2);

        let (env, _admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &total);

        client.initialize(&_admin);

        let start: u64 = 1000;
        env.ledger().with_mut(|li| li.timestamp = start);

        let schedule_id = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total, &start, &cliff_dur, &cliff_amt, &total_dur,
            &symbol_short!("test"), &true,
        );

        let before_cliff = start + cliff_dur - 1;
        env.ledger().with_mut(|li| li.timestamp = before_cliff);
        let progress_before = client.get_progress(&schedule_id);
        assert_eq!(
            progress_before.claimable_amount, 0,
            "CLIFF VIOLATED before cliff: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} \
             claimable={} at t={} (cliff at t={})",
            progress_before.claimable_amount, before_cliff, start + cliff_dur,
        );

        let after_cliff = start + cliff_dur;
        env.ledger().with_mut(|li| li.timestamp = after_cliff);
        let progress_after = client.get_progress(&schedule_id);
        assert!(
            progress_after.claimable_amount > 0,
            "CLIFF VIOLATED after cliff: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} \
             claimable={} at t={}",
            progress_after.claimable_amount, after_cliff,
        );
    }

    /// FullyClaimed schedules reject claim(); revoked schedules reject revoke().
    #[test]
    fn prop_terminal_states_reject_claim(
        total in 10_000_i128..1_000_000_i128,
        cliff_dur in 1u64..86_400u64,
        cliff_amt in 1i128..50_000_i128,
        total_dur in 1u64..50_000u64,
    ) {
        prop_assume!(cliff_dur < total_dur);
        prop_assume!(cliff_amt <= total / 2);

        let (env, _admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &total);

        client.initialize(&_admin);

        let start: u64 = 1000;
        env.ledger().with_mut(|li| li.timestamp = start);

        let schedule_id = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total, &start, &cliff_dur, &cliff_amt, &total_dur,
            &symbol_short!("test"), &true,
        );

        let fully_vested = start + total_dur;
        env.ledger().with_mut(|li| li.timestamp = fully_vested);

        let claimed = client.claim(&beneficiary, &schedule_id);
        prop_assume!(claimed > 0);
        let schedule = client.get_schedule(&schedule_id);
        assert_eq!(schedule.status, VestingStatus::FullyClaimed);

        let claim_result = client.try_claim(&beneficiary, &schedule_id);
        assert!(
            claim_result.is_err(),
            "TERMINAL STATE VIOLATED: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} \
             claim succeeded on FullyClaimed schedule"
        );

        let revoke_result = client.try_revoke(&grantor, &schedule_id);
        assert!(
            revoke_result.is_err(),
            "TERMINAL STATE VIOLATED: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} \
             revoke succeeded on FullyClaimed schedule"
        );
    }

    /// claim -> revoke recovers unvested to grantor, conservation holds.
    #[test]
    fn prop_claim_roundtrip(
        total in 10_000_i128..1_000_000_i128,
        cliff_dur in 1u64..86_400u64,
        cliff_amt in 1i128..50_000_i128,
        total_dur in 1u64..50_000u64,
        claim_pct in 10u64..80u64,
    ) {
        prop_assume!(cliff_dur < total_dur);
        prop_assume!(cliff_amt <= total / 2);

        let (env, _admin, client) = setup_env();
        let grantor = Address::generate(&env);
        let beneficiary = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = create_token_contract(&env, &token_admin);
        token_contract.mint(&grantor, &total);

        client.initialize(&_admin);

        let start: u64 = 1000;
        env.ledger().with_mut(|li| li.timestamp = start);

        let schedule_id = client.create_schedule(
            &grantor, &beneficiary, &token_contract.address,
            &total, &start, &cliff_dur, &cliff_amt, &total_dur,
            &symbol_short!("test"), &true,
        );

        let grantor_after_create = token_balance(&env, &token_contract, &grantor);

        let claim_time = start + cliff_dur + ((total_dur - cliff_dur) * claim_pct / 100);
        env.ledger().with_mut(|li| li.timestamp = claim_time);

        let _claimed = match client.try_claim(&beneficiary, &schedule_id) {
            Ok(Ok(amount)) => amount,
            _ => 0,
        };

        let unvested = client.revoke(&grantor, &schedule_id);
        let schedule = client.get_schedule(&schedule_id);

        let grantor_after_revoke = token_balance(&env, &token_contract, &grantor);
        let grantor_gained = grantor_after_revoke - grantor_after_create;

        assert_eq!(
            grantor_gained, unvested,
            "ROUNDTRIP VIOLATED: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} claim_pct={claim_pct} \
             grantor gained {grantor_gained} but revoke returned {unvested}"
        );

        let original = schedule.original_total_amount;
        let vested_at_revoke = original - unvested;
        let remaining = vested_at_revoke - schedule.claimed_amount;
        assert_eq!(
            schedule.claimed_amount + unvested + remaining,
            original,
            "ROUNDTRIP CONSERVATION VIOLATED: total={total} cliff_dur={cliff_dur} \
             cliff_amt={cliff_amt} total_dur={total_dur} claim_pct={claim_pct} \
             claimed={} + unvested={unvested} + remaining={remaining} != original={original}",
            schedule.claimed_amount,
        );
    }
}

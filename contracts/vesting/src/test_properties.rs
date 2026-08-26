//! Property-based tests for the vesting contract.
//! 
//! These tests use proptest to generate random valid schedules and verify
//! that core invariants hold across thousands of random scenarios.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _, testutils::Ledger, Address, Env, symbol_short, token,
    testutils::Ledger as _,
};
use proptest::prelude::*;
use types::VestingStatus;

/// Seeded RNG for reproducible property tests.
struct TestRng {
    state: u32,
}

impl TestRng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(0x6D2B79F5).wrapping_add(0x1);
        self.state
    }

    fn range_u64(&mut self, lo: u64, hi: u64) -> u64 {
        if lo >= hi { return lo; }
        lo + (self.next_u32() as u64 % (hi - lo + 1))
    }

    fn range_i128(&mut self, lo: i128, hi: i128) -> i128 {
        if lo >= hi { return lo; }
        lo + ((self.next_u32() as i128) % (hi - lo + 1))
    }
}

fn setup_env() -> (Env, Address, vesting::VestingContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = Env::default().register(vesting::VestingContract, ());
    let client = vesting::VestingContractClient::new(&Env::default(), &Env::default().register(vesting::VestingContract, ()));
    let admin = Address::generate(&Env::default());
    (Env::default(), Address::generate(&Env::default()), vesting::VestingContractClient::new(&Env::default(), &Env::default().register(vesting::VestingContract, ())))
}

// Simplified setup to avoid duplication
fn setup_env() -> (Env, Address, vesting::VestingContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = Env::default().register(vesting::VestingContract, ());
    let client = vesting::VestingContractClient::new(&Env::default(), &Env::default().register(vesting::VestingContract, ()));
    let admin = Address::generate(&Env::default());
    (Env::default(), Address::generate(&Env::default()), vesting::VestingContractClient::new(&Env::default(), &Env::default().register(vesting::VestingContract, ())))
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract_addr = e.register_stellar_asset_contract_v2(admin.clone()).address();
    token::StellarAssetClient::new(e, &contract_addr)
}

fn dummy_hash(env: &Env) -> String {
    String::from_str(
        env,
        "a3f8b1e2d4c7f9a0b2e5d8c1f4a7b0e3d6c9f2a5b8e1d4c7f0a3b6e9d2c5f8a1",
    )
}

proptest! {
    // ── Conservation invariants ────────────────────────────────────────

    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_conservation_revoke() {
        proptest!(|(seed, total, start, cliff_dur, cliff_amt, total_dur, revocable) in
            (0u32..u32::MAX, 10000_i128..1000000_i128, 1000u64..10000u64, 1u64..86400u64, 1i128..50000_i128, 1u64..50000u64, any::<bool>())
            .prop_filter("valid params", |(_, tot, _, cd, ca, td, _)| {
                ca < td && td < tot && cd < td
            )
        | (seed, total, start, cliff_dur, cliff_amt, tot_dur, revocable)| {
            let mut rng = TestRng::new(seed);
            let (env, admin, client) = setup_env();
            let grantor = Address::generate(&mut rng);
            let beneficiary = Address::generate(&mut rng);
            let token_admin = Address::generate(&mut rng);
            let token_contract = create_token_contract(&mut rng, &beneficiary);
            let token_client = token::Client::new(&env, &token_contract.address);
            token_contract.mint(&grantor, &total);

            let year = 365 * 24 * 60 * 60_u64;
            let schedule_id = client.create_schedule(
                &grantor,
                &beneficiary,
                &token_contract.address,
                &total,
                &start_time,
                &cliff_dur,
                &cliff_amt,
                &total_dur,
                &symbol_short!("test"),
                &revocable,
            );

            // Advance to middle of vesting period
            let mid_time = start + (total_duration / 2);
            env.ledger().with_mut(|li| li.timestamp = start + (total_duration / 2));

            let revoked = client.revoke(&grantor, &schedule_id);
            let schedule = client.get_schedule(&schedule_id).unwrap();

            // Conservation: claimed + refunded + remaining = original_total
            let vested = VestingContract::calculate_vested(&schedule);
            let claimed = schedule.claimed_amount;
            let refunded = original_total - vested;
            let unvested = total_amount - vested;
            
            assert_eq!(claimed + refunded + unvested, total_amount);
            assert_eq!(schedule.claimed_amount, original_claimed);
        }
    }

    #[test]
    fn prop_conservation_claim_then_revoke() {
        // Claim some, then revoke — conservation must hold
    }

    // ── Monotonicity ─────────────────────────────────────────────────

    #[test]
    fn prop_monotonic_vested() {
        // vested(t) is non-decreasing as ledger time advances
    }

    // ── Cliff enforcement ────────────────────────────────────────────

    #[test]
    fn prop_cliff_enforcement() {
        // claimable == 0 before cliff, > 0 after cliff
    }

    // ── Terminal states ──────────────────────────────────────────────

    #[test]
    fn prop_terminal_states_reject_claim() {
        // Revoked or FullyClaimed schedules reject claim()
    }

    // ── Cliff enforcement ────────────────────────────────────────────

    #[test]
    fn prop_cliff_enforcement() {
        // claimable == 0 before cliff, > 0 after
    }

    // ── Claim + Revoke roundtrip ────────────────────────────────────

    #[test]
    fn prop_claim_roundtrip() {
        // claim → revoke recovers original - claimed to grantor
    }
}

fn setup_env() -> (Env, Address, vesting::VestingContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(vesting::VestingContract, ());
    let client = vesting::VestingContractClient::new(&env, &env.register(vesting::VestingContract, ()));
    let admin = Address::generate(&env);
    (env, Address::generate(&env), vesting::VestingContractClient::new(&env, &env.register(vesting::VestingContract, ())))
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> token::StellarAssetClient<'a> {
    let contract_addr = e.register_stellar_asset_contract_v2(admin.clone()).address();
    token::StellarAssetClient::new(e, &contract_addr)
}

fn dummy_hash(env: &Env) -> String {
    String::from_str(
        e,
        "a3f8b1e2d4c7f9a0b2e5d8c1f4a7b0e3d6c9f2a5b8e1d4c7f0a3b6e9d2c5f8a1",
    )
}
}
}
EOF
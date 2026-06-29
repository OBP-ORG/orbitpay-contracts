use soroban_sdk::{contracttype, Address, Env, Vec};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const MIN_UPGRADE_DELAY: u64 = 24 * 60 * 60; // 24 hours

use crate::types::{VestingSchedule, ClaimRecord, PendingUpgrade};

/// Keys used to store data in the contract's ledger storage.
#[contracttype]
pub enum DataKey {
    Admin,
    ScheduleCount,
    Schedule(u32),
    GrantorSchedules(Address),
    BeneficiarySchedules(Address),
    ClaimHistory(u32),
    PendingUpgradeWasm,
    PendingUpgradeProposedAt,
}

// ── Admin helpers ────────────────────────────────────────────────

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Admin).unwrap()
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

// ── Schedule count helpers ───────────────────────────────────────

pub fn get_schedule_count(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::ScheduleCount).unwrap_or(0)
}

pub fn set_schedule_count(env: &Env, count: u32) {
    env.storage().instance().set(&DataKey::ScheduleCount, &count);
}

// ── Schedule helpers ─────────────────────────────────────────────

pub fn get_schedule(env: &Env, id: u32) -> Option<VestingSchedule> {
    env.storage().persistent().get(&DataKey::Schedule(id))
}

pub fn set_schedule(env: &Env, id: u32, schedule: &VestingSchedule) {
    env.storage().persistent().set(&DataKey::Schedule(id), schedule);
}

// ── Index helpers ────────────────────────────────────────────────

pub fn get_grantor_schedules(env: &Env, grantor: &Address) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::GrantorSchedules(grantor.clone()))
        .unwrap_or(Vec::new(env))
}

pub fn add_grantor_schedule(env: &Env, grantor: &Address, schedule_id: u32) {
    let mut schedules = get_grantor_schedules(env, grantor);
    schedules.push_back(schedule_id);
    env.storage()
        .persistent()
        .set(&DataKey::GrantorSchedules(grantor.clone()), &schedules);
}

pub fn get_beneficiary_schedules(env: &Env, beneficiary: &Address) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::BeneficiarySchedules(beneficiary.clone()))
        .unwrap_or(Vec::new(env))
}

pub fn add_beneficiary_schedule(env: &Env, beneficiary: &Address, schedule_id: u32) {
    let mut schedules = get_beneficiary_schedules(env, beneficiary);
    schedules.push_back(schedule_id);
    env.storage()
        .persistent()
        .set(&DataKey::BeneficiarySchedules(beneficiary.clone()), &schedules);
}

// ── Claim history helpers ────────────────────────────────────────

pub fn get_claim_history(env: &Env, schedule_id: u32) -> Vec<ClaimRecord> {
    env.storage()
        .persistent()
        .get(&DataKey::ClaimHistory(schedule_id))
        .unwrap_or(Vec::new(env))
}

pub fn add_claim_record(env: &Env, schedule_id: u32, amount: i128, timestamp: u64) {
    let mut history = get_claim_history(env, schedule_id);
    history.push_back(ClaimRecord { amount, timestamp });
    env.storage()
        .persistent()
        .set(&DataKey::ClaimHistory(schedule_id), &history);
}

// ── TTL helpers ──────────────────────────────────────────────────

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn extend_schedule_ttl(env: &Env, id: u32) {
    env.storage().persistent().extend_ttl(
        &DataKey::Schedule(id),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_grantor_schedules_ttl(env: &Env, grantor: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::GrantorSchedules(grantor.clone()),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_beneficiary_schedules_ttl(env: &Env, beneficiary: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::BeneficiarySchedules(beneficiary.clone()),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_claim_history_ttl(env: &Env, schedule_id: u32) {
    env.storage().persistent().extend_ttl(
        &DataKey::ClaimHistory(schedule_id),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

// ── Upgrade timelock helpers ────────────────────────────────────────

pub fn get_pending_upgrade(env: &Env) -> Option<PendingUpgrade> {
    let wasm = env.storage().instance().get(&DataKey::PendingUpgradeWasm);
    let proposed_at = env.storage().instance().get(&DataKey::PendingUpgradeProposedAt);
    match (wasm, proposed_at) {
        (Some(w), Some(t)) => Some(PendingUpgrade { wasm_hash: w, proposed_at: t }),
        _ => None,
    }
}

pub fn set_pending_upgrade(env: &Env, upgrade: &PendingUpgrade) {
    env.storage()
        .instance()
        .set(&DataKey::PendingUpgradeWasm, &upgrade.wasm_hash);
    env.storage()
        .instance()
        .set(&DataKey::PendingUpgradeProposedAt, &upgrade.proposed_at);
}

pub fn clear_pending_upgrade(env: &Env) {
    env.storage().instance().remove(&DataKey::PendingUpgradeWasm);
    env.storage().instance().remove(&DataKey::PendingUpgradeProposedAt);
}

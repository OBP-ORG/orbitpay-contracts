use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::types::{UpgradeProposal, PauseState, PendingAdminChange, WithdrawalRequest};

pub(crate) const MIN_SIGNER_CHANGE_DELAY: u64 = 24 * 60 * 60; // 1 day in seconds

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const DEFAULT_SIGNER_CHANGE_DELAY: u64 = 7 * 24 * 60 * 60; // 7 days in seconds

/// Keys used to store data in the contract's ledger storage.
#[contracttype]
pub enum DataKey {
    /// The admin address — stored in Instance storage.
    Admin,
    /// List of authorized signers — stored in Instance storage.
    Signers,
    /// The multi-sig approval threshold — stored in Instance storage.
    Threshold,
    /// Running count of withdrawal proposals — stored in Instance storage.
    ProposalCount,
    /// Signer set version — incremented on signer changes, stored in Instance storage.
    SignerSetVersion,
    /// Current pause state — stored in Instance storage.
    PauseState,
    /// Timelock delay for signer changes in seconds — stored in Instance storage.
    SignerChangeDelay,
    /// Pending admin change — stored in Persistent storage.
    PendingAdminChange,
    /// A specific withdrawal request — stored in Persistent storage.
    Withdrawal(u32),
    /// Upgrade proposal — stored in Persistent storage.
    Upgrade(u32),
    /// Upgrade proposal count — stored in Instance storage.
    UpgradeCount,
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

// ── Signer helpers ───────────────────────────────────────────────

pub fn get_signers(env: &Env) -> Vec<Address> {
    env.storage().instance().get(&DataKey::Signers).unwrap()
}

pub fn set_signers(env: &Env, signers: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Signers, signers);
}

// ── Threshold helpers ────────────────────────────────────────────

pub fn get_threshold(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::Threshold).unwrap()
}

pub fn set_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

// ── Proposal count helpers ───────────────────────────────────────

pub fn get_proposal_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::ProposalCount)
        .unwrap()
}

pub fn set_proposal_count(env: &Env, count: u32) {
    env.storage()
        .instance()
        .set(&DataKey::ProposalCount, &count);
}

// ── Signer set version helpers ───────────────────────────────────

pub fn get_signer_set_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::SignerSetVersion)
        .unwrap()
}

pub fn set_signer_set_version(env: &Env, version: u32) {
    env.storage()
        .instance()
        .set(&DataKey::SignerSetVersion, &version);
}

pub fn increment_signer_set_version(env: &Env) -> u32 {
    let current = get_signer_set_version(env);
    let new_version = current + 1;
    set_signer_set_version(env, new_version);
    new_version
}

// ── Withdrawal helpers ──────────────────────────────────────────

pub fn get_withdrawal(env: &Env, id: u32) -> Option<WithdrawalRequest> {
    env.storage().persistent().get(&DataKey::Withdrawal(id))
}

pub fn set_withdrawal(env: &Env, id: u32, request: &WithdrawalRequest) {
    env.storage()
        .persistent()
        .set(&DataKey::Withdrawal(id), request);
}

// ── Pause state helpers ────────────────────────────────────────────

pub fn get_pause_state(env: &Env) -> PauseState {
    env.storage()
        .instance()
        .get(&DataKey::PauseState)
        .unwrap_or(PauseState::Unpaused)
}

pub fn set_pause_state(env: &Env, state: &PauseState) {
    env.storage().instance().set(&DataKey::PauseState, state);
}

pub fn is_paused(env: &Env) -> bool {
    matches!(get_pause_state(env), PauseState::Paused)
}

// ── Signer change delay helpers ────────────────────────────────────

pub fn get_signer_change_delay(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::SignerChangeDelay)
        .unwrap_or(DEFAULT_SIGNER_CHANGE_DELAY)
}

pub fn set_signer_change_delay(env: &Env, delay: u64) {
    env.storage().instance().set(&DataKey::SignerChangeDelay, &delay);
}

// ── Pending admin change helpers ────────────────────────────────────

pub fn get_pending_admin_change(env: &Env) -> Option<PendingAdminChange> {
    env.storage().persistent().get(&DataKey::PendingAdminChange)
}

pub fn set_pending_admin_change(env: &Env, change: &PendingAdminChange) {
    env.storage()
        .persistent()
        .set(&DataKey::PendingAdminChange, change);
}

pub fn clear_pending_admin_change(env: &Env) {
    env.storage().persistent().remove(&DataKey::PendingAdminChange);
}

// ── Upgrade proposal helpers ────────────────────────────────────────

pub fn get_upgrade_proposal(env: &Env, id: u32) -> Option<UpgradeProposal> {
    env.storage().persistent().get(&DataKey::Upgrade(id))
}

pub fn set_upgrade_proposal(env: &Env, id: u32, proposal: &UpgradeProposal) {
    env.storage()
        .persistent()
        .set(&DataKey::Upgrade(id), proposal);
}

pub fn get_upgrade_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::UpgradeCount)
        .unwrap_or(0)
}

pub fn set_upgrade_count(env: &Env, count: u32) {
    env.storage()
        .instance()
        .set(&DataKey::UpgradeCount, &count);
}

// ── TTL helpers ──────────────────────────────────────────────────

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn extend_withdrawal_ttl(env: &Env, id: u32) {
    env.storage().persistent().extend_ttl(
        &DataKey::Withdrawal(id),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_upgrade_proposal_ttl(env: &Env, id: u32) {
    env.storage().persistent().extend_ttl(
        &DataKey::Upgrade(id),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_pending_admin_change_ttl(env: &Env) {
    env.storage().persistent().extend_ttl(
        &DataKey::PendingAdminChange,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

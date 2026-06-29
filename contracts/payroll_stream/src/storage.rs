use soroban_sdk::{contracttype, Address, Env, Vec};

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

pub(crate) const MIN_UPGRADE_DELAY: u64 = 24 * 60 * 60; // 24 hours

use crate::types::{PayrollStream, PendingUpgrade};

/// Keys used to store data in the contract's ledger storage.
#[contracttype]
pub enum DataKey {
    Admin,
    StreamCount,
    Stream(u32),
    SenderStreams(Address),
    RecipientStreams(Address),
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

// ── Stream count helpers ─────────────────────────────────────────

pub fn get_stream_count(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::StreamCount)
        .unwrap_or(0)
}

pub fn set_stream_count(env: &Env, count: u32) {
    env.storage().instance().set(&DataKey::StreamCount, &count);
}

// ── Stream helpers ───────────────────────────────────────────────

pub fn get_stream(env: &Env, id: u32) -> Option<PayrollStream> {
    env.storage().persistent().get(&DataKey::Stream(id))
}

pub fn set_stream(env: &Env, id: u32, stream: &PayrollStream) {
    env.storage().persistent().set(&DataKey::Stream(id), stream);
}

// ── Index helpers for sender/recipient stream lookups ────────────

pub fn get_sender_streams(env: &Env, sender: &Address) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::SenderStreams(sender.clone()))
        .unwrap_or(Vec::new(env))
}

pub fn add_sender_stream(env: &Env, sender: &Address, stream_id: u32) {
    let mut streams = get_sender_streams(env, sender);
    streams.push_back(stream_id);
    env.storage()
        .persistent()
        .set(&DataKey::SenderStreams(sender.clone()), &streams);
}

pub fn get_recipient_streams(env: &Env, recipient: &Address) -> Vec<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::RecipientStreams(recipient.clone()))
        .unwrap_or(Vec::new(env))
}

pub fn add_recipient_stream(env: &Env, recipient: &Address, stream_id: u32) {
    let mut streams = get_recipient_streams(env, recipient);
    streams.push_back(stream_id);
    env.storage()
        .persistent()
        .set(&DataKey::RecipientStreams(recipient.clone()), &streams);
}

// ── TTL helpers ──────────────────────────────────────────────────

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn extend_stream_ttl(env: &Env, id: u32) {
    env.storage().persistent().extend_ttl(
        &DataKey::Stream(id),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_sender_streams_ttl(env: &Env, sender: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::SenderStreams(sender.clone()),
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn extend_recipient_streams_ttl(env: &Env, recipient: &Address) {
    env.storage().persistent().extend_ttl(
        &DataKey::RecipientStreams(recipient.clone()),
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

# Storage Layout — OrbitPay Contracts

All Soroban smart contracts use Stellar's ledger-based storage model with two
tiers: **Instance** (contract metadata, bounded size) and **Persistent**
(unbounded, per-key TTL).

---

## TTL Constants (Shared)

Defined in `contracts/treasury/src/storage.rs`:

| Constant | Value | Purpose |
|---|---|---|
| `DAY_IN_LEDGERS` | 17,280 | Ledgers per day (~5s/ledger) |
| `INSTANCE_BUMP_AMOUNT` | 518,400 | 30 days |
| `INSTANCE_LIFETIME_THRESHOLD` | 501,120 | 29 days (bump when <1 day remains) |
| `PERSISTENT_BUMP_AMOUNT` | 518,400 | 30 days |
| `PERSISTENT_LIFETIME_THRESHOLD` | 501,120 | 29 days |

**Note:** Payroll Stream, Vesting, and Governance do **NOT** implement TTL
extension. Only Treasury actively bumps storage TTL. This should be flagged
during audit as a potential availability risk.

---

## 1. Treasury Contract

### Instance Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Admin` | `Address` | `initialize` | All admin-gated functions, `require_initialized` |
| `DataKey::Signers` | `Vec<Address>` | `initialize`, `add_signer`, `remove_signer` | `create_withdrawal`, `approve_withdrawal`, `get_signers`, `update_threshold` |
| `DataKey::Threshold` | `u32` | `initialize`, `update_threshold` | `create_withdrawal`, `approve_withdrawal`, `remove_signer`, `get_threshold` |
| `DataKey::ProposalCount` | `u32` | `initialize`, `create_withdrawal` | `create_withdrawal`, `get_proposal_count` |
| `DataKey::SignerSetVersion` | `u32` | `initialize`, `add_signer`, `remove_signer` | `create_withdrawal`, `update_threshold` |

### Persistent Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Withdrawal(id)` | `WithdrawalRequest` | `create_withdrawal`, `approve_withdrawal`, `execute_withdrawal` | `get_withdrawal`, `approve_withdrawal`, `execute_withdrawal` |

### TTL Behavior

- Instance: bumped on `initialize`, `create_withdrawal`, `add_signer`, `remove_signer`, `update_threshold`
- Persistent (withdrawals): bumped on `create_withdrawal`, `approve_withdrawal`, `execute_withdrawal`
- TTL: 30-day bump when < 1 day remains

---

## 2. Payroll Stream Contract

### Instance Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Admin` | `Address` | `initialize` | `upgrade`, `has_admin` |
| `DataKey::StreamCount` | `u32` | `initialize`, `create_stream`, `create_batch_streams` | `create_stream`, `create_batch_streams`, `get_stream_count` |

### Persistent Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Stream(id)` | `PayrollStream` | `create_stream`, `create_batch_streams`, `claim`, `cancel_stream` | `get_stream`, `claim`, `cancel_stream` |
| `DataKey::SenderStreams(Address)` | `Vec<u32>` | `create_stream`, `create_batch_streams` | `get_streams_by_sender` |
| `DataKey::RecipientStreams(Address)` | `Vec<u32>` | `create_stream`, `create_batch_streams` | `get_streams_by_recipient` |

### TTL Behavior

**No TTL extension implemented.** Persistent entries will eventually expire
unless manually bumped. This is a gap for long-running payroll streams.

---

## 3. Vesting Contract

### Instance Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Admin` | `Address` | `initialize` | `upgrade`, `has_admin` |
| `DataKey::ScheduleCount` | `u32` | `initialize`, `create_schedule` | `create_schedule`, `get_schedule_count` |

### Persistent Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Schedule(id)` | `VestingSchedule` | `create_schedule`, `claim`, `revoke` | `get_schedule`, `claim`, `revoke` |
| `DataKey::GrantorSchedules(Address)` | `Vec<u32>` | `create_schedule` | `get_schedules_by_grantor` |
| `DataKey::BeneficiarySchedules(Address)` | `Vec<u32>` | `create_schedule` | `get_schedules_by_beneficiary` |
| `DataKey::ClaimHistory(id)` | `Vec<ClaimRecord>` | `claim` | `get_claim_history` |

### TTL Behavior

**No TTL extension implemented.** Vesting schedules (often multi-year) will
expire without explicit TTL management.

---

## 4. Governance Contract

### Instance Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Admin` | `Address` | `initialize` | Admin-gated functions, `has_admin` |
| `DataKey::Members` | `Vec<Address>` | `initialize`, `add_member`, `remove_member` | `create_proposal`, `vote`, `get_members` |
| `DataKey::ProposalCount` | `u32` | `initialize`, `create_proposal` | `create_proposal`, `get_proposal_count` |
| `DataKey::QuorumPercentage` | `u32` | `initialize` | `create_proposal`, `get_config` |
| `DataKey::VotingDuration` | `u64` | `initialize` | `create_proposal`, `get_config` |
| `DataKey::GracePeriod` | `u64` | `initialize` | `create_proposal`, `get_config` |
| `DataKey::TotalWeight` | `u128` | `initialize`, `add_member`, `remove_member`, `set_voting_weight` | `create_proposal`, `get_config` |

### Persistent Storage

| Key | Type | Set During | Read By |
|---|---|---|---|
| `DataKey::Proposal(id)` | `Proposal` | `create_proposal`, `vote`, `finalize`, `execute`, `cancel_proposal` | `get_proposal`, `vote`, `finalize`, `execute`, `cancel_proposal` |
| `DataKey::VotingWeight(Address)` | `u128` | `add_member`, `set_voting_weight` | `create_proposal`, `remove_member`, `set_voting_weight` |

### TTL Behavior

**No TTL extension implemented.** Proposal storage and voting weights will
expire without manual renewal.

---

## 5. Summary: TTL Coverage Gaps

| Contract | Instance TTL | Persistent TTL |
|---|---|---|
| Treasury | ✅ Bumped on write | ✅ Bumped on write |
| Payroll Stream | ❌ Not bumped | ❌ Not bumped |
| Vesting | ❌ Not bumped | ❌ Not bumped |
| Governance | ❌ Not bumped | ❌ Not bumped |

**Risk:** If storage entries expire, users can no longer claim vested tokens,
cancel streams, or finalize proposals. Audit should verify whether Soroban's
default TTL (typically ~1 week on testnet) is sufficient, or whether explicit
TTL extension must be added to all contracts.

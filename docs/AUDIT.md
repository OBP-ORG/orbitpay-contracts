# Audit Scope — OrbitPay Contracts

This document maps every entry point, privileged role, external call, and
event across all four OrbitPay contracts to support an independent security
audit.

---

## 1. Treasury Contract (`contracts/treasury`)

### 1.1 Entry Points

| Function | Auth Required | Role Restriction | Mutates State | External Calls |
|---|---|---|---|---|
| `initialize` | `admin` | First-call only | Yes — storage init | None |
| `deposit` | `from` | Any address | Yes — emits event | `token::transfer` |
| `create_withdrawal` | `proposer` | Must be a signer | Yes — creates proposal | None |
| `approve_withdrawal` | `signer` | Must be a signer | Yes — updates proposal | None |
| `execute_withdrawal` | `executor` | Any address | Yes — transfers funds | `token::Client::transfer` |
| `add_signer` | `admin` | Admin only | Yes — modifies signer list | None |
| `remove_signer` | `admin` | Admin only | Yes — modifies signer list | None |
| `update_threshold` | `admin` | Admin only | Yes — modifies config | None |
| `propose_pause` | `proposer` | Must be a signer | Yes — creates pause proposal | None |
| `approve_pause` | `signer` | Must be a signer | Yes — may activate pause | None |
| `propose_unpause` | `proposer` | Must be a signer | Yes — creates unpause proposal | None |
| `approve_unpause` | `signer` | Must be a signer | Yes — may deactivate pause | None |
| `propose_upgrade` | `proposer` | Must be a signer | Yes — creates upgrade proposal | None |
| `approve_upgrade` | `signer` | Must be a signer | Yes — may approve upgrade | None |
| `execute_upgrade` | `executor` | Any address | Yes — WASM upgrade | `env.deployer()` |
| `propose_admin_change` | `admin` | Admin only | Yes — schedules change | None |
| `execute_admin_change` | `caller` | Any address | Yes — changes admin | None |
| `cancel_admin_change` | `admin` | Admin only | Yes — cancels change | None |
| `propose_emergency_admin_change` | `proposer` | Must be a signer | Yes — creates emergency proposal | None |
| `execute_emergency_admin_change` | `executor` | Any address | Yes — replaces admin | None |
| `get_admin` | None | None | No | None |
| `get_signers` | None | None | No | None |
| `get_threshold` | None | None | No | None |
| `get_withdrawal` | None | None | No | None |
| `get_proposal_count` | None | None | No | None |
| `get_config` | None | None | No | None |

### 1.2 Privileged Roles

| Role | Storage Key | Set During | Scope |
|---|---|---|---|
| Admin | `DataKey::Admin` | `initialize` | Full control: signer mgmt, threshold, upgrade, admin change |
| Signer | `DataKey::Signers` (Vec) | `initialize` / `add_signer` | Can create & approve withdrawals, propose/approve pause/upgrades |

### 1.3 Events Emitted

| Event | Key | Data | Trigger |
|---|---|---|---|
| `init` | `("init",)` | `admin: Address` | `initialize` |
| `deposit` | `("deposit", from)` | `amount: i128` | `deposit` |
| `w_create` | `("w_create", proposer)` | `proposal_id: u32` | `create_withdrawal` |
| `approve` | `("approve", signer)` | `proposal_id: u32` | `approve_withdrawal` |
| `w_exec` | `("w_exec", recipient)` | `amount: i128` | `execute_withdrawal` |
| `s_add` | `("s_add", admin, signer, threshold, version)` | `(signer_count,)` | `add_signer` |
| `s_remove` | `("s_remove", admin, signer, threshold, version)` | `(signer_count,)` | `remove_signer` |
| `t_upd` | `("t_upd", admin, new_threshold, signer_count, version)` | `()` | `update_threshold` |
| `pause_propose` | `("pause_propose", proposer)` | `(proposal_id, reason)` | `propose_pause` |
| `pause_executed` | `("pause_executed", signer)` | `reason` | `approve_pause` |
| `unpause_propose` | `("unpause_propose", proposer)` | `(proposal_id, reason)` | `propose_unpause` |
| `unpause_executed` | `("unpause_executed", signer)` | `reason` | `approve_unpause` |
| `upgrade_proposed` | `("upgrade_proposed",)` | `(proposal_id, description)` | `propose_upgrade` |
| `upgrade_approved` | `("upgrade_approved", signer)` | `proposal_id` | `approve_upgrade` |
| `upgrade_executed` | `("upgrade_executed", executor)` | `(proposal_id, description)` | `execute_upgrade` |
| `admin_change` | `("admin_change", admin)` | `(effective_at, new_admin)` | `propose_admin_change` |
| `admin_changed` | `("admin_changed", old_admin)` | `new_admin` | `execute_admin_change` |
| `admin_change_cancelled` | `("admin_change_cancelled", admin)` | `()` | `cancel_admin_change` |
| `emergency_admin_changed` | `("emergency_admin_changed", executor)` | `old_admin` | `execute_emergency_admin_change` |
| `delay_updated` | `("delay_updated", admin)` | `new_delay` | `update_signer_change_delay` |

### 1.4 Error Codes

22 errors (1–22): `AlreadyInitialized`, `NotInitialized`, `Unauthorized`, `InvalidThreshold`, `NotASigner`, `InvalidAmount`, `ProposalNotFound`, `ProposalNotPending`, `ProposalNotApproved`, `AlreadyApproved`, `AlreadyASigner`, `InsufficientBalance`, `ProposalExpired`, `DuplicateSigner`, `Paused`, `UpgradeProposalNotFound`, `UpgradeProposalNotPending`, `UpgradeAlreadyExecuted`, `TimelockNotExpired`, `InvalidTimelock`, `NoPendingAdminChange`, `UpgradeProposalExecuted`.

---

## 2. Payroll Stream Contract (`contracts/payroll_stream`)

### 2.1 Entry Points

| Function | Auth Required | Role Restriction | Mutates State | External Calls |
|---|---|---|---|---|
| `initialize` | `admin` | First-call only | Yes | None |
| `create_stream` | `sender` | Any address | Yes — creates stream | `token::Client::transfer` |
| `create_batch_streams` | `sender` | Any address | Yes — creates N streams | `token::Client::transfer` |
| `claim` | `recipient` | Must be stream recipient | Yes — transfers tokens | `token::Client::transfer` |
| `cancel_stream` | `sender` | Must be stream sender | Yes — cancels stream | `token::Client::transfer` |
| `get_stream` | None | None | No | None |
| `get_claimable` | None | None | No | None |
| `get_stream_count` | None | None | No | None |
| `get_streams_by_sender` | None | None | No | None |
| `get_streams_by_recipient` | None | None | No | None |
| `get_admin` | None | None | No | None |
| `propose_upgrade` | `admin` | Admin only | Yes — records pending | None |
| `execute_upgrade` | `executor` | Any address | Yes — WASM upgrade | `env.deployer()` |
| `get_pending_upgrade` | None | None | No | None |

### 2.2 Privileged Roles

| Role | Storage Key | Scope |
|---|---|---|
| Admin | `DataKey::Admin` | Propose upgrades |
| Sender | `DataKey::SenderStreams` | Create/cancel their own streams |
| Recipient | `DataKey::RecipientStreams` | Claim tokens from their streams |

### 2.3 Events Emitted

| Event | Key | Data | Trigger |
|---|---|---|---|
| `init` | `("init",)` | `admin: Address` | `initialize` |
| `s_create` | `("s_create", sender)` | `stream_id: u32` | `create_stream` |
| `b_create` | `("b_create", sender)` | `stream_ids: Vec<u32>` | `create_batch_streams` |
| `claim` | `("claim", recipient)` | `claimable: i128` | `claim` |
| `cancel` | `("cancel", sender)` | `stream_id: u32` | `cancel_stream` |
| `upgrade_proposed` | `("upgrade_proposed",)` | `(description, proposed_at)` | `propose_upgrade` |
| `upgrade_executed` | `("upgrade_executed", executor)` | `()` | `execute_upgrade` |

### 2.4 Error Codes

15 errors (1–15): `AlreadyInitialized`, `NotInitialized`, `Unauthorized`, `InvalidAmount`, `InvalidDuration`, `StreamNotFound`, `StreamAlreadyCancelled`, `StreamCompleted`, `NothingToClaim`, `InvalidStartTime`, `InvalidRecipient`, `InsufficientBalance`, `ArithmeticError`, `NoPendingUpgrade`, `TimelockNotExpired`.

### 2.5 Known Issues / Fixes

- **FIXED**: `claim` previously performed a double token transfer (lines 240–244 in old code). This has been corrected to a single `token_client.transfer` call.

---

## 3. Vesting Contract (`contracts/vesting`)

### 3.1 Entry Points

| Function | Auth Required | Role Restriction | Mutates State | External Calls |
|---|---|---|---|---|
| `initialize` | `admin` | First-call only | Yes | None |
| `create_schedule` | `grantor` | Any address | Yes — creates schedule | `token::Client::transfer` |
| `claim` | `beneficiary` | Must be schedule beneficiary | Yes — transfers tokens | `token::Client::transfer` |
| `revoke` | `grantor` | Must be schedule grantor | Yes — transfers unvested | `token::Client::transfer` |
| `get_schedule` | None | None | No | None |
| `get_progress` | None | None | No | None |
| `get_schedules_by_grantor` | None | None | No | None |
| `get_schedules_by_beneficiary` | None | None | No | None |
| `get_claim_history` | None | None | No | None |
| `get_schedule_count` | None | None | No | None |
| `get_admin` | None | None | No | None |
| `propose_upgrade` | `admin` | Admin only | Yes — records pending | None |
| `execute_upgrade` | `executor` | Any address | Yes — WASM upgrade | `env.deployer()` |
| `get_pending_upgrade` | None | None | No | None |

### 3.2 Privileged Roles

| Role | Storage Key | Scope |
|---|---|---|
| Admin | `DataKey::Admin` | Propose upgrades |
| Grantor | `DataKey::GrantorSchedules` | Create/revoke their schedules |
| Beneficiary | `DataKey::BeneficiarySchedules` | Claim from their schedules |

### 3.3 Events Emitted

| Event | Key | Data | Trigger |
|---|---|---|---|
| `init` | `("init",)` | `admin: Address` | `initialize` |
| `v_create` | `("v_create", grantor, beneficiary)` | `(total_amount, cliff_duration, total_duration)` | `create_schedule` |
| `v_claim` | `("v_claim", beneficiary, schedule_id)` | `claimable: i128` | `claim` |
| `v_fully` | `("v_fully", schedule_id)` | `()` | Fully claimed |
| `v_revoke` | `("v_revoke", grantor, schedule_id)` | `unvested: i128` | `revoke` |
| `upgrade_proposed` | `("upgrade_proposed",)` | `(description, proposed_at)` | `propose_upgrade` |
| `upgrade_executed` | `("upgrade_executed", executor)` | `()` | `execute_upgrade` |

### 3.4 Error Codes

14 errors (1–14): `AlreadyInitialized`, `NotInitialized`, `Unauthorized`, `InvalidAmount`, `InvalidSchedule`, `ScheduleNotFound`, `ScheduleRevoked`, `NothingToClaim`, `CliffNotReached`, `AlreadyFullyClaimed`, `InvalidCliffDuration`, `InsufficientBalance`, `NoPendingUpgrade`, `TimelockNotExpired`.

---

## 4. Governance Contract (`contracts/governance`)

### 4.1 Entry Points

| Function | Auth Required | Role Restriction | Mutates State | External Calls |
|---|---|---|---|---|
| `initialize` | `admin` | First-call only | Yes | None |
| `create_proposal` | `proposer` | Must be a DAO member | Yes — creates proposal | None |
| `vote` | `voter` | Must be in proposal snapshot | Yes — records vote | None |
| `finalize` | `caller` | Any address | Yes — determines outcome | None |
| `execute` | `admin` | Admin only | Yes — disburses funds | `token::transfer` (TODO) |
| `cancel_proposal` | `proposer` | Must be original proposer | Yes — cancels proposal | None |
| `add_member` | `admin` | Admin only | Yes — modifies members | None |
| `remove_member` | `admin` | Admin only | Yes — modifies members | None |
| `set_voting_weight` | `admin` | Admin only | Yes — modifies weight | None |
| `get_proposal` | None | None | No | None |
| `get_proposal_count` | None | None | No | None |
| `get_members` | None | None | No | None |
| `get_config` | None | None | No | None |
| `get_proposal_status` | None | None | No | None |
| `get_admin` | None | None | No | None |
| `propose_upgrade` | `admin` | Admin only | Yes — records pending | None |
| `execute_upgrade` | `executor` | Any address | Yes — WASM upgrade | `env.deployer()` |
| `get_pending_upgrade` | None | None | No | None |

### 4.2 Privileged Roles

| Role | Storage Key | Scope |
|---|---|---|
| Admin | `DataKey::Admin` | Member management, weight, execute, proposal upgrades |
| Member | `DataKey::Members` (Vec) | Create proposals, vote |

### 4.3 Snapshot Immutability

Proposals capture a `ProposalSnapshot` at creation time containing:
- `quorum_percentage`, `grace_period`, `total_weight`, and the full `electorate` with per-member weights.

All subsequent vote-eligibility and finalization checks use the **snapshot**, not live state. Admin mutations (add/remove member, weight changes) after proposal creation cannot alter in-flight proposals.

### 4.4 Events Emitted

| Event | Key | Data | Trigger |
|---|---|---|---|
| `init` | `("init",)` | `admin: Address` | `initialize` |
| `p_create` | `("p_create", proposer)` | `proposal_id: u32` | `create_proposal` |
| `vote` | `("vote", voter)` | `proposal_id: u32` | `vote` |
| `finalize` | `("finalize",)` | `status: ProposalStatus` | `finalize` |
| `execute` | `("execute",)` | `proposal_id: u32` | `execute` |
| `p_cancel` | `("p_cancel", proposer)` | `proposal_id: u32` | `cancel_proposal` |
| `m_add` | `("m_add",)` | `new_member: Address` | `add_member` |
| `m_remove` | `("m_remove",)` | `member: Address` | `remove_member` |
| `w_set` | `("w_set", member)` | `new_weight: u128` | `set_voting_weight` |
| `upgrade_proposed` | `("upgrade_proposed",)` | `(description, proposed_at)` | `propose_upgrade` |
| `upgrade_executed` | `("upgrade_executed", executor)` | `()` | `execute_upgrade` |

### 4.5 Error Codes

15 errors (1–15): `AlreadyInitialized`, `NotInitialized`, `Unauthorized`, `ProposalNotFound`, `VotingNotActive`, `AlreadyVoted`, `NotAMember`, `QuorumNotReached`, `ProposalNotApproved`, `ProposalAlreadyExecuted`, `InvalidAmount`, `VotingPeriodExpired`, `ProposalStillActive`, `NoPendingUpgrade`, `TimelockNotExpired`.

---

## 5. Cross-Contract Interaction Map

```
Treasury ─── (transfers tokens) ──→ token::Client
Payroll    ─── (transfers tokens) ──→ token::Client
Vesting    ─── (transfers tokens) ──→ token::Client
Governance ─── TODO: transfer from treasury ──→ Treasury?

All contracts ─── upgrade ──→ env.deployer().update_current_contract_wasm()
```

**Note:** Governance's `execute` function has a TODO for transferring funds from Treasury. The cross-contract call path (Governance → Treasury) is not yet implemented. All upgrades in Payroll Stream, Vesting, and Governance now require a 24-hour timelock after `propose_upgrade`.

---

## 6. Audit Focus Areas

1. **Treasury** — Multi-sig approval: threshold enforcement, signer versioning for in-flight proposals, duplicate signer checks, TTL management, pause state machine correctness.
2. **Payroll Stream** — Fixed double transfer in `claim`; arithmetic in `calculate_claimable`, batch stream validation, timelock upgrade flow.
3. **Vesting** — Cliff + linear vesting math, revocation with partial refunds, integer rounding, timelock upgrade flow.
4. **Governance** — Snapshot immutability guarantees, quorum calculation with weighted voting, grace period edge cases, timelock upgrade flow.
5. **All Contracts** — WASM upgrade access control, initialization guard (one-shot), storage TTL extension consistency.

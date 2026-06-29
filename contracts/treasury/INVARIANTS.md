# Treasury Multisig Invariants

This document describes the core invariants that the Treasury multisig contract maintains to ensure correct and predictable behavior.

## Core Invariants

### 1. Unique Signers
- **Invariant**: All signers in the signer set must be unique addresses.
- **Enforcement**: Validated at initialization time in `initialize()`.
- **Error**: Returns `DuplicateSigner` error if duplicates are detected.
- **Rationale**: Duplicate signers would create misleading approval counts and weaken security guarantees.

### 2. Valid Threshold Range
- **Invariant**: The approval threshold must be greater than 0 and less than or equal to the number of signers.
- **Enforcement**: Validated at initialization in `initialize()` and on updates in `update_threshold()`.
- **Error**: Returns `InvalidThreshold` error if threshold is 0 or exceeds signer count.
- **Rationale**: Threshold 0 would allow unauthorized withdrawals; threshold exceeding signer count would make approvals impossible.

### 3. Threshold 1 Immediate Approval
- **Invariant**: When threshold is 1, a withdrawal is immediately approved upon creation.
- **Enforcement**: In `create_withdrawal()`, if threshold == 1, status is set to `Approved` instead of `Pending`.
- **Rationale**: Prevents stuck withdrawals when only one approval is needed; ensures withdrawals are executable immediately.

### 4. Signer Set Versioning
- **Invariant**: Each withdrawal request records the signer set version at creation time.
- **Enforcement**: `WithdrawalRequest` includes `signer_set_version` field; set to current version in `create_withdrawal()`.
- **Rationale**: Allows deterministic approval checks even when signer set changes; prevents silent invalidation of pending withdrawals.

### 5. Signer Set Version Increment
- **Invariant**: The signer set version increments on every signer addition or removal.
- **Enforcement**: `add_signer()` and `remove_signer()` call `increment_signer_set_version()`.
- **Rationale**: Provides a clear audit trail of signer set changes; enables version-based approval validation.

### 6. Pending Withdrawal Resilience
- **Invariant**: Pending withdrawals are not invalidated by signer set changes.
- **Enforcement**: Withdrawals retain their original `signer_set_version`; approval checks use the signer set at creation time.
- **Rationale**: Prevents stuck withdrawals when signers are added/removed; ensures predictable behavior during signer rotation.

### 7. Duplicate Approval Prevention
- **Invariant**: A signer cannot approve the same withdrawal request more than once.
- **Enforcement**: `approve_withdrawal()` checks if signer is already in the approvals list.
- **Error**: Returns `AlreadyApproved` error if duplicate approval is attempted.
- **Rationale**: Prevents artificial inflation of approval counts; ensures threshold requirements are meaningful.

### 8. Signer Removal Safety
- **Invariant**: A signer cannot be removed if it would make the threshold unachievable.
- **Enforcement**: `remove_signer()` checks if `signers.len() <= threshold` before removal.
- **Error**: Returns `InvalidThreshold` error if removal would violate threshold.
- **Rationale**: Prevents configuration states where approvals become impossible.

## Security Invariants

### 9. Pause Cannot Confiscate Funds
- **Invariant**: Pause mechanism never modifies user token balances.
- **Enforcement**: `pause` only sets `PauseState::Paused`; no token transfer occurs.
- **Rationale**: Pause is an emergency safety mechanism, not a fund seizure tool. User funds remain in the contract and can be withdrawn after unpause.

### 10. No Single-Key Upgrade Authority
- **Invariant**: Contract upgrades require threshold signer approvals, not single admin.
- **Enforcement**: `propose_upgrade()` requires signer authorization; `execute_upgrade()` requires Approved status from multisig.
- **Rationale**: Prevents single point of failure; distributed control protects against compromised admin keys.

### 11. Timelock Minimum Duration
- **Invariant**: Signer change delay cannot be set below 1 day.
- **Enforcement**: `update_signer_change_delay()` enforces `MIN_SIGNER_CHANGE_DELAY` check.
- **Error**: Returns `InvalidTimelock` if below minimum.
- **Rationale**: Provides meaningful window for detecting unauthorized admin changes.

### 12. Emergency Admin Change Requires Threshold
- **Invariant**: Emergency admin change still requires threshold approvals.
- **Enforcement**: `propose_emergency_admin_change()` and `execute_emergency_admin_change()` require threshold approvals.
- **Rationale**: Even in emergency, distributed control prevents unilateral takeover.

## Event Emission

### Signer Addition Event
- **Topic**: `(s_add, admin, new_signer, threshold, new_version)`
- **Data**: `(new_signer_count,)`
- **Emitted by**: `add_signer()`

### Signer Removal Event
- **Topic**: `(s_remove, admin, removed_signer, threshold, new_version)`
- **Data**: `(new_signer_count,)`
- **Emitted by**: `remove_signer()`

### Threshold Update Event
- **Topic**: `(t_upd, admin, new_threshold, signer_count, signer_set_version)`
- **Data**: `()`
- **Emitted by**: `update_threshold()`

### Pause Proposal Event
- **Topic**: `(pause_propose, proposer)`
- **Data**: `(proposal_id, reason)`
- **Emitted by**: `propose_pause()`

### Pause Execution Event
- **Topic**: `(pause_executed, signer)`
- **Data**: `reason`
- **Emitted by**: `approve_pause()` (when threshold met)

### Unpause Proposal Event
- **Topic**: `(unpause_propose, proposer)`
- **Data**: `(proposal_id, reason)`
- **Emitted by**: `propose_unpause()`

### Unpause Execution Event
- **Topic**: `(unpause_executed, signer)`
- **Data**: `reason`
- **Emitted by**: `approve_unpause()` (when threshold met)

### Upgrade Proposal Event
- **Topic**: `(upgrade_proposed,)`
- **Data**: `(proposal_id, description)`
- **Emitted by**: `propose_upgrade()`

### Upgrade Approval Event
- **Topic**: `(upgrade_approved, signer)`
- **Data**: `proposal_id`
- **Emitted by**: `approve_upgrade()`

### Upgrade Execution Event
- **Topic**: `(upgrade_executed, executor)`
- **Data**: `(proposal_id, description)`
- **Emitted by**: `execute_upgrade()`

## Withdrawal Lifecycle

1. **Creation**: Withdrawal created with current signer set version. If threshold == 1, immediately approved.
2. **Approval**: Signers from the withdrawal's signer set version can approve. Duplicate approvals rejected.
3. **Threshold Met**: When approvals >= threshold, status changes to `Approved`.
4. **Execution**: Only withdrawals with `Approved` status can be executed.
5. **Signer Changes**: Do not affect pending withdrawals; they retain their original signer set version.

## Security Considerations

- **Threshold Selection**: Administrators should choose thresholds that balance security and operational needs. Higher thresholds provide more security but require more coordination.
- **Signer Rotation**: When rotating signers, ensure pending withdrawals are either approved or cancelled before removing critical signers if needed.
- **Version Tracking**: The signer set version provides an audit trail but does not restrict approvals based on current signer set. This design prioritizes withdrawal completion over strict version enforcement.
- **Pause Design**: Pause is intentionally narrow-scoped. It blocks token transfers but preserves all user balances and allows signer management to continue for recovery.
- **Emergency Procedures**: Emergency admin change requires same threshold as normal operations, just with signer-initiated flow instead of admin-initiated.

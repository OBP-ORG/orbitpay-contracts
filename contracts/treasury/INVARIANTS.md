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

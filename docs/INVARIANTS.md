# OrbitPay Security Invariants

The following invariants MUST hold true at all times across the OrbitPay smart contract ecosystem.

---

## 1. Initialization Invariants

*   **I-INIT-1:** A contract CANNOT be initialized more than once. `initialize` must revert if the admin is already set.
*   **I-INIT-2:** Initialization MUST require the signature (`require_auth`) of the specified admin address.
*   **I-INIT-3:** Treasury initialization MUST validate that all signers are unique (no duplicates).
*   **I-INIT-4:** Treasury initialization MUST set `signer_set_version` to 0 and `pause_state` to `Unpaused`.

---

## 2. Balance & Arithmetic Invariants

*   **I-BAL-1:** For any active stream or vesting schedule, `claimed_amount <= total_amount`.
*   **I-BAL-2:** The total balance of the contract MUST always be `>=` the sum of `(total_amount - claimed_amount)` for all active streams.
*   **I-BAL-3:** When a stream is cancelled, `refund_to_sender + owed_to_recipient == total_amount - claimed_amount`.
*   **I-BAL-4:** All arithmetic operations MUST be checked. Overflow, underflow, or division by zero MUST revert the transaction.
*   **I-BAL-5:** Treasury balance MUST always be `>=` the sum of amounts for all `Approved` withdrawals.
*   **I-BAL-6:** When `claim` executes, exactly ONE token transfer event MUST occur. (Fixes double-transfer bug.)

---

## 3. Authorization Invariants

*   **I-AUTH-1:** `claim` can ONLY be executed by the designated `recipient`.
*   **I-AUTH-2:** `cancel_stream` can ONLY be executed by the designated `sender` (or Admin, depending on specific policy).
*   **I-AUTH-3:** Treasury `propose_pause`, `approve_pause`, `propose_unpause`, `approve_unpause` can ONLY be called by registered signers.
*   **I-AUTH-4:** Treasury `propose_upgrade`, `approve_upgrade`, `execute_upgrade` require threshold signer approval.
*   **I-AUTH-5:** Treasury `execute_emergency_admin_change` requires proposer to have been a signer at proposal time.
*   **I-AUTH-6:** Payroll Stream, Vesting, and Governance `execute_upgrade` can ONLY execute a pending upgrade that was proposed with `propose_upgrade`.
*   **I-AUTH-7:** Treasury `execute_admin_change` can ONLY execute a pending change whose `effective_at` timestamp has passed.
*   **I-AUTH-8:** No single signer or admin can unilaterally execute withdrawals, upgrades, or admin changes in Treasury.

---

## 4. Timelock Invariants

*   **I-TIME-1:** Treasury `execute_admin_change` MUST check that `now >= effective_at` before executing.
*   **I-TIME-2:** Treasury `propose_signer_add` and `schedule_signer_remove` MUST enforce `signer_change_delay >= MIN_SIGNER_CHANGE_DELAY` (1 day).
*   **I-TIME-3:** Payroll Stream, Vesting, and Governance `execute_upgrade` MUST check that `now >= proposed_at + MIN_UPGRADE_DELAY` (24 hours).
*   **I-TIME-4:** A pending upgrade in Payroll Stream, Vesting, or Governance MUST be cleared after successful execution.
*   **I-TIME-5:** Treasury `execute_emergency_admin_change` bypasses the timelock but requires threshold signer approval.

---

## 5. Pause Invariants

*   **I-PAUSE-1:** When Treasury is paused, deposits and withdrawals MUST be blocked.
*   **I-PAUSE-2:** Pause MUST NOT silently transfer or confiscate user balances.
*   **I-PAUSE-3:** Pause MUST NOT block signer management or pause control itself.
*   **I-PAUSE-4:** Unpause MUST restore normal operation immediately after threshold approvals.

---

## 6. Upgrade Invariants

*   **I-UPG-1:** No single hot key can upgrade contracts or withdraw treasury funds.
*   **I-UPG-2:** Treasury upgrades require threshold multisig approval.
*   **I-UPG-3:** Payroll Stream, Vesting, and Governance upgrades require a 24-hour timelock.
*   **I-UPG-4:** Upgrade proposals MUST be immutable once created (signer set versioning for Treasury).

---

## 7. Signer & Role Rotation Invariants

*   **I-ROT-1:** Pending signer changes MUST be stored with `effective_at` timestamp.
*   **I-ROT-2:** Signer addition MUST increment `signer_set_version`.
*   **I-ROT-3:** Signer removal MUST increment `signer_set_version`.
*   **I-ROT-4:** Pending withdrawals retain the `signer_set_version` at creation time so that original signers can still approve.
*   **I-ROT-5:** Removing a signer MUST NOT reduce the signer count below the current threshold.

---

## 8. Snapshot Invariants (Governance)

*   **I-SNAP-1:** Governance proposals MUST capture a `ProposalSnapshot` at creation time.
*   **I-SNAP-2:** All vote-eligibility and finalization checks MUST use the snapshot, not live state.
*   **I-SNAP-3:** Admin mutations after proposal creation cannot alter in-flight proposal outcomes.
*   **I-SNAP-4:** Members added after proposal creation MUST NOT be in the snapshot electorate and must be rejected when voting.

---

## 9. Event Invariants

*   **I-EVT-1:** Every privileged action MUST emit an event identifying the actor and resource.
*   **I-EVT-2:** Event keys MUST follow the documented topic structure for monitoring and audit.
*   **I-EVT-3:** Failed privileged actions MUST NOT produce state changes (no partial state on revert).

---

## 10. Migration Invariants

*   **I-MIG-1:** Migration MUST preserve all pending withdrawal requests (including approvals).
*   **I-MIG-2:** Migration MUST preserve the signer set with version.
*   **I-MIG-3:** Migration MUST preserve threshold setting.
*   **I-MIG-4:** Migration MUST preserve admin configuration.
*   **I-MIG-5:** Migration MUST NOT silently transfer or confiscate user balances.

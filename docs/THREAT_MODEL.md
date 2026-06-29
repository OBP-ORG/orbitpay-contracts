# OrbitPay Threat Model

## 1. Trust Boundaries and Actors
*   **Admin / Owner:** Trusted to initialize the contracts, configure parameters (e.g., fee rates, quorum limits), and potentially upgrade contracts if upgradeability is enabled. Must securely hold their private key; compromise means total loss of protocol control.
*   **Senders / Organization Managers:** Semi-trusted. Allowed to create streams, fund vesting schedules, and cancel streams (if revocable). Could attempt to create streams with malicious arithmetic or drain contract funds via duplicate refunds.
*   **Recipients / Employees:** Untrusted. Allowed to claim accrued funds. Could attempt to claim before start times, claim more than accrued, or execute reentrancy attacks (though Soroban inherently prevents most reentrancy).
*   **Voters (Governance):** Semi-trusted. Allowed to propose and vote. Could attempt to double-vote, manipulate quorum, or execute malicious payloads.

## 2. Assets Protected
*   **Treasury Tokens:** The bulk of the protocol's TVL.
*   **Payroll & Vesting Allocations:** Tokens locked in the contracts pending time-based release.
*   **Governance Voting Weight:** Represents the control over the treasury and protocol parameters.

## 3. Attack Paths & Mitigations

### 3.1. Arithmetic Overflows & Precision Loss
*   **Threat:** A malicious sender creates a stream with a duration that causes `rate_per_second` to round down to 0, resulting in free stream creation or locked funds.
*   **Mitigation:** `calculate_claimable` computes `total_amount * elapsed / duration` using checked arithmetic, avoiding the reliance on `rate_per_second`. All math operations MUST use `.checked_add()`, `.checked_sub()`, `.checked_mul()`, and `.checked_div()`.

### 3.2. Double Initialization
*   **Threat:** An attacker initializes an uninitialized contract and takes over as Admin.
*   **Mitigation:** Every `initialize` function MUST check `has_admin(&env)` and return `AlreadyInitialized`. The admin caller MUST provide authorization via `admin.require_auth()`.

### 3.3. Unauthorized Access & Privilege Escalation
*   **Threat:** An untrusted user calls `cancel_stream` on someone else's stream or executes a governance proposal.
*   **Mitigation:** Every state-mutating function MUST have a strict authorization policy documented and enforced via `<actor>.require_auth()`.

### 3.4. Duplicate Claims / Duplicate Votes
*   **Threat:** A user claims their stream balance twice in the same ledger, or votes multiple times on the same proposal.
*   **Mitigation:** State updates (e.g., `stream.claimed_amount = new_amount`) MUST happen *before* the cross-contract token transfer to prevent logical reentrancy. Voting records MUST strictly enforce one-vote-per-address.

### 3.5. Fund Draining via Duplicate Transfers
*   **Threat:** A bug in stream creation causes tokens to be transferred twice from the sender, or a cancellation bug refunds more than the remaining balance.
*   **Mitigation:** Code reviews MUST ensure single transfer executions. Invariants MUST guarantee that `refund + owed_to_recipient == remaining_in_contract`.

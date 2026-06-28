# OrbitPay Security Invariants

The following invariants MUST hold true at all times across the OrbitPay smart contract ecosystem:

## 1. Initialization Invariants
*   **I-INIT-1:** A contract CANNOT be initialized more than once. `initialize` must revert if the admin is already set.
*   **I-INIT-2:** Initialization MUST require the signature (`require_auth`) of the specified admin address.

## 2. Balance & Arithmetic Invariants
*   **I-BAL-1:** For any active stream or vesting schedule, `claimed_amount <= total_amount`.
*   **I-BAL-2:** The total balance of the contract MUST always be `>=` the sum of `(total_amount - claimed_amount)` for all active streams.
*   **I-BAL-3:** When a stream is cancelled, `refund_to_sender + owed_to_recipient == total_amount - claimed_amount`.
*   **I-BAL-4:** All arithmetic operations MUST be checked. Overflow, underflow, or division by zero MUST revert the transaction.

## 3. Authorization Invariants
*   **I-AUTH-1:** `claim` can ONLY be executed by the designated `recipient`.
*   **I-AUTH-2:** `cancel_stream` can ONLY be executed by the designated `sender` (or Admin, depending on specific policy).
*   **I-AUTH-3:** `upgrade` can ONLY be executed by the `admin`.
*   **I-AUTH-4:** Governance `execute` can ONLY be called if the proposal is in the `Succeeded` state and the timelock (grace period) has expired.

## 4. Lifecycle & Expiry Invariants
*   **I-LIFE-1:** A stream or vesting schedule CANNOT have an `end_time` less than or equal to its `start_time`.
*   **I-LIFE-2:** A proposal CANNOT be voted on after its `end_time` has passed.
*   **I-LIFE-3:** A stream status CANNOT transition from `Cancelled` to `Active`, nor from `Completed` to `Active`.

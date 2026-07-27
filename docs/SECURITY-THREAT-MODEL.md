# OrbitPay Security Threat Model — Pre-Audit Review

## 1. Document Purpose

This document serves as the **pre-audit security review and threat model** for the OrbitPay four-contract suite (treasury, payroll_stream, vesting, governance). It validates completeness of the production-oriented security model (multisig upgrades, timelocks, pause, signer rotation) and identifies gaps ahead of any mainnet consideration.

This is a decision/artifact document, **not** an implementation deliverable. Findings are tracked as severity-tagged follow-up issues.

---

## 2. System Overview & Architecture

OrbitPay consists of four Soroban smart contracts:

| Contract | Purpose | Upgrade Model |
|----------|---------|---------------|
| **Treasury** | Multi-sig custody, withdrawal approval/execution, pause control | Multisig threshold |
| **Payroll Stream** | Streaming payments with time-based claim | Admin + 24h timelock |
| **Vesting** | Cliff + linear vesting with revocable schedules | Admin + 24h timelock |
| **Governance** | DAO proposal voting, budget disbursement | Admin + 24h timelock |

The Treasury holds all protocol funds. Payroll Stream, Vesting, and Governance interact with the Treasury for fund custody (stream/vesting creation) and disbursement (budget execution).

---

## 3. Asset Inventory

| Asset | Storage Location | Protection Mechanism |
|-------|-----------------|---------------------|
| Treasury tokens (all TVL) | Treasury contract balance | Multi-sig withdrawals, pause, timelocked admin changes |
| Payroll stream allocations | Payroll Stream contract balance | Time-based claimable, sender cancellation |
| Vesting schedule allocations | Vesting contract balance | Cliff protection, grantor revocation, time-based vesting |
| Governance voting weight | Governance contract storage | Snapshot-based immutability, weight-based quorum |
| Admin keys (per contract) | Instance storage | Require_auth, timelocked changes, multisig for Treasury |
| Signer set (Treasury) | Instance storage | Multi-sig changes, version-tracked |

---

## 4. Trust Boundaries & Actors

### 4.1 Trust Boundaries Diagram (Logical)

```
[External Users]
     |
     v
[Token Contracts] <--> Treasury (fund custody)
     ^                 |-- Signers (multi-sig threshold)
     |                 |-- Admin (timelocked changes)
     |
     +-- Payroll Stream (streaming allocations)
     |   |-- Senders (create/cancel)
     |   |-- Recipients (claim)
     |
     +-- Vesting (vested allocations)
     |   |-- Grantors (create/revoke)
     |   |-- Beneficiaries (claim)
     |
     +-- Governance (voting/budget)
         |-- Members (propose/vote)
         |-- Admin (execute)
```

### 4.2 Actor Trust Levels

| Actor | Trust Level | Rationale |
|-------|------------|-----------|
| **Treasury Admin** | High trust | Can add/remove signers, change threshold, propose upgrades (requires multisig) |
| **Treasury Signers** | Medium-high trust | Signers collectively control withdrawals, pause, upgrades via threshold |
| **Payroll/Vesting/Governance Admin** | Medium trust | Can propose upgrades (delayed 24h), manage members, execute proposals |
| **Senders / Grantors** | Medium trust | Fund streams/vesting; could attempt duplicate creation or fund drain |
| **Recipients / Beneficiaries** | Low trust | Can only claim accrued amounts; could attempt pre-claim or over-claim |
| **Governance Members** | Medium-low trust | Can propose/vote; could attempt double-vote, quorum manipulation |
| **External (any address)** | Untrusted | Can only call execute_upgrade after timelock; all other operations require authorisation |

---

## 5. Ranked Threat List (Likelihood x Impact)

Threats are ranked on a scale of **Low / Medium / High / Critical** for both Likelihood (L) and Impact (I). Overall Severity = max(L, I) with critical impact always producing at least High severity.

### 5.1 Critical Severity

| ID | Threat | L | I | Summary |
|----|--------|---|---|---------|
| T-01 | **Admin key compromise of Treasury** | Medium | Critical | Compromised admin could use emergency admin change (bypasses timelock) to take over, then remove all signers and drain funds. |
| T-02 | **Governance→Treasury budget execution without real custody** | Medium | Critical | Governance `execute` calls `token::Client::transfer` from Governance contract address — but Governance may not hold those tokens. If Treasury is the custodian, the cross-contract trust boundary needs explicit design. |

### 5.2 High Severity

| ID | Threat | L | I | Summary |
|----|--------|---|---|---------|
| T-03 | **Treasury signer threshold set to 1** | Low | High | If threshold is 1, a single compromised signer can drain the treasury. Mitigation: init enforces `threshold > 0 && threshold <= signers.len()`, but admin can lower to 1 post-init. |
| T-04 | **Batch stream creation reuses contract address without token check** | Medium | High | `create_batch_streams` calls `token::Client::transfer` per stream with no per-stream balance check or insufficient-funds rollback. If one transfer fails, previous ones are already executed (Soroban atomicity covers this, but needs explicit verification). |
| T-05 | **Payroll Stream double-transfer bug (known issue)** | Medium | High | `create_stream` has been reported to have a double transfer. AUDIT-CHECKLIST line 51 documents this as a known issue. Still unresolved. |
| T-06 | **Vesting `claim` state update after transfer** | Medium | High | `claim` updates `schedule.claimed_amount += claimable` *before* the token transfer (lines 167-185). If the transfer fails, the claimed amount has already been incremented — but the Soroban atomic execution model means the whole tx reverts. Confirm with audit. |
| T-07 | **Emergency admin change bypasses timelock** | Low | High | `execute_emergency_admin_change` requires threshold signer approval but skips timelock. If signers are compromised, instant admin takeover is possible. This is a design trade-off documented in ADR-SECURITY.md. |
| T-08 | **Treasury `deposit` inflow not verified on-chain** | High | High | The `deposit` function calls `token::Client::transfer(from, contract, amount)` but there is no check that the token address is trusted. A malicious token could be deposited with fake balances. Additionally, the contract has no `balance` state variable synced with token balances — deposits and withdrawals manipulate actual token custody directly. The balance is not tracked internally, so off-chain monitors lack a queryable position. |

### 5.3 Medium Severity

| ID | Threat | L | I | Summary |
|----|--------|---|---|---------|
| T-09 | **Governance snapshot bypass via admin mutation** | Low | Medium | Proposals use snapshots, but admin can add members with voting weight and create new proposals. Snapshot only protects in-flight proposals. |
| T-10 | **Vesting `total_amount` vs `original_total_amount` divergence** | Medium | Medium | After revocation, `total_amount` is set to `vested` amount. If claimed + refunded amounts don't equal `original_total_amount`, token accounting breaks. Conservation invariant checked at line 267 but needs fuzz testing. |
| T-11 | **Treasury pause can be bypassed by signer compromise** | Low | Medium | Pause requires threshold multisig. If attacker has threshold signers, they can unpause and drain. Pause is a mitigation, not a prevention. |
| T-12 | **Instance TTL expiry could wipe config** | Medium | Medium | Treasury extends TTL on all entry points (bumps). Payroll Stream, Vesting, and Governance do NOT implement TTL extension (documented known issue: AUDIT-CHECKLIST line 54). If TTL expires, contracts become uninitialized. |
| T-13 | **Signer set version not checked on withdrawal execution** | Low | Medium | `create_withdrawal` stores `signer_set_version` at creation time. `execute_withdrawal` does not check that the current version matches. If signer set changed, approved withdrawal could be executed with a different set of signers. Unlike StellarSentinel, this contract uses a `WithdrawalStatus::Approved` model rather than signer-set-version invalidation. The invariant at I-ROT-4 states "pending withdrawals retain the signer_set_version at creation time so that original signers can still approve" — but this opens a window where a removed signer's approval still counts. |
| T-14 | **Upgrade timelock observation window** | Low | Medium | Payroll/Vesting/Governance upgrades have a 24h timelock, but there is no notification system. A malicious upgrade proposal could go unobserved until execution. |
| T-15 | **Custody correctness: batch withdrawal unfunded edge** | Medium | Medium | The issue references a "ties to the unfunded-batch bug". In `create_batch_streams`, tokens are transferred for each stream, but if a mid-batch transfer fails, the transaction reverts entirely due to Soroban atomicity. The concern is whether the batch proposal validation (Treasury) correctly rejects proposals exceeding available balance. |

### 5.4 Low Severity

| ID | Threat | L | I | Summary |
|----|--------|---|---|---------|
| T-16 | **Integer rounding in calculate_claimable** | Low | Low | `calculate_claimable` uses `total_amount * elapsed / duration` which truncates down. Over many small claims, rounding loss is negligible but could accumulate over very long streams. |
| T-17 | **Event topic collision** | Low | Low | Events use short symbols (e.g., `sa`, `sr`, `pp`, `pe`, `up`, `ue`). If two contracts use the same topic names, monitoring tools may conflate events. Each contract emits to its own contract address, but cross-contract monitors should filter by contract ID. |
| T-18 | **Governance grace period mutable** | Low | Low | `grace_period` is used from proposal snapshot, but admin can change it globally. Only affects new proposals. |
| T-19 | **Missing `initialized` flag check in some query functions** | Low | Low | Some query functions call `require_initialized`, others do not. Pre-initialization read queries return default values silently instead of errors. |

---

## 6. Mitigations vs Gaps Analysis

### 6.1 Existing Strong Mitigations

| Area | Mitigation | Coverage |
|------|-----------|----------|
| Multi-sig custody | Treasury withdrawal execution requires threshold approvals | Full: all withdrawal types |
| Timelocked admin changes | Admin changes require delay (`DEFAULT_SIGNER_CHANGE_DELAY` = 7 days) | Full: Treasury |
| Timelocked upgrades | Payroll/Vesting/Governance upgrades require 24h delay | Full |
| Emergency pause | Treasury can be paused to block deposits/withdrawals without affecting signer management | Full |
| Snapshot governance | Proposals use immutable snapshots of electorate and parameters | Full |
| Checked arithmetic | `checked_add`, `checked_sub`, `checked_mul`, `checked_div` used throughout | Full |
| Reentrancy protection | Soroban inherently prevents cross-contract reentrancy | Full |
| Event audit trail | Every privileged action emits events with actor and resource identification | Full |

### 6.2 Identified Gaps

| Gap | Related Threats | Severity |
|-----|----------------|----------|
| No real-time admin key compromise detection | T-01, T-07 | High |
| Governance↔Treasury trust boundary undefined | T-02 | Critical |
| No per-stream balance check in batch creation | T-04 | High |
| Known double-transfer bug (Payroll Stream) | T-05 | High |
| No TTL extension on non-Treasury contracts | T-12 | Medium |
| Signer set version not invalidated on execution | T-13 | Medium |
| No on-chain balance reconciliation | T-08 | High |
| No notification system for pending upgrades | T-14 | Medium |
| Treasury deposit lacks token allowlist/validation | T-08 | High |

---

## 7. Audit Readiness Assessment

### 7.1 Go / No-Go Recommendation: **CONDITIONAL GO**

The OrbitPay contract suite demonstrates a production-grade security posture with:
- Multi-sig treasury custody
- Timelocked admin changes and upgrades
- Emergency pause mechanism
- Snapshot-based governance
- Checked arithmetic throughout
- Immutable proposal semantics

**However**, the following must be addressed before external audit:

#### Critical Pre-Audit Blockers:
1. **Resolve the known double-transfer bug in Payroll Stream** (T-05). This is a documented fund-drain vulnerability.
2. **Define the Governance→Treasury trust boundary** (T-02). The cross-contract budget execution flow must account for real token custody.
3. **Implement TTL extension on all non-Treasury contracts** (T-12). Expired contract state could lead to permanent lock or uninitialized-state attacks.

#### High-Priority Pre-Audit Items:
4. **Add token allowlist/validation to Treasury deposit** (T-08). Deposits should validate the token address against a whitelist.
5. **Implement signer set version invalidation on withdrawal execution** (T-13). Withdrawals created under an old signer set should require re-approval.
6. **Add per-stream balance checks to batch stream creation** (T-04). Verify aggregate transfer amounts are covered.

#### Recommended (not blocking):
7. Add indexer-friendly reconciliation events for deposit/withdrawal flows.
8. Implement a notification relay for pending upgrade proposals.
9. Add fuzz testing for vesting conservation invariants (T-10).
10. Standardize query function behavior when uninitialized (T-19).

### 7.2 Audit Scope Recommendation

The security audit should focus on:
1. **Custody correctness**: Full trace of token flow across all four contracts, including batch paths and cancellation/refund paths.
2. **Access control completeness**: Verify every entry point's authorization logic against its documented policy.
3. **Arithmetic safety**: Edge cases in `calculate_claimable`, `calculate_vested`, and governance quorum computation.
4. **Multi-sig and timelock bypass paths**: Exhaustive analysis of all privileged operations.
5. **Storage TTL lifecycle**: Instance and persistent storage expiry across all contracts.
6. **Cross-contract trust assumptions**: Treasury↔Governance and Treasury↔Payroll/Vesting interactions.

---

## 8. Follow-Up Hardening Issues

### Critical (must fix before audit)

| Issue | Description | Threats |
|-------|-------------|---------|
| **FIX-DOUBLE-TRANSFER** | Resolve duplicate `token::Client::transfer` in Payroll Stream `create_stream` (lines 73 and 75 of the known double-transfer bug) | T-05 |
| **FIX-GOV-TREASURY-BOUNDARY** | Define and implement the Governance→Treasury trust boundary for budget execution | T-02 |

### High (should fix before audit)

| Issue | Description | Threats |
|-------|-------------|---------|
| **FIX-TTL-NON-TREASURY** | Implement TTL extension (`extend_instance_ttl`) on Payroll Stream, Vesting, and Governance contracts | T-12 |
| **FIX-TOKEN-VALIDATION** | Add token address allowlist or validation to Treasury `deposit` | T-08 |
| **FIX-SIGNER-VERSION** | Validate signer set version on withdrawal execution or invalidate old-version withdrawals | T-13 |
| **FIX-BATCH-CHECKS** | Add aggregate balance checks and per-stream validation in `create_batch_streams` | T-04 |

### Medium (can fix during audit)

| Issue | Description | Threats |
|-------|-------------|---------|
| **HARDEN-THRESHOLD-MINIMUM** | Add a minimum threshold constant (e.g., 2) for Treasury signers that cannot be lowered | T-03 |
| **HARDEN-UPGRADE-NOTIFICATION** | Add event-based upgrade proposal notification for monitoring | T-14 |
| **HARDEN-USDC-WHITELIST** | Implement token whitelist validation on vesting creation and stream creation | T-08 |
| **HARDEN-FUZZ-VESTING** | Add property-based fuzz tests for vesting conservation invariants | T-10 |
| **HARDEN-QUERY-INIT** | Standardize query functions to return `NotInitialized` error consistently | T-19 |
| **HARDEN-FUNDING-VALIDATION** | Add `InsufficientBalance` checks before token transfers at Treasury deposit | T-08 |

### Low (nice to have)

| Issue | Description | Threats |
|-------|-------------|---------|
| **IMPROVE-EVENT-TOPICS** | Use consistent, namespaced event topic naming across all contracts | T-17 |
| **IMPROVE-ROUNDING-DOCS** | Document expected rounding behavior in `calculate_claimable` and `calculate_vested` | T-16 |
| **IMPROVE-BALANCE-RECONCILIATION** | Add on-chain queryable balance tracking for off-chain reconciliation | T-08 |

---

## 9. Methodology & Assumptions

### 9.1 Threat Modeling Methodology
- **STRIDE** per component: Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege
- **Attack tree** per asset: Root goal = "drain treasury funds" or "execute unauthorized withdrawal"
- **Invariant-based analysis**: Each invariant in `docs/INVARIANTS.md` was stress-tested against contract code

### 9.2 Assumptions
1. Soroban's atomic execution model correctly reverts all state on any failure.
2. `require_auth()` correctly verifies Ed25519 signatures.
3. Ledger timestamps are monotonic and not manipulable.
4. Contract storage TTL extension works as documented.
5. The Stellar network consensus provides eventual finality.
6. Admin keys are held in secure custody (hardware wallets or multisig).

### 9.3 Out of Scope
- Network-level attacks (DDoS, Eclipse)
- Compiler-level vulnerabilities (Rust compiler, Soroban SDK)
- Stellar Core consensus vulnerabilities
- Frontend/web application security
- Key management infrastructure
- Social engineering attacks on signers/admins

---

## 10. References

- [INVARIANTS.md](./INVARIANTS.md) — Formal security invariants
- [ADR-SECURITY.md](./ADR-SECURITY.md) — Architecture decision records for security
- [AUDIT-CHECKLIST.md](./AUDIT-CHECKLIST.md) — Pre-audit readiness checklist
- [THREAT_MODEL.md](./THREAT_MODEL.md) — Initial threat model (superseded by this document)
- [STORAGE.md](./STORAGE.md) — Storage layout documentation
- [RUNBOOK.md](./RUNBOOK.md) — Operational runbook
- [AUDIT.md](./AUDIT.md) — Audit scope and entry point documentation

---

*Document version: 1.0 — Pre-audit security review*
*Date: July 2026*
*Status: Complete (SPIKE deliverable)*

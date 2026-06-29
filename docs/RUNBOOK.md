# OrbitPay Production Security Runbook

## Overview

This runbook defines procedures for security incident response, key rotation, upgrades, and recovery for OrbitPay production contracts. All actions require multi-signature consensus from designated signers or adhere to enforced timelocks.

---

## Decision Makers

### Security Council (Threshold Signers)

| Role | Identifier | Responsibility |
|------|-----------|----------------|
| Security Lead | `signer_1` | Primary incident coordinator; declares emergencies |
| Operations Lead | `signer_2` | Operational security procedures; executes pause/resume |
| Finance Lead | `signer_3` | Treasury fund management oversight |

**Threshold**: 2 of 3 signers required for all privileged actions.

**Admin Key Holder** (single key for administrative tasks):
- Responsible for proposing upgrades, adding/removing signers, and scheduling admin changes.
- Protected by timelock: any upgrade proposed by admin cannot execute until 24 hours have elapsed.

### Escalation Path

1. **Level 1 (Operational)**: Security Lead + Operations Lead (within 2 hours)
2. **Level 2 (Critical)**: All 3 Security Council members (within 24 hours)
3. **Level 3 (Emergency)**: External security partners (if internal signers compromised)

---

## Response Time Targets (SLA)

| Incident Type | Detection to Acknowledgment | Detection to Mitigation | Full Resolution |
|---------------|---------------------------|------------------------|-----------------|
| Vulnerability (non-exploited) | 2 hours | 12 hours | 7 days |
| Active Exploit | 30 minutes | 2 hours | 72 hours |
| Compromised Key (signer) | 1 hour | 4 hours | 24 hours |
| Compromised Admin | 1 hour | 4 hours | 24 hours |
| Protocol Upgrade | 24 hours planning | N/A | Scheduled rollout |

---

## Procedures

### 1. Emergency Pause Procedure (Treasury)

When a vulnerability is detected or exploit is suspected:

```
Step 1: Detect incident
├── Monitor alerts from security monitoring
├── Review transaction patterns
└── Confirm threat assessment

Step 2: Initiate pause
├── Any signer calls propose_pause()
│   ├── Arguments: proposer=sender, reason="incident_description"
│   └── Emits: (pause_propose, proposer, (proposal_id, reason))
└── Second signer calls approve_pause(proposal_id)
    ├── Validates proposal exists
    ├── Adds approval
    └── If threshold met, sets pause state and emits (pause_executed, signer, reason)

Step 3: Post-pause actions
├── Audit all pending withdrawals
├── Document incident details
├── Engage security auditors
└── Prepare remediation plan
```

**Pause Effects**:
- All deposits and withdrawals blocked.
- Signer management functions remain available.
- No funds are moved or frozen — balances preserved.

**Pause Invariant**: The pause mechanism does NOT silently transfer or confiscate user balances. It only blocks token transfers; existing balances remain in the contract.

### 2. Vulnerability Response Flow

```
┌─────────────────────────────────────────────────────────────┐
│                     PAUSE TREASURY                           │
│  Signer1.propose_pause → Signer2.approve_pause              │
└─────────────────────────────────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────┐
│                  DIAGNOSE THREAT                            │
│  • Review suspicious transactions                           │
│  • Identify affected funds                                  │
│  • Determine root cause                                      │
│  • Assess impact scope                                       │
└─────────────────────────────────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────┐
│                 IMPLEMENT REMEDIATION                       │
│  Option A: Contract upgrade (Timelock path)                  │
│    Admin.propose_upgrade → [Wait 24 hours] →                 │
│    anyone.execute_upgrade                                    │
│                                                              │
│  Option B: Treasury multi-sig upgrade                        │
│    Signer.propose_upgrade → Signers.approve_upgrade →         │
│    executor.execute_upgrade                                  │
│                                                              │
│  Option C: Withdraw and migrate                             │
│    Signer.create_withdrawal → Signers.approve →               │
│    execute_withdrawal (to migration address)                  │
└─────────────────────────────────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────┐
│               VERIFY REMEDIATION                            │
│  • Confirm fix deployed                                      │
│  • Audit state preservation                                  │
│  • Validate no unauthorized changes                          │
│  • Multi-sig verification of fix                             │
└─────────────────────────────────────────────────────────────┘
                               ↓
┌─────────────────────────────────────────────────────────────┐
│                  RESUME OPERATIONS                            │
│  Signer1.propose_unpause → Signer2.approve_unpause          │
└─────────────────────────────────────────────────────────────┘
```

### 3. Compromised Signer Procedure

When a signer key is suspected compromised:

```
Step 1: Confirm compromise
├── Signer cannot authenticate transactions
├── Suspicious activity detected on signer address
└── Security alert triggered

Step 2: Emergency admin change (if admin compromised)
├── Signer.propose_emergency_admin_change(new_admin)
├── Signers.approve_upgrade(proposal_id)
└── Signer.execute_emergency_admin_change(new_admin)

Step 3: Rotate signer
├── Admin.add_signer(new_signer)
├── Admin.remove_signer(compromised_signer)
└── Verify signer set via get_signers()
```

### 4. Admin Key Rotation

Scheduled admin rotation (timelocked):

```
Step 1: Schedule change
├── Admin.propose_admin_change(new_admin)
│   └── Emits (admin_change, admin, (effective_at, new_admin))
└── Wait for signer_change_delay (default: 7 days)

Step 2: Execute after delay
├── Any address calls execute_admin_change()
│   └── Sets new admin, emits (admin_changed, old_admin, new_admin)
└── Cancel possible before execution via cancel_admin_change()
```

### 5. Contract Upgrade Procedure

#### Treasury (Multisig Upgrade)

```
Step 1: Proposal
├── Any signer calls propose_upgrade()
│   ├── Args: proposer, wasm_hash, description
│   └── Emits (upgrade_proposed, (proposal_id, description))
│
Step 2: Approval
├── Signers call approve_upgrade(proposal_id)
│   └── Emits (upgrade_approved, signer, proposal_id)
│
Step 3: Execution
├── Any address calls execute_upgrade(executor, proposal_id)
│   └── Final WASM upgrade, emits (upgrade_executed, executor, proposal_id)

Step 4: Post-upgrade verification
├── Testnet drill (mandatory)
├── Verify all state preserved
├── Monitor for anomalies
└── Publish upgrade report
```

#### Payroll Stream / Vesting / Governance (Timelock Upgrade)

```
Step 1: Proposal
├── Admin calls propose_upgrade()
│   ├── Args: admin, wasm_hash, description
│   └── Emits (upgrade_proposed, (description, proposed_at))
│
Step 2: Wait for timelock
└── MIN_UPGRADE_DELAY (24 hours) must elapse
    ├── Query via get_pending_upgrade()
    └── Security council monitors during delay window

Step 3: Execution
├── Any address calls execute_upgrade(executor)
│   └── Final WASM upgrade, emits (upgrade_executed, executor)

Step 4: Post-upgrade verification
├── Testnet drill (mandatory)
├── Verify all state preserved
├── Monitor for anomalies
└── Publish upgrade report
```

**Emergency Override**: If a critical vulnerability is discovered during the timelock window, the Security Council can:
1. Pause the Treasury (blocking malicious fund movements).
2. Coordinate with the admin to propose an emergency fix.
3. If admin is compromised, use the Treasury's emergency admin change procedure.

### 6. Migration Procedure

When deploying a new treasury contract:

```
Step 1: Deploy new contract
├── Deploy TreasuryContract v2
├── Initialize with same admin/signers/threshold
└── Verify deployment on testnet

Step 2: Transfer state
├── Query all pending withdrawals: get_withdrawal(id)
├── Signer set preserved with version tracking
├── Threshold and configuration copied
└── Balances preserved via token transfer (no silent confiscation)

Step 3: Cutover
├── Create withdrawal from old contract
├── Transfer all token balances
├── Update protocol integrations to new address
└── Keep old contract paused for safety period

Step 4: Resume
├── Unpause new contract
├── Monitor for anomalies
└── Archive old contract
```

**Migration Invariant**: Migration preserves balances and all active protocol state. No balances are silently transferred or confiscated during migration.

### 7. Recovery from Double-Transfer or Token Anomaly (Payroll Stream)

The previously identified double-transfer bug in `claim` has been fixed. Recovery procedure for any residual anomaly:

```
Step 1: Detect
├── Monitor contract token balance vs expected sum(stream amounts)
├── Cross-check with recipient claim records
└── Identify anomaly source

Step 2: Pause (Treasury blocks cross-contract fund movements)
├── Signer1.propose_pause
└── Signer2.approve_pause

Step 3: Diagnose
├── Review all claim transactions
├── Identify over-withdrawn amounts
└── Prepare remediation transaction

Step 4: Remediate
├── Option A: Governance proposes refund (if funded)
├── Option B: Emergency admin change to recovery address
└── Option C: Manual compensation outside protocol

Step 5: Resume
├── Signer1.propose_unpause
└── Signer2.approve_unpause
```

---

## Event Monitoring

All privileged actions emit events with the following topic structure:

| Topic Pattern | Event Type | Data |
|---------------|------------|------|
| `("init",)` | Contract initialization | `admin: Address` |
| `("pause_propose", proposer)` | Pause proposal | `(proposal_id, reason)` |
| `("pause_executed", signer)` | Pause activated | `reason` |
| `("unpause_propose", proposer)` | Unpause proposal | `(proposal_id, reason)` |
| `("unpause_executed", signer)` | Unpause activated | `reason` |
| `("upgrade_proposed",)` | Upgrade proposal | `(proposal_id, description)` or `(description, proposed_at)` |
| `("upgrade_approved", signer)` | Upgrade approval | `proposal_id` |
| `("upgrade_executed", executor)` | Upgrade executed | `()` |
| `("admin_change", admin)` | Admin change scheduled | `(effective_at, new_admin)` |
| `("admin_changed", old_admin)` | Admin changed | `new_admin` |
| `("admin_change_cancelled", admin)` | Admin change cancelled | `()` |
| `("emergency_admin_changed", executor)` | Emergency admin changed | `old_admin` |
| `("s_add", admin, signer, threshold, version)` | Signer added | `(new_signer_count,)` |
| `("s_remove", admin, signer, threshold, version)` | Signer removed | `(new_signer_count,)` |
| `("t_upd", admin, new_threshold, signer_count, version)` | Threshold updated | `()` |
| `("delay_updated", admin)` | Delay updated | `new_delay` |
| `("w_create", proposer)` | Withdrawal created | `proposal_id` |
| `("approve", signer)` | Withdrawal approved | `proposal_id` |
| `("w_exec", recipient)` | Withdrawal executed | `amount` |
| `("deposit", from)` | Deposit | `amount` |
| `("s_create", sender)` | Stream created | `stream_id` |
| `("claim", recipient)` | Claimed | `claimable: i128` |
| `("cancel", sender)` | Stream cancelled | `stream_id` |
| `("v_create", grantor, beneficiary)` | Vesting schedule created | `(total_amount, cliff_duration, total_duration)` |
| `("v_claim", beneficiary, schedule_id)` | Vesting claimed | `claimable: i128` |
| `("v_fully", schedule_id)` | Fully claimed | `()` |
| `("v_revoke", grantor, schedule_id)` | Schedule revoked | `unvested: i128` |
| `("p_create", proposer)` | Proposal created | `proposal_id` |
| `("vote", voter)` | Vote cast | `proposal_id` |
| `("finalize",)` | Proposal finalized | `status: ProposalStatus` |
| `("execute",)` | Proposal executed | `proposal_id` |

---

## Testnet Drill Requirements

Before production upgrades or parameter changes:

1. **Pause Drill**: Execute full pause/unpause cycle on testnet.
2. **Upgrade Drill**: Deploy and test upgrade flow with state preservation.
3. **Recovery Drill**: Simulate recovery from compromised signer scenario.
4. **Migration Drill**: Deploy new contract, migrate state, verify balances.
5. **Audit Drill**: Verify all events emitted correctly and query functions work.

Document results in `docs/testnet-drills/YYYY-MM-DD.md`.

---

## Audit Scope

Recovery mechanisms are included in external audit scope:

- Pause functionality and state transitions (Treasury)
- Upgrade proposal and approval flow (all contracts)
- Admin change timelock and emergency paths (Treasury)
- Signer rotation procedures (Treasury)
- Timelock upgrade flow (Payroll Stream, Vesting, Governance)
- Event emission for all privileged actions (all contracts)
- Migration state preservation (Treasury)

---

## Contact Information

| Role | Contact Method | Response SLA |
|------|---------------|--------------|
| Security Council | multisig@orbitpay.fyi | 24/7 |
| Audit Firm | audit@external.com | Business hours |
| Incident Response | incident@orbitpay.fyi | 1 hour |

---

## Key Rotation Checklist

- [ ] Verify new key stored in HSM or secure enclave
- [ ] Confirm key has never been used on an insecure device
- [ ] Update signing infrastructure
- [ ] Test with non-critical operation first
- [ ] Document in security council records
- [ ] Update this runbook with new signer addresses

---

## Treasury INVARIANTS

Refer to `contracts/treasury/INVARIANTS.md` for the complete set of security invariants enforced by the Treasury contract.

Key invariants:
- No single signer can execute withdrawals or upgrades.
- Pause cannot transfer or confiscate balances.
- Timelock delays are bounded by minimum values.
- Signer set versioning preserves in-flight withdrawal approvals.
- All privileged actions are event-emitted with actor identification.

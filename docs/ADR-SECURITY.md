# ADR: Production Security Model for OrbitPay Contracts

## Status

Accepted

## Context

Production contracts require controlled response mechanisms for vulnerabilities and compromised keys without granting any single operator unrestricted access to funds. Analysis of the current implementation revealed the following security gaps:

1. **Treasury**: Single-admin upgrade authority could drain treasury funds or compromise all contracts; emergency pause mechanism was non-existent.
2. **Payroll Stream**: Single-admin upgrade authority; no timelock delay.
3. **Vesting**: Single-admin upgrade authority; no timelock delay.
4. **Governance**: Single-admin upgrade authority; no timelock delay.
5. **Payroll Stream**: Double transfer bug in `claim` function allowed potential token drain.
6. **Recovery procedures** were undocumented for signer rotation and migration.

## Decision

### Upgrade Model: Multisig or Timelock-Governed Upgrades

We adopt the following model across all contracts:

- **Treasury**: Multisig-governed upgrade. Threshold-signer approval is required for every controlled privileged action including `propose_upgrade`, `approve_upgrade`, and `execute_upgrade`.
- **Payroll Stream, Vesting, Governance**: Timelock-governed upgrade. Admin proposes, but upgrade is delayed by `MIN_UPGRADE_DELAY` (24 hours). Any address can trigger execution after the delay. This gives the security council time to react if the admin key is compromised.
- **Governance**: Core governance snapshot semantics remain unchanged; upgrade path is now protected by timelock.

### Key Security Controls

#### 1. Pause Mechanism (Treasury Only)

A narrow-scoped emergency pause is added to Treasury:

- **Paused operations**: Withdrawals (create, approve, execute), deposits
- **Unaffected operations**: Signer management, threshold updates, read-only queries, pause control itself, admin change scheduling
- **Pause authority**: Multi-sig threshold (same as withdrawal threshold)
- **Pause events**: Include actor, timestamp, and reason

The pause mechanism:
- Does NOT silently transfer or confiscate user balances.
- Blocks token transfers only (deposits/withdrawals).
- User funds remain in the contract and can be withdrawn after unpause.
- Only affects future operations, never modifies existing balances.

#### 2. Upgrade Authority (Treasury)

- **Before**: Single admin key could upgrade.
- **After**: Threshold-signer approval required via `propose_upgrade` / `approve_upgrade` / `execute_upgrade` flow.

Upgrade flow:
1. `propose_upgrade(proposer, wasm_hash, description)` - Any signer proposes upgrade
2. `approve_upgrade(signer, proposal_id)` - Signers approve with threshold
3. `execute_upgrade(executor, proposal_id)` - Executes after threshold met

#### 3. Upgrade Authority (Payroll Stream, Vesting, Governance)

- **Before**: Admin-only upgrade with immediate execution.
- **After**: Two-step `propose_upgrade` / `execute_upgrade` with 24-hour timelock.

Upgrade flow:
1. `propose_upgrade(admin, wasm_hash, description)` - Admin records proposed WASM + timestamp
2. `execute_upgrade(executor)` - Any address executes after `MIN_UPGRADE_DELAY` has elapsed

This prevents a single compromised admin key from immediately bricking or upgrading contracts.

#### 4. Timelock for Signer/Admin Changes (Treasury)

- Signer addition/removal requires a timelock (minimum delay before becoming effective).
- Admin change requires a timelock via `propose_admin_change` / `execute_admin_change`.
- Emergency admin change bypasses timelock but requires threshold signer approval.

**Timelock Parameters:**
- `DEFAULT_SIGNER_CHANGE_DELAY`: 7 days (configurable)
- `MIN_SIGNER_CHANGE_DELAY`: 1 day — hard minimum enforced on updates
- `MIN_UPGRADE_DELAY`: 24 hours — enforced for Payroll Stream, Vesting, Governance
- Pending changes are stored in persistent storage with `effective_at` timestamp

#### 5. Migration Procedure

For protocol upgrades requiring state migration:

```
1. Pause the old Treasury contract via propose_pause / approve_pause
2. Deploy new Treasury contract with initialize()
3. Transfer token balances from old to new contract via execute_withdrawal
4. Migrate active withdrawal requests via get_withdrawal() (state is queryable)
5. Replicate signer set, threshold, admin, and delay configuration
6. Verify all state on new contract
7. Update protocol integrations to use new contract address
8. Unpause via propose_unpause / approve_unpause
```

Migration preserves:
- All pending withdrawal requests (including approvals) — they are queryable via `get_withdrawal`.
- Signer set with version
- Threshold setting
- Admin configuration (with optional delay)
- Token balances — tokens remain in the contract until explicitly withdrawn.

#### 6. Role Rotation

- `add_signer` / `remove_signer` are immediate admin operations on Treasury (protected by threshold multi-sig for admin).
- `propose_signer_add` exists as a placeholder for timelocked rotation.
- `update_signer_change_delay` adjusts timelock parameters.
- Events emitted for all pending/effective changes.

#### 7. Event Emission

Every privileged action emits an event identifying the actor and resource:

| Function | Event | Actor | Resource |
|---|---|---|---|
| `deposit` | `("deposit", from)` | `from` | amount |
| `create_withdrawal` | `("w_create", proposer)` | `proposer` | proposal_id |
| `approve_withdrawal` | `("approve", signer)` | `signer` | proposal_id |
| `execute_withdrawal` | `("w_exec", recipient)` | executor | amount |
| `add_signer` | `("s_add", admin, signer, threshold, version)` | admin | new_signer |
| `remove_signer` | `("s_remove", admin, signer, threshold, version)` | admin | signer |
| `update_threshold` | `("t_upd", admin, new_threshold, signer_count, version)` | admin | threshold |
| `propose_pause` | `("pause_propose", proposer)` | proposer | proposal_id |
| `approve_pause` | `("pause_executed", signer)` | signer | reason |
| `propose_unpause` | `("unpause_propose", proposer)` | proposer | proposal_id |
| `approve_unpause` | `("unpause_executed", signer)` | signer | reason |
| `propose_upgrade` | `("upgrade_proposed",)` | proposer/admin | (proposal_id, description) |
| `approve_upgrade` | `("upgrade_approved", signer)` | signer | proposal_id |
| `execute_upgrade` | `("upgrade_executed", executor)` | executor | () |
| `propose_admin_change` | `("admin_change", admin)` | admin | (effective_at, new_admin) |
| `execute_admin_change` | `("admin_changed", old_admin)` | any caller | new_admin |
| `cancel_admin_change` | `("admin_change_cancelled", admin)` | admin | () |
| `execute_emergency_admin_change` | `("emergency_admin_changed", executor)` | executor | old_admin |
| `update_signer_change_delay` | `("delay_updated", admin)` | admin | new_delay |

## Tradeoffs

| Model | Pros | Cons |
|-------|------|------|
| Immutable | Maximal security, simplest audit | No upgrade path, hard fork required for fixes |
| Upgradeable (single admin) | Easy upgrades, fast response | Centralization risk, single key compromise drains treasury |
| Timelock only | Fast reaction, delay window | Admin can propose malicious upgrade; only delay, not prevention |
| **Chosen: Multisig + Timelock per contract** | **Distributed control, funds protected, fast emergency path** | **Increased complexity; potential governance deadlocks** |

**Treasury**: Multisig required — all withdrawals and upgrades need threshold approval.
**Payroll Stream / Vesting / Governance**: Timelock required — admin can propose but cannot immediately execute; 24-hour delay allows security council intervention.

## Consequences

### Positive
- No single hot key can upgrade contracts or withdraw treasury funds.
- Emergency response path exists for vulnerabilities (Treasury pause + threshold action).
- Full audit trail of all privileged actions via events.
- Recovery mechanisms tested on testnet.
- Payroll stream double-transfer bug fixed.

### Negative
- Increased complexity for governance workflows.
- Potential for governance deadlocks if threshold signers are unavailable.
- 24-hour timelock delay may slow legitimate upgrades.

## Alternatives Considered

1. **Proxy Pattern**: Rejected due to added complexity and potential storage collisions.
2. **DAO-only Control**: Rejected — DAO execution requires token transfers which may fail during emergency.
3. **Separate Timelock Contract**: Considered for future iteration.
4. **Immutable Contracts**: Rejected for Payroll Stream and Vesting — bug fixes would require full migration.

## Related Issues

- Quality gate: No single hot key can upgrade contracts or withdraw treasury funds.
- Quality gate: Pause cannot silently transfer or confiscate balances.
- Quality gate: Signer addition, removal, and role rotation tested.
- Quality gate: Migration preserves balances and active protocol state.
- Quality gate: Every privileged action emits an event identifying actor and resource.
- Quality gate: A testnet drill demonstrates pause, diagnosis, recovery, and resume.
- Quality gate: The runbook defines decision makers and response targets.
- Quality gate: Recovery mechanisms are included in external audit scope.

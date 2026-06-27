# Audit & Mainnet Release Checklist — OrbitPay Contracts

This checklist must be fully signed off before mainnet deployment.

---

## 1. Audit Scope Completeness

- [ ] Every entry point documented in `docs/AUDIT.md` with auth requirements
- [ ] Every privileged role identified and scoped
- [ ] All external calls (`token::Client::transfer`, `env.deployer()`) mapped
- [ ] All events emitted by each contract cataloged
- [ ] All error codes enumerated with trigger conditions

## 2. Storage & TTL Audit

- [ ] Storage layout documented in `docs/STORAGE.md`
- [ ] Instance vs Persistent storage correctly categorized
- [ ] TTL extension behavior reviewed (currently only Treasury implements TTL bumps)
- [ ] Long-lived storage (vesting schedules, proposals) has TTL plan
- [ ] No storage keys can collide across contracts

## 3. Access Control

- [ ] `initialize()` is strictly one-shot (guarded by `has_admin` check)
- [ ] Admin-only functions check `stored_admin == caller`
- [ ] `upgrade()` is restricted to admin on all contracts
- [ ] Treasury: only signers can create/approve withdrawals
- [ ] Governance: only members can create proposals and vote
- [ ] Governance: proposal snapshot prevents post-creation admin interference
- [ ] Vesting: only grantor can revoke, only if `revocable == true`

## 4. Arithmetic & Overflow

- [ ] All `i128` arithmetic uses `checked_*` operations where appropriate
- [ ] Vesting cliff + linear calculation reviewed for rounding errors
- [ ] Payroll stream `calculate_claimable` reviewed for edge cases
- [ ] Governance quorum calculation reviewed for integer division
- [ ] No unchecked arithmetic in token transfers

## 5. Token Transfer Integrity

- [ ] All `token::Client::transfer` calls preceded by balance checks
- [ ] No double transfers (Payroll Stream known issue: duplicate transfer)
- [ ] Transfers use correct `from` and `to` addresses
- [ ] Insufficient balance is caught before attempting transfer

## 6. Known Issues (Pre-Audit)

- [ ] **Payroll Stream**: Duplicate `transfer` call in `create_stream` (lines 73 and 75)
- [ ] **Payroll Stream**: `create_batch_streams` has a TODO for batch transfer optimization
- [ ] **Treasury**: `deposit` has a TODO for invoking token transfer
- [ ] **Governance**: `execute` has a TODO for transferring funds from treasury
- [ ] **All contracts except Treasury**: No TTL extension implemented

## 7. Reproducible Build

- [ ] `scripts/build.sh` produces deterministic artifacts
- [ ] Build manifest records commit, toolchain, and checksums
- [ ] Same commit + same toolchain = same WASM hashes on independent builds
- [ ] WASM files stored in versioned `artifacts/` directory

## 8. Testnet Soak

- [ ] All contracts deployed to testnet
- [ ] Treasury: multi-sig withdrawal flow tested end-to-end
- [ ] Payroll Stream: stream creation → claim → cancel tested
- [ ] Vesting: schedule creation → cliff vesting → full vesting → revoke tested
- [ ] Governance: proposal creation → voting → finalization → execution tested
- [ ] No unresolved critical or high-severity defects
- [ ] All test suite passes: `cargo test --all`

## 9. Independent Audit

- [ ] Third-party auditor engaged
- [ ] Audit scope document (`docs/AUDIT.md`) provided
- [ ] Source code frozen at audit commit
- [ ] All critical and high findings addressed
- [ ] Remediation verified and retested
- [ ] Audit report published

## 10. Mainnet Launch

- [ ] Signed release checklist (this document) by all required approvers
- [ ] Deployment manifest (`artifacts/deploy-mainnet-*.json`) generated
- [ ] Deployer key secured (hardware wallet or multisig)
- [ ] Transaction hashes recorded in deployment manifest
- [ ] Published source at deployment commit verifiable
- [ ] Deployed WASM hash matches published checksum
- [ ] Emergency pause/upgrade procedure documented

---

## Sign-off

| Role | Name | Signature | Date |
|---|---|---|---|
| Lead Developer | | | |
| Security Auditor | | | |
| Protocol Admin | | | |
| Operations | | | |

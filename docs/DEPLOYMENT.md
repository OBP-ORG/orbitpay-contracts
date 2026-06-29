# Reproducible Build & Deployment — OrbitPay Contracts

This document describes how to produce deterministic WASM artifacts, verify
bytecode integrity, and deploy contracts with an auditable manifest.

---

## 1. Reproducible Build

### Prerequisites

- Rust toolchain: `1.78+` (wasm32-unknown-unknown target)
- Soroban CLI: `soroban-cli 22.0+`

### Install the WASM target

```bash
rustup target add wasm32-unknown-unknown
```

### Deterministic Build Procedure

```bash
# Freeze the Rust toolchain version
rustup show

# Build all contracts with deterministic settings
./scripts/build.sh
```

The build script (`scripts/build.sh`):
1. Records the git commit hash and toolchain version
2. Builds each contract with `--target wasm32-unknown-unknown --release`
3. Computes SHA-256 checksums of each `.wasm` artifact
4. Writes a build manifest to `artifacts/build-manifest.json`

### Build Profile (from workspace `Cargo.toml`)

```toml
[profile.release]
opt-level = "z"         # Optimize for size
overflow-checks = true  # Safety: panic on overflow
debug = 0               # Strip debug info
strip = "symbols"       # Strip symbol table
debug-assertions = false
panic = "abort"         # Abort on panic (smaller binary)
codegen-units = 1       # Single codegen unit for determinism
lto = true              # Link-time optimization
```

### Verifying Reproducibility

Two independent builds with the same commit and toolchain MUST produce
identical checksums. Verify with:

```bash
# Build on machine A
./scripts/build.sh
cat artifacts/build-manifest.json

# Build on machine B (same commit)
./scripts/build.sh
diff <(cat artifacts/build-manifest.json) <(ssh machine-a cat artifacts/build-manifest.json)
```

---

## 2. Deployment

### Pre-deployment Checklist

- [ ] All tests pass: `cargo test --all`
- [ ] Deterministic build completed and checksums recorded
- [ ] Target network confirmed (testnet / mainnet)
- [ ] Deployer identity key configured
- [ ] Required token contracts deployed (if dependency)

### Testnet Deployment

```bash
# Deploy all contracts
./scripts/deploy.sh testnet

# Deploy a specific contract
./scripts/deploy.sh testnet treasury
```

The deploy script:
1. Reads the build manifest for WASM hashes
2. Requires explicit network confirmation (prompts user)
3. Deploys each contract via Soroban CLI
4. Invokes `initialize()` with the deployer as admin
5. Writes a deployment manifest to `artifacts/deploy-{network}-{timestamp}.json`

### Deployment Manifest Format

```json
{
  "network": "testnet",
  "timestamp": "2026-06-27T12:00:00Z",
  "commit": "abc123def456",
  "toolchain": "1.78.0",
  "contracts": [
    {
      "name": "treasury",
      "wasm_hash": "sha256:abc123...",
      "contract_id": "C...",
      "transaction_hash": "tx:def456...",
      "signers": ["G..."]
    }
  ]
}
```

### Mainnet Deployment

```bash
# Requires explicit flag to confirm mainnet
./scripts/deploy.sh mainnet --confirm
```

The `--confirm` flag is MANDATORY for mainnet. Without it, the script aborts
with a warning.

---

## 3. Post-Deployment Verification

### Verify Deployed Bytecode

```bash
soroban contract inspect --wasm artifacts/treasury.wasm
soroban contract inspect --network testnet --id <contract_id>
```

Compare the WASM hash from the build manifest with the on-chain contract code.

### Verify Source-to-Bytecode

Published source at the deployment commit can be independently rebuilt:

```bash
git checkout <deployment-commit>
./scripts/build.sh
sha256sum artifacts/treasury.wasm
# Must match the deployed on-chain WASM hash
```

---

## 4. Contract Upgrade

Each contract exposes an `upgrade(admin, new_wasm_hash)` entry point:

```bash
soroban contract invoke \
  --id <contract_id> \
  --source <admin_key> \
  --network testnet \
  -- upgrade \
  --admin <admin_address> \
  --new_wasm_hash <bytesn32_hash>
```

**Audit note:** `upgrade` is the highest-risk entry point. It must be
restricted to admin and audited for proper access control.

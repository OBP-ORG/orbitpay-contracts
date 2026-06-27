#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# OrbitPay Contracts — Deployment Script
#
# Deploys contracts to the specified Stellar network with explicit confirmation.
# Produces an auditable deployment manifest.
#
# Usage:
#   ./scripts/deploy.sh testnet [contract_name]
#   ./scripts/deploy.sh mainnet --confirm [contract_name]
# ──────────────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
ARTIFACTS_DIR="$REPO_ROOT/artifacts"

NETWORK="${1:-}"
CONFIRM="${2:-}"
FILTER_CONTRACT="${3:-}"

# ── Validate network ─────────────────────────────────────────────────────────

if [[ "$NETWORK" != "testnet" ]] && [[ "$NETWORK" != "mainnet" ]]; then
    echo "Usage: $0 <testnet|mainnet> [--confirm] [contract_name]"
    echo ""
    echo "  testnet    Deploy to Stellar testnet"
    echo "  mainnet    Deploy to Stellar mainnet (requires --confirm)"
    echo "  --confirm  Required flag for mainnet deployment"
    exit 1
fi

# Handle positional --confirm
if [[ "$CONFIRM" == "--confirm" ]]; then
    FILTER_CONTRACT="${3:-}"
elif [[ "$NETWORK" == "mainnet" ]]; then
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  MAINNET DEPLOYMENT REQUIRES --confirm FLAG                ║"
    echo "║  Usage: ./scripts/deploy.sh mainnet --confirm              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    exit 1
fi

# ── Prerequisites ────────────────────────────────────────────────────────────

if ! command -v soroban &>/dev/null; then
    echo "ERROR: soroban CLI not found. Install with: cargo install soroban-cli"
    exit 1
fi

MANIFEST="$ARTIFACTS_DIR/build-manifest.json"
if [[ ! -f "$MANIFEST" ]]; then
    echo "ERROR: Build manifest not found. Run ./scripts/build.sh first."
    exit 1
fi

GIT_COMMIT="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo 'unknown')"
DEPLOYER="$(soroban config identity address default 2>/dev/null || echo 'UNCONFIGURED')"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# ── Mainnet confirmation ─────────────────────────────────────────────────────

if [[ "$NETWORK" == "mainnet" ]]; then
    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  ⚠️  MAINNET DEPLOYMENT CONFIRMATION                       ║"
    echo "╠══════════════════════════════════════════════════════════════╣"
    echo "║  Network:    mainnet                                       ║"
    echo "║  Deployer:   $DEPLOYER"
    echo "║  Commit:     $GIT_COMMIT"
    echo "║  Timestamp:  $TIMESTAMP"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
    read -r -p "Type 'DEPLOY TO MAINNET' to confirm: " confirm_input
    if [[ "$confirm_input" != "DEPLOY TO MAINNET" ]]; then
        echo "Deployment aborted."
        exit 1
    fi
    echo ""
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  OrbitPay Contracts — $NETWORK Deployment"
echo "  Deployer:  $DEPLOYER"
echo "  Commit:    $GIT_COMMIT"
echo "  Timestamp: $TIMESTAMP"
echo "═══════════════════════════════════════════════════════════════"

# ── Deploy contracts ─────────────────────────────────────────────────────────

CONTRACTS=$(jq -r '.contracts[].name' "$MANIFEST")
DEPLOY_MANIFEST="{\"network\":\"$NETWORK\",\"timestamp\":\"$TIMESTAMP\",\"commit\":\"$GIT_COMMIT\",\"deployer\":\"$DEPLOYER\",\"contracts\":[]}"
DEPLOY_ENTRIES="["

first=true
for contract in $CONTRACTS; do
    if [[ -n "$FILTER_CONTRACT" ]] && [[ "$contract" != "$FILTER_CONTRACT" ]]; then
        continue
    fi

    WASM_HASH=$(jq -r ".contracts[] | select(.name == \"$contract\") | .sha256" "$MANIFEST")
    WASM_PATH="$ARTIFACTS_DIR/${contract}.wasm"

    echo ""
    echo "───────────────────────────────────────────────────────────────"
    echo "  Deploying: $contract"
    echo "  WASM hash: $WASM_HASH"
    echo "───────────────────────────────────────────────────────────────"

    # Deploy via soroban CLI
    CONTRACT_ID=$(soroban contract deploy \
        --wasm "$WASM_PATH" \
        --source default \
        --network "$NETWORK" \
        2>&1 | tail -1)

    TX_HASH="unknown"

    echo "  Contract ID: $CONTRACT_ID"

    if [[ "$first" == true ]]; then
        first=false
    else
        DEPLOY_ENTRIES+=","
    fi
    DEPLOY_ENTRIES+="{\"name\":\"$contract\",\"wasm_sha256\":\"$WASM_HASH\",\"contract_id\":\"$CONTRACT_ID\",\"transaction_hash\":\"$TX_HASH\"}"
done

DEPLOY_ENTRIES+="]"

# ── Write deployment manifest ────────────────────────────────────────────────

DEPLOY_MANIFEST=$(jq -n \
    --arg network "$NETWORK" \
    --arg timestamp "$TIMESTAMP" \
    --arg commit "$GIT_COMMIT" \
    --arg deployer "$DEPLOYER" \
    --argjson contracts "$DEPLOY_ENTRIES" \
    '{network: $network, timestamp: $timestamp, commit: $commit, deployer: $deployer, contracts: $contracts}')

MANIFEST_PATH="$ARTIFACTS_DIR/deploy-${NETWORK}-$(date -u +"%Y%m%d-%H%M%S").json"
echo "$DEPLOY_MANIFEST" | jq '.' > "$MANIFEST_PATH"

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Deployment complete"
echo "  Manifest: $MANIFEST_PATH"
echo "═══════════════════════════════════════════════════════════════"

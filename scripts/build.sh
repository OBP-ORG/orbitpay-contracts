#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# OrbitPay Contracts — Deterministic Reproducible Build
#
# Produces WASM artifacts with checksums and a build manifest for audit.
#
# Usage: ./scripts/build.sh
# ──────────────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
ARTIFACTS_DIR="$REPO_ROOT/artifacts"

CONTRACTS=("treasury" "payroll_stream" "vesting" "governance")
TARGET="wasm32-unknown-unknown"

# ── Toolchain & commit info ──────────────────────────────────────────────────

RUST_VERSION="$(rustc --version 2>/dev/null || echo 'unknown')"
GIT_COMMIT="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo 'unknown')"
GIT_TAG="$(git -C "$REPO_ROOT" describe --tags --exact-match 2>/dev/null || echo 'untagged')"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

echo "═══════════════════════════════════════════════════════════════"
echo "  OrbitPay Contracts — Reproducible Build"
echo "  Commit:   $GIT_COMMIT"
echo "  Tag:      $GIT_TAG"
echo "  Toolchain: $RUST_VERSION"
echo "  Timestamp: $TIMESTAMP"
echo "═══════════════════════════════════════════════════════════════"

# ── Ensure target ────────────────────────────────────────────────────────────

if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    echo "Installing target: $TARGET"
    rustup target add "$TARGET"
fi

# ── Clean build ──────────────────────────────────────────────────────────────

mkdir -p "$ARTIFACTS_DIR"
rm -f "$ARTIFACTS_DIR"/*.wasm "$ARTIFACTS_DIR"/checksums.txt

echo ""
echo "Building contracts..."
cargo build --target "$TARGET" --release

# ── Copy WASM artifacts & compute checksums ──────────────────────────────────

echo ""
echo "Collecting WASM artifacts..."

MANIFEST_JSON='{"network":"build","timestamp":"'$TIMESTAMP'","commit":"'$GIT_COMMIT'","toolchain":"'$RUST_VERSION'","tag":"'$GIT_TAG'","contracts":[]}'

# Use jq to build JSON array
CONTRACT_ENTRIES="["

for contract in "${CONTRACTS[@]}"; do
    WASM_PATH="$REPO_ROOT/target/$TARGET/release/${contract//-/_}.wasm"

    if [[ -f "$WASM_PATH" ]]; then
        cp "$WASM_PATH" "$ARTIFACTS_DIR/${contract}.wasm"
        HASH="$(sha256sum "$ARTIFACTS_DIR/${contract}.wasm" | awk '{print $1}')"
        echo "  $contract → $HASH"

        if [[ "$contract" != "${CONTRACTS[0]}" ]]; then
            CONTRACT_ENTRIES+=","
        fi
        CONTRACT_ENTRIES+="{\"name\":\"$contract\",\"wasm\":\"${contract}.wasm\",\"sha256\":\"$HASH\"}"
    else
        echo "  ERROR: $contract WASM not found at $WASM_PATH"
        exit 1
    fi
done

CONTRACT_ENTRIES+="]"

# ── Write build manifest ─────────────────────────────────────────────────────

MANIFEST_JSON=$(jq -n \
    --arg network "build" \
    --arg timestamp "$TIMESTAMP" \
    --arg commit "$GIT_COMMIT" \
    --arg toolchain "$RUST_VERSION" \
    --arg tag "$GIT_TAG" \
    --argjson contracts "$CONTRACT_ENTRIES" \
    '{network: $network, timestamp: $timestamp, commit: $commit, toolchain: $toolchain, tag: $tag, contracts: $contracts}')

echo "$MANIFEST_JSON" | jq '.' > "$ARTIFACTS_DIR/build-manifest.json"

# ── Write checksums file ─────────────────────────────────────────────────────

for contract in "${CONTRACTS[@]}"; do
    sha256sum "$ARTIFACTS_DIR/${contract}.wasm" >> "$ARTIFACTS_DIR/checksums.txt"
done

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  Build complete"
echo "  Artifacts: $ARTIFACTS_DIR"
echo "  Manifest:  $ARTIFACTS_DIR/build-manifest.json"
echo "═══════════════════════════════════════════════════════════════"

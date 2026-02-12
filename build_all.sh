#!/bin/bash
set -e

# Build script for Paxi Network Smart Contracts
# Optimized for CI and local development

# 1. Setup build directory
mkdir -p artifacts

# 2. Sequential Build
# We use -C target-feature=-bulk-memory,-sign-ext to ensure compatibility with older WASM runtimes.
echo "Building all contracts..."
RUSTFLAGS="-C target-feature=-bulk-memory,-sign-ext -C link-arg=-s" cargo build --release --target wasm32-unknown-unknown

# 3. Optimize and Move artifacts
CONTRACTS=("lp_locker" "reward_controller" "prc20_vesting")

for CONTRACT in "${CONTRACTS[@]}"; do
    echo "Optimizing $CONTRACT..."
    
    # Check if wasm-opt is available
    if command -v wasm-opt > /dev/null; then
        # We enable the features for wasm-opt so it can process the file even if they are present.
        # This allows the optimization pass to complete.
        wasm-opt -Oz \
            --enable-bulk-memory \
            --enable-sign-ext \
            "target/wasm32-unknown-unknown/release/$CONTRACT.wasm" \
            -o "artifacts/$CONTRACT.wasm"
    else
        echo "Warning: wasm-opt not found, using unoptimized build."
        cp "target/wasm32-unknown-unknown/release/$CONTRACT.wasm" "artifacts/$CONTRACT.wasm"
    fi
done

echo "Build complete. Artifacts are in the artifacts/ directory."
ls -lh artifacts/

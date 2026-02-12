#!/bin/bash
set -e

# Build script for Paxi Network Smart Contracts
# Optimized for CI and local development

# 1. Setup build directory
mkdir -p artifacts

# 2. Sequential Build (avoiding memory issues in CI)
# Building from the root workspace
echo "Building all contracts..."
RUSTFLAGS="-C link-arg=-s" cargo build --release --target wasm32-unknown-unknown

# 3. Optimize and Move artifacts
CONTRACTS=("lp_locker" "reward_controller" "prc20_vesting")

for CONTRACT in "${CONTRACTS[@]}"; do
    echo "Optimizing $CONTRACT..."
    
    # Check if wasm-opt is available
    if command -v wasm-opt > /dev/null; then
        # Note: --enable-bulk-memory/sign-ext might be needed for wasm-opt to read the file
        # but the target chain might reject them. Adjust flags if necessary.
        wasm-opt -Oz \
            --signext-lowering \
            "target/wasm32-unknown-unknown/release/$CONTRACT.wasm" \
            -o "artifacts/$CONTRACT.wasm"
    else
        echo "Warning: wasm-opt not found, using unoptimized build."
        cp "target/wasm32-unknown-unknown/release/$CONTRACT.wasm" "artifacts/$CONTRACT.wasm"
    fi
done

echo "Build complete. Artifacts are in the artifacts/ directory."
ls -lh artifacts/

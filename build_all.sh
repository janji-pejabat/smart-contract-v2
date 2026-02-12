#!/usr/bin/env bash
# build_all.sh - Build both LP Platform contracts without bulk-memory

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo -e "${GREEN}Building LP Platform Contracts (CosmWasm Compatible)...${NC}"

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm

CONTRACTS=("lp-locker" "reward-controller")

for contract in "${CONTRACTS[@]}"; do
    CONTRACT_NAME_SNAKE="${contract//-/_}"
    echo "Building $contract..."
    
    cd "contracts/$contract"
    cargo clean --quiet
    
    # Build with bulk-memory and sign-ext disabled at compile time
    RUSTFLAGS='-C target-feature=-bulk-memory,-sign-ext' cargo build --release --target wasm32-unknown-unknown --quiet
    
    WASM_PATH="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

    # Optimize using wasm-opt if available
    if command -v wasm-opt &> /dev/null; then
        echo "Optimizing $contract..."
        # Use --lower-bulk-memory and --lower-sign-ext if supported by your wasm-opt version
        # Otherwise use strictly MVP features
        wasm-opt -Oz --strip-debug --mvp-features "$WASM_PATH" -o "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    else
        echo "wasm-opt not found, copying raw WASM..."
        cp "$WASM_PATH" "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    fi
    
    # Validate using cosmwasm-check if available
    if command -v cosmwasm-check &> /dev/null; then
        echo "Validating $contract..."
        cosmwasm-check "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    fi

    # Show file size
    SIZE=$(du -h "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm" | cut -f1)
    echo -e "${GREEN}✓ Created artifacts/${CONTRACT_NAME_SNAKE}.wasm ($SIZE)${NC}"

    cd ../..
done

echo -e "${GREEN}✅ Build Complete! Artifacts are ready for upload.${NC}"

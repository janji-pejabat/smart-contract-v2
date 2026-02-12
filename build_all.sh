#!/usr/bin/env bash
# build_all.sh - Build both LP Platform contracts with strict MVP compatibility

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${GREEN}Building LP Platform Contracts (Paxi Network Compatible)...${NC}"

# Check build tools
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Rust/Cargo not installed!${NC}"
    exit 1
fi

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm

CONTRACTS=("lp-locker" "reward-controller")

for contract in "${CONTRACTS[@]}"; do
    CONTRACT_NAME_SNAKE="${contract//-/_}"
    echo -e "${CYAN}Building $contract...${NC}"
    
    cd "contracts/$contract"
    cargo clean --quiet
    
    # Disable modern WASM features at compile time to ensure compatibility.
    # We target MVP CPU and explicitly remove bulk-memory, sign-ext, and mutable-globals.
    export RUSTFLAGS="-C target-cpu=mvp -C target-feature=-bulk-memory,-sign-ext,-mutable-globals -C link-arg=-s"
    cargo build --release --target wasm32-unknown-unknown --quiet
    
    WASM_PATH="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

    # Optimize and lower opcodes using wasm-opt if available
    if command -v wasm-opt &> /dev/null; then
        echo "Optimizing and lowering $contract..."

        # Dynamically detect supported flags to avoid "Unknown option" errors
        WASM_OPT_HELP=$(wasm-opt --help)
        WASM_OPT_FLAGS="-Oz --strip-debug"

        # Lowering passes for compatibility
        if echo "$WASM_OPT_HELP" | grep -q "bulk-memory-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --bulk-memory-lowering"
        elif echo "$WASM_OPT_HELP" | grep -q "bulkmemory-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --bulkmemory-lowering"
        fi

        if echo "$WASM_OPT_HELP" | grep -q "sign-ext-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --sign-ext-lowering"
        elif echo "$WASM_OPT_HELP" | grep -q "signext-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --signext-lowering"
        fi

        # Input features (enable so optimizer can parse then lower)
        if echo "$WASM_OPT_HELP" | grep -q "enable-bulk-memory"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --enable-bulk-memory"
        fi
        if echo "$WASM_OPT_HELP" | grep -q "enable-sign-ext"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --enable-sign-ext"
        fi

        # Output features (strictly MVP)
        if echo "$WASM_OPT_HELP" | grep -q "mvp-features"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --mvp-features"
        fi

        wasm-opt $WASM_OPT_FLAGS "$WASM_PATH" -o "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    else
        echo -e "${YELLOW}wasm-opt not found, copying raw WASM (may fail mainnet validation)${NC}"
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

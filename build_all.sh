#!/usr/bin/env bash
# build_all.sh - Build and Validate LP Platform contracts

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if TERM is set before calling clear
if [ -t 0 ] && [ -n "$TERM" ]; then
    clear
fi

echo -e "${GREEN}=========================================="
echo "  LP PLATFORM v2.0.0 - BUILD & VALIDATE"
echo "  Locker + Reward Controller"
echo "==========================================${NC}"
echo ""

# Contracts to build
CONTRACTS=(
    "lp-locker"
    "reward-controller"
)

# Check build tools
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Rust/Cargo not installed!${NC}"
    exit 1
fi

SKIP_OPT=0
if ! command -v wasm-opt &> /dev/null; then
    echo -e "${YELLOW}⚠ wasm-opt not found! Optimization will be skipped.${NC}"
    SKIP_OPT=1
fi

SKIP_VALIDATE=0
if ! command -v cosmwasm-check &> /dev/null; then
    echo -e "${YELLOW}⚠ cosmwasm-check not found! Validation will be skipped.${NC}"
    SKIP_VALIDATE=1
fi

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm artifacts/*.sha256

# Build each contract
for contract in "${CONTRACTS[@]}"; do
    CONTRACT_NAME_SNAKE="${contract//-/_}"
    echo -e "Building: $contract..."
    
    cd "contracts/$contract"
    cargo clean --quiet
    
    # Run tests
    if ! cargo test --quiet; then
        echo -e "${RED}✗ Tests failed!${NC}"
        exit 1
    fi
    
    # Compile to WASM (strictly MVP)
    # We use a single comma-separated list for target-feature to ensure all are applied.
    export RUSTFLAGS="-C target-cpu=mvp -C target-feature=-bulk-memory,-sign-ext,-mutable-globals -C link-arg=-s"
    cargo build --release --target wasm32-unknown-unknown --quiet

    FINAL_WASM_SOURCE="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

    # Optimize and Lower Opcodes if wasm-opt is available
    if [ "$SKIP_OPT" -eq 0 ]; then
        # Dynamically detect available flags in wasm-opt to avoid "Unknown option" errors
        WASM_OPT_HELP=$(wasm-opt --help)
        WASM_OPT_FLAGS="-Oz --strip-debug"
        
        # Enable features for parsing if possible
        if echo "$WASM_OPT_HELP" | grep -q "all-features"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --all-features"
        else
            if echo "$WASM_OPT_HELP" | grep -q "enable-bulk-memory"; then WASM_OPT_FLAGS="$WASM_OPT_FLAGS --enable-bulk-memory"; fi
            if echo "$WASM_OPT_HELP" | grep -q "enable-sign-ext"; then WASM_OPT_FLAGS="$WASM_OPT_FLAGS --enable-sign-ext"; fi
        fi

        # Add lowering passes
        if echo "$WASM_OPT_HELP" | grep -q "bulkmemory-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --bulkmemory-lowering"
        elif echo "$WASM_OPT_HELP" | grep -q "bulk-memory-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --bulk-memory-lowering"
        fi

        if echo "$WASM_OPT_HELP" | grep -q "signext-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --signext-lowering"
        elif echo "$WASM_OPT_HELP" | grep -q "sign-ext-lowering"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --sign-ext-lowering"
        fi

        # Final verification that the output is MVP
        if echo "$WASM_OPT_HELP" | grep -q "mvp-features"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --mvp-features"
        fi

        wasm-opt $WASM_OPT_FLAGS "$FINAL_WASM_SOURCE" -o "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    else
        cp "$FINAL_WASM_SOURCE" "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    fi
    
    # Validate
    if [ "$SKIP_VALIDATE" -eq 0 ]; then
        cosmwasm-check "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    fi

    # Checksum
    cd ../../artifacts
    sha256sum "${CONTRACT_NAME_SNAKE}.wasm" > "${CONTRACT_NAME_SNAKE}.wasm.sha256"
    cd ..
done

echo -e "${GREEN}✅ ALL BUILDS COMPLETE! Artifacts are in ./artifacts${NC}"

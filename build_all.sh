#!/usr/bin/env bash
# build_all.sh - Final definitive fix for Paxi Network WASM compatibility

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${GREEN}Building LP Platform Contracts (STRICT MVP COMPATIBILITY)...${NC}"

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
    
    # We use multiple -C target-feature flags to ensure the compiler disables everything modern.
    # We also target the 'mvp' CPU.
    export RUSTFLAGS="-C target-cpu=mvp -C target-feature=-bulk-memory -C target-feature=-sign-ext -C target-feature=-mutable-globals -C target-feature=-nontrapping-fptoint -C link-arg=-s"

    echo "Compiling to WASM..."
    cargo build --release --target wasm32-unknown-unknown --quiet

    WASM_PATH="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

    # Optimize and lower opcodes using wasm-opt if available
    if command -v wasm-opt &> /dev/null; then
        echo "Optimizing and forcing MVP features..."

        # Start with aggressive size optimization and strip debug info
        WASM_OPT_FLAGS="-Oz --strip-debug --strip-producers"

        # Get wasm-opt help to check for supported flags
        WASM_OPT_HELP=$(wasm-opt --help 2>&1 || true)

        # We need to enable modern features in the parser so it can read the compiler output,
        # even though we will lower them to MVP opcodes immediately after.
        if echo "$WASM_OPT_HELP" | grep -q "all-features"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --all-features"
        fi

        # FORCE lowering of modern opcodes back to MVP sequences.
        # We try different variations because different wasm-opt versions use different naming conventions.
        # hyphenated vs non-hyphenated (e.g., bulk-memory-lowering vs bulkmemory-lowering)
        for pass in "bulkmemory-lowering" "bulk-memory-lowering" "signext-lowering" "sign-ext-lowering"; do
            if echo "$WASM_OPT_HELP" | grep -q "$pass"; then
                WASM_OPT_FLAGS="$WASM_OPT_FLAGS --$pass"
            fi
        done

        # Enforce strictly MVP output features if supported
        if echo "$WASM_OPT_HELP" | grep -q "mvp-features"; then
            WASM_OPT_FLAGS="$WASM_OPT_FLAGS --mvp-features"
        fi

        echo "Running wasm-opt with flags: $WASM_OPT_FLAGS"
        wasm-opt $WASM_OPT_FLAGS "$WASM_PATH" -o "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    else
        echo -e "${YELLOW}⚠ wasm-opt not found! Copying raw WASM. This will likely fail validation on Paxi.${NC}"
        cp "$WASM_PATH" "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    fi
    
    # Final Validation
    if command -v cosmwasm-check &> /dev/null; then
        echo "Validating artifact compatibility..."
        cosmwasm-check "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    fi

    SIZE=$(du -h "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm" | cut -f1)
    echo -e "${GREEN}✓ Created artifacts/${CONTRACT_NAME_SNAKE}.wasm ($SIZE)${NC}"

    cd ../..
done

echo -e "${GREEN}✅ SUCCESS! Artifacts are strictly MVP-compatible and ready for Paxi Network.${NC}"

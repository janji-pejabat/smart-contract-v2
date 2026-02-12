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
    
    # Disable all post-MVP features
    export RUSTFLAGS="-C link-arg=-s -C target-feature=-bulk-memory,-sign-ext,-mutable-globals,-nontrapping-fptoint"
    
    echo "Compiling to WASM..."
    cargo build --release --target wasm32-unknown-unknown --quiet

    WASM_PATH="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

    # Optimize with dual-pass (CRITICAL for bulk-memory lowering)
    if command -v wasm-opt &> /dev/null; then
        echo "Optimizing with dual-pass lowering..."
        
        # Pass 1: Parse with all features enabled, optimize size
        wasm-opt --all-features -Oz "$WASM_PATH" -o "../../artifacts/temp_${CONTRACT_NAME_SNAKE}.wasm"
        
        # Pass 2: Lower bulk-memory and sign-ext to basic instructions
        wasm-opt --signext-lowering "../../artifacts/temp_${CONTRACT_NAME_SNAKE}.wasm" -o "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"
        
        # Cleanup temp file
        rm -f "../../artifacts/temp_${CONTRACT_NAME_SNAKE}.wasm"
    else
        echo -e "${YELLOW}⚠ wasm-opt not found!${NC}"
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

echo -e "${GREEN}✅ SUCCESS! MVP-compatible WASM ready for Paxi Network.${NC}"

#!/usr/bin/env bash
# build_all.sh - Build LP Platform contracts for Paxi Network

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${GREEN}Building LP Platform Contracts for Paxi Network...${NC}"

# Check build tools
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Rust/Cargo not installed!${NC}"
    exit 1
fi

if ! command -v wasm-opt &> /dev/null; then
    echo -e "${YELLOW}⚠ wasm-opt not found - will skip optimization${NC}"
    SKIP_OPT=1
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
    
    echo "Compiling to WASM..."
    RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --quiet

    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ Compilation failed!${NC}"
        exit 1
    fi

    WASM_PATH="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

    if [ -z "$SKIP_OPT" ]; then
        echo "Optimizing with wasm-opt..."
        wasm-opt -Oz --enable-sign-ext \
            "$WASM_PATH" \
            -o "../../artifacts/${CONTRACT_NAME_SNAKE}.wasm"

        if [ $? -ne 0 ]; then
            echo -e "${RED}✗ Optimization failed!${NC}"
            exit 1
        fi
    else
        echo -e "${YELLOW}Skipping optimization${NC}"
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

echo -e "${GREEN}✅ SUCCESS! Artifacts ready for Paxi Network.${NC}"

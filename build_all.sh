#!/usr/bin/env bash
# build_all.sh - Build and Validate LP Platform contracts

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
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

# Validate project structure
echo -e "${CYAN}Validating project structure...${NC}"
for contract in "${CONTRACTS[@]}"; do
    if [ ! -d "contracts/$contract" ]; then
        echo -e "${RED}✗ contracts/$contract not found!${NC}"
        echo -e "${YELLOW}Please ensure project structure is correct${NC}"
        exit 1
    fi
done
echo -e "${GREEN}✓ All contracts found${NC}"
echo ""

# Check build tools
echo -e "${CYAN}Checking build tools...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Rust/Cargo not installed!${NC}"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

# wasm-opt check
if ! command -v wasm-opt &> /dev/null; then
    echo -e "${YELLOW}⚠ wasm-opt not found! Optimization will be skipped.${NC}"
    echo -e "To install (Ubuntu): ${CYAN}sudo apt-get install binaryen${NC}"
    echo -e "To install (macOS): ${CYAN}brew install binaryen${NC}"
    SKIP_OPT=1
else
    echo -e "${GREEN}✓ wasm-opt found${NC}"
fi

# cosmwasm-check check
if ! command -v cosmwasm-check &> /dev/null; then
    echo -e "${YELLOW}⚠ cosmwasm-check not found! Artifact validation will be skipped.${NC}"
    echo -e "To install: ${CYAN}cargo install cosmwasm-check${NC}"
    SKIP_VALIDATE=1
else
    echo -e "${GREEN}✓ cosmwasm-check found${NC}"
fi
echo ""

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm artifacts/*.sha256

# Build each contract
for contract in "${CONTRACTS[@]}"; do
    CONTRACT_NAME_SNAKE="${contract//-/_}"
    
    echo -e "${BLUE}======================================"
    echo "  Building: $contract"
    echo "======================================${NC}"
    echo ""
    
    cd "contracts/$contract"
    
    # Clean previous builds
    echo -e "${CYAN}[1/5] Cleaning build environment...${NC}"
    cargo clean --quiet

    # Step 2: Run tests
    echo -e "${CYAN}[2/5] Running tests...${NC}"
    if cargo test --quiet; then
        echo -e "${GREEN}✓ Tests passed${NC}"
    else
        echo -e "${RED}✗ Some tests failed!${NC}"
        if [ -t 0 ]; then
            echo -e "${YELLOW}Continue anyway? (y/n)${NC}"
            read -r response
            if [[ ! "$response" =~ ^[Yy]$ ]]; then exit 1; fi
        else
            exit 1
        fi
    fi
    echo ""
    
    # Step 3: Compile to WASM
    echo -e "${CYAN}[3/5] Compiling to WASM (strictly MVP)...${NC}"
    # Target MVP CPU and disable modern extensions to ensure compatibility with older chains
    export RUSTFLAGS="-C target-cpu=mvp -C target-feature=-bulk-memory -C target-feature=-sign-ext -C target-feature=-mutable-globals -C link-arg=-s"
    cargo build --release --target wasm32-unknown-unknown --quiet
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Compilation successful${NC}"
    else
        echo -e "${RED}✗ Compilation failed!${NC}"
        exit 1
    fi
    echo ""
    
    # Step 4: Optimize with wasm-opt
    if [ -z "$SKIP_OPT" ]; then
        echo -e "${CYAN}[4/5] Optimizing and Lowering Opcodes...${NC}"
        # We use explicit lowering to resolve "bulk memory support is not enabled" errors.
        # --all-features allows parsing the input, then we lower everything to MVP.
        wasm-opt -Oz \
            --all-features \
            --bulkmemory-lowering \
            --signext-lowering \
            --strip-debug \
            "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm" \
            -o "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}_optimized.wasm"
        
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓ Optimization & Lowering successful${NC}"
            FINAL_WASM_SOURCE="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}_optimized.wasm"
        else
            echo -e "${RED}✗ Optimization failed!${NC}"
            exit 1
        fi
    else
        echo -e "${YELLOW}[4/5] Skipping optimization (Artifact may fail chain validation)${NC}"
        FINAL_WASM_SOURCE="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"
    fi
    echo ""
    
    # Step 5: Validate and Copy Artifact
    FINAL_WASM_NAME="${CONTRACT_NAME_SNAKE}.wasm"
    cp "$FINAL_WASM_SOURCE" "../../artifacts/${FINAL_WASM_NAME}"

    if [ -z "$SKIP_VALIDATE" ]; then
        echo -e "${CYAN}[5/5] Validating with cosmwasm-check...${NC}"
        if cosmwasm-check "../../artifacts/${FINAL_WASM_NAME}"; then
            echo -e "${GREEN}✓ Artifact validated${NC}"
        else
            echo -e "${RED}✗ Validation failed! The WASM is not compatible with standard CosmWasm VM.${NC}"
            exit 1
        fi
    else
        echo -e "${YELLOW}[5/5] Skipping validation${NC}"
    fi
    echo ""

    # Generate checksum
    cd ../../artifacts
    sha256sum "$FINAL_WASM_NAME" > "${FINAL_WASM_NAME}.sha256"
    SIZE=$(du -h "$FINAL_WASM_NAME" | cut -f1)
    CHECKSUM=$(cut -d' ' -f1 "${FINAL_WASM_NAME}.sha256")
    
    echo -e "${GREEN}✅ Artifact created: ${CYAN}$FINAL_WASM_NAME${GREEN} (${SIZE})${NC}"
    echo -e "  SHA256: ${CYAN}${CHECKSUM:0:16}...${NC}"
    echo ""
    
    cd ..
done

# Summary
echo -e "${GREEN}=========================================="
echo "  🏆 ALL BUILDS COMPLETE!"
echo "==========================================${NC}"
echo ""
echo -e "${BLUE}Deployment Commands:${NC}"
echo "  paxid tx wasm store artifacts/lp_locker.wasm --from admin --gas auto --gas-adjustment 1.3"
echo ""
echo -e "${GREEN}Happy deploying! 🚀${NC}"
echo ""

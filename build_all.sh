#!/usr/bin/env bash
# build_all.sh - Build both LP Platform contracts

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Clear screen only if in TTY
if [ -t 1 ]; then
    clear
fi

echo -e "${GREEN}=========================================="
echo "  PAXI NETWORK DEFI - BUILD SUITE"
echo "  Locker + Rewards + Staking"
echo "==========================================${NC}"
echo ""

# Contracts to build
CONTRACTS=(
    "lp-locker"
    "reward-controller"
    "prc20-staking"
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
    echo -e "${RED}✗ Rust not installed!${NC}"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

if ! command -v wasm-opt &> /dev/null; then
    echo -e "${YELLOW}⚠ wasm-opt not found - will skip optimization${NC}"
    echo "Install with: sudo apt-get install binaryen"
    SKIP_OPT=1
else
    echo -e "${GREEN}✓ wasm-opt found${NC}"
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
    
    # Step 1: Run tests
    echo -e "${CYAN}[1/4] Running tests...${NC}"
    if cargo test --quiet; then
        echo -e "${GREEN}✓ Tests passed${NC}"
    else
        if [ -t 1 ]; then
            echo -e "${YELLOW}⚠ Some tests failed - continue anyway? (y/n)${NC}"
            read -r response
            if [[ ! "$response" =~ ^[Yy]$ ]]; then
                exit 1
            fi
        else
            echo -e "${RED}✗ Tests failed!${NC}"
            exit 1
        fi
    fi
    echo ""
    
    # Step 2: Compile to WASM
    echo -e "${CYAN}[2/4] Compiling to WASM...${NC}"
    RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --quiet
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Compilation successful${NC}"
    else
        echo -e "${RED}✗ Compilation failed!${NC}"
        exit 1
    fi
    echo ""
    
    # Step 3: Optimize with wasm-opt
    if [ -z "$SKIP_OPT" ]; then
        echo -e "${CYAN}[3/4] Optimizing with wasm-opt...${NC}"
        wasm-opt -Oz --enable-sign-ext --enable-bulk-memory \
            "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm" \
            -o "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}_optimized.wasm"
        
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓ Optimization successful${NC}"
            FINAL_WASM="${CONTRACT_NAME_SNAKE}.wasm"
            cp "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}_optimized.wasm" \
               "../../artifacts/${FINAL_WASM}"
        else
            echo -e "${RED}✗ Optimization failed!${NC}"
            exit 1
        fi
    else
        echo -e "${YELLOW}[3/4] Skipping optimization${NC}"
        FINAL_WASM="${CONTRACT_NAME_SNAKE}.wasm"
        cp "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm" \
           "../../artifacts/${FINAL_WASM}"
    fi
    echo ""
    
    # Step 4: Generate checksum
    echo -e "${CYAN}[4/4] Generating checksum...${NC}"
    cd ../../artifacts
    sha256sum "$FINAL_WASM" > "${FINAL_WASM}.sha256"
    SIZE=$(du -h "$FINAL_WASM" | cut -f1)
    CHECKSUM=$(cut -d' ' -f1 "${FINAL_WASM}.sha256")
    
    echo -e "${GREEN}✓ Artifact created${NC}"
    echo -e "  File: ${CYAN}$FINAL_WASM${NC}"
    echo -e "  Size: ${CYAN}${SIZE}${NC}"
    echo -e "  SHA256: ${CYAN}${CHECKSUM:0:16}...${NC}"
    echo ""
    
    cd ..
done

# Summary
echo -e "${GREEN}=========================================="
echo "  ✅ BUILD COMPLETE!"
echo "==========================================${NC}"
echo ""
echo -e "${BLUE}Artifacts created:${NC}"
ls -lh artifacts/*.wasm | awk '{print "  " $9 " (" $5 ")"}'
echo ""
echo -e "${BLUE}Checksums:${NC}"
cat artifacts/*.sha256 | awk '{print "  " substr($1,1,16) "... - " $2}'
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "  1. Deploy to testnet for testing (minimum 2 weeks)"
echo "  2. Deploy lp-locker first, get contract address"
echo "  3. Deploy reward-controller with lp-locker address"
echo "  4. Deploy prc20-staking"
echo "  5. Configure contracts (whitelist LP, create pools, create rooms)"
echo "  6. Test complete user flow"
echo "  7. Deploy to mainnet"
echo ""
echo -e "${CYAN}Deployment Commands:${NC}"
echo "  # Store code"
echo "  paxid tx wasm store artifacts/lp_locker.wasm \\"
echo "    --from admin --gas auto --gas-adjustment 1.3"
echo ""
echo "  # Instantiate LP Locker"
echo "  paxid tx wasm instantiate <CODE_ID> \\"
echo "    '{\"admin\":\"paxi1...\",\"emergency_unlock_delay\":259200}' \\"
echo "    --from admin --label \"LP Locker v2\" --admin paxi1... --gas auto"
echo ""
echo "  # Store & instantiate Reward Controller"
echo "  paxid tx wasm store artifacts/reward_controller.wasm \\"
echo "    --from admin --gas auto --gas-adjustment 1.3"
echo ""
echo "  paxid tx wasm instantiate <CODE_ID> \\"
echo "    '{\"admin\":\"paxi1...\",\"lp_locker_contract\":\"paxi1...locker\"}' \\"
echo "    --from admin --label \"Reward Controller v2\" --admin paxi1... --gas auto"
echo ""
echo -e "${GREEN}Happy deploying! 🚀${NC}"
echo ""

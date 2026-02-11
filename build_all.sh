#!/usr/bin/env bash
# build_all.sh - Build all contracts in the workspace

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Parse arguments
SKIP_TESTS=0
for arg in "$@"; do
    if [ "$arg" == "--skip-tests" ]; then
        SKIP_TESTS=1
    fi
done

echo -e "${GREEN}=========================================="
echo "  LP PLATFORM - OPTIMIZED BUILD SUITE"
echo "==========================================${NC}"
echo ""

# Contracts to build
CONTRACTS=(
    "lp-locker"
    "reward-controller"
    "prc20-vesting"
)

# Check build tools
echo -e "${CYAN}Checking build tools...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ cargo not found!${NC}"
    exit 1
fi

if ! command -v wasm-opt &> /dev/null; then
    echo -e "${YELLOW}⚠ wasm-opt not found - will skip optimization${NC}"
    SKIP_OPT=1
else
    echo -e "${GREEN}✓ wasm-opt found${NC}"
fi
echo ""

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm artifacts/*.sha256

# Step 1: Run tests (if not skipped)
if [ "$SKIP_TESTS" -eq 0 ]; then
    echo -e "${CYAN}[1/4] Running workspace tests...${NC}"
    if cargo test --workspace --quiet; then
        echo -e "${GREEN}✓ All tests passed${NC}"
    else
        echo -e "${RED}✗ Tests failed!${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}[1/4] Skipping tests${NC}"
fi
echo ""

# Step 2: Compile all to WASM
echo -e "${CYAN}[2/4] Compiling workspace to WASM...${NC}"
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown
echo -e "${GREEN}✓ Compilation successful${NC}"
echo ""

# Step 3: Optimize with wasm-opt
if [ -z "$SKIP_OPT" ]; then
    echo -e "${CYAN}[3/4] Optimizing WASM files...${NC}"
    
    for contract in "${CONTRACTS[@]}"; do
        CONTRACT_NAME_SNAKE="${contract//-/_}"
        WASM_IN="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"
        WASM_OUT="artifacts/${CONTRACT_NAME_SNAKE}.wasm"
        
        if [ ! -f "$WASM_IN" ]; then
            echo -e "${RED}✗ Artifact not found: $WASM_IN${NC}"
            exit 1
        fi

        echo -e "  Optimizing ${BLUE}${contract}${NC}..."
        wasm-opt -Oz --enable-sign-ext --enable-bulk-memory "$WASM_IN" -o "$WASM_OUT"
    done
    echo -e "${GREEN}✓ Optimization complete${NC}"
else
    echo -e "${YELLOW}[3/4] Skipping optimization, copying files directly${NC}"
    for contract in "${CONTRACTS[@]}"; do
        CONTRACT_NAME_SNAKE="${contract//-/_}"
        WASM_IN="target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm"

        if [ ! -f "$WASM_IN" ]; then
            echo -e "${RED}✗ Artifact not found: $WASM_IN${NC}"
            exit 1
        fi

        cp "$WASM_IN" "artifacts/${CONTRACT_NAME_SNAKE}.wasm"
    done
fi
echo ""

# Step 4: Generate checksums
echo -e "${CYAN}[4/4] Generating checksums...${NC}"
cd artifacts
# Use a more robust loop that doesn't fail if no matches
shopt -s nullglob
WASM_FILES=(*.wasm)
if [ ${#WASM_FILES[@]} -eq 0 ]; then
    echo -e "${RED}✗ No WASM files found in artifacts/ directory!${NC}"
    exit 1
fi

for wasm in "${WASM_FILES[@]}"; do
    sha256sum "$wasm" > "${wasm}.sha256"
    CHECKSUM=$(cut -d' ' -f1 "${wasm}.sha256")
    echo -e "  ${GREEN}✓${NC} $wasm (${CYAN}${CHECKSUM:0:16}...${NC})"
done
cd ..

# Summary
echo ""
echo -e "${GREEN}=========================================="
echo "  ✅ ALL CONTRACTS BUILT!"
echo "==========================================${NC}"
echo ""
ls -lh artifacts/*.wasm | awk '{print "  " $9 " (" $5 ")"}'
echo ""

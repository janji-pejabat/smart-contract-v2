#!/usr/bin/env bash
# build_all.sh - Build Paxi Network DeFi contracts using official optimizer

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Clear screen only if in TTY
if [ -t 1 ]; then
    clear
fi

echo -e "${GREEN}=========================================="
echo "  PAXI NETWORK DEFI - OPTIMIZED BUILD"
echo "  Using cosmwasm/workspace-optimizer:0.16.1"
echo "==========================================${NC}"
echo ""

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm artifacts/*.sha256

# Check for Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}✗ Docker not found!${NC}"
    echo "Workspace optimizer requires Docker."
    exit 1
fi

echo -e "${CYAN}Running workspace optimizer...${NC}"
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.16.1

echo ""
echo -e "${GREEN}=========================================="
echo "  ✅ BUILD COMPLETE!"
echo "==========================================${NC}"
echo ""
echo -e "${CYAN}Artifacts are located in the artifacts/ directory.${NC}"
ls -lh artifacts/

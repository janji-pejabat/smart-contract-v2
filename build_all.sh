#!/usr/bin/env bash
# build_all.sh - Using official CosmWasm Optimizer for guaranteed compatibility

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${GREEN}Building LP Platform Contracts using CosmWasm Optimizer...${NC}"

# Check for Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}✗ Docker not found! Docker is required for the official CosmWasm Optimizer.${NC}"
    echo -e "${YELLOW}Please install Docker or run in an environment with Docker available.${NC}"
    exit 1
fi

# Create artifacts directory
mkdir -p artifacts
rm -f artifacts/*.wasm artifacts/*.sha256

# Run the workspace optimizer
# We use version 0.16.1 which is the current stable standard
echo -e "${CYAN}Running cosmwasm/workspace-optimizer:0.16.1...${NC}"
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.16.1

# The optimizer puts results in 'artifacts/' by default, which is perfect.
# But sometimes it uses 'artifacts/' and we want to make sure they match our expected names.
# In a workspace, it usually produces <crate_name>.wasm

echo -e "${GREEN}✅ SUCCESS! Artifacts were generated using the official optimizer.${NC}"
ls -lh artifacts/

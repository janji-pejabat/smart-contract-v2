#!/usr/bin/env bash
# build_all.sh - Production build script using official CosmWasm Optimizer
# This is the ONLY guaranteed way to produce strictly MVP-compatible WASM for Paxi Network.

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${GREEN}Building LP Platform Contracts (OFFICIAL OPTIMIZER)...${NC}"

# Check if docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}✗ Docker not installed! Official optimizer requires Docker.${NC}"
    echo -e "${YELLOW}Attempting local build fallback (NOT GUARANTEED FOR PAXI)...${NC}"

    # Fallback logic (similar to previous version but with warnings)
    mkdir -p artifacts
    CONTRACTS=("lp-locker" "reward-controller")
    for contract in "${CONTRACTS[@]}"; do
        CONTRACT_NAME_SNAKE="${contract//-/_}"
        echo -e "${CYAN}Building $contract (Local Fallback)...${NC}"
        cd "contracts/$contract"
        cargo build --release --target wasm32-unknown-unknown --quiet
        cp "target/wasm32-unknown-unknown/release/${CONTRACT_NAME_SNAKE}.wasm" "../../artifacts/"
        cd ../..
    done
    echo -e "${RED}⚠ WARNING: Local build may contain non-MVP opcodes. Use Docker for production releases.${NC}"
    exit 0
fi

# Use official Workspace Optimizer
# Using version 0.16.1 for maximum compatibility with Paxi mainnet
echo -e "${CYAN}Running cosmwasm/workspace-optimizer:0.16.1...${NC}"

docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.16.1

echo -e "${GREEN}✅ SUCCESS! Optimized artifacts are in the artifacts/ directory.${NC}"

#!/bin/bash
set -e

# Build script for Paxi Network Smart Contracts using Official Workspace Optimizer
# This ensures 100% valid WASM MVP for the target network.

echo "Building and optimizing all contracts using cosmwasm/workspace-optimizer:0.16.1..."

# Check if docker is available
if command -v docker > /dev/null; then
    docker run --rm -v "$(pwd)":/code \
      --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
      --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
      cosmwasm/workspace-optimizer:0.16.1
else
    echo "Error: Docker not found. This project requires Docker to build production-grade WASM."
    exit 1
fi

echo "Build complete. Optimized artifacts are in the artifacts/ directory."
ls -lh artifacts/

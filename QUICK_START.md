# 🚀 Quick Start Guide

## What This Is

**LP Platform v2** = Professional DeFi platform for locking LP tokens and earning rewards

- **LP Locker**: Locks LP tokens (NOT regular PRC20 tokens!) with time-lock
- **Reward Controller**: Distributes rewards to LP lockers

## One-Command Setup

```bash
# Clone repo
git clone <your-repo-url>
cd lp-platform-v2

# Build everything
chmod +x build_all.sh
./build_all.sh

# Deploy to testnet (see DEPLOYMENT.md for details)
```

## File Structure

```
lp-platform-v2/
├── contracts/
│   ├── lp-locker/           # LP token locker contract
│   └── reward-controller/   # Reward distribution contract
├── build_all.sh             # Build both contracts
├── README.md                # Full documentation
└── docs/
    ├── DEPLOYMENT.md        # Deployment guide
    └── API.md               # API reference
```

## Important Notes

⚠️ **CRITICAL**: This locks **LP TOKENS** (liquidity pool tokens), NOT regular PRC20 tokens!

- LP tokens must be whitelisted before use
- LP tokens are CW20 tokens representing pool shares
- Example: PAXI-USDT LP token from DEX

## GitHub Actions

Push to `main` branch to trigger automatic build:

1. Tests run automatically
2. Contracts build to WASM
3. Artifacts uploaded
4. On git tag `v*`, creates release

## Next Steps

1. ✅ Build contracts: `./build_all.sh`
2. ✅ Deploy to testnet (see docs/DEPLOYMENT.md)
3. ✅ Whitelist LP tokens
4. ✅ Create reward pools
5. ✅ Test for 2+ weeks
6. ✅ Deploy to mainnet

## Support

- 📖 Full docs: README.md
- 🚀 Deployment: docs/DEPLOYMENT.md
- 📡 API: docs/API.md
- 🐛 Issues: GitHub Issues

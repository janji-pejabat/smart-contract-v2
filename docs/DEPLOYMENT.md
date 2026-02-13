# Deployment Guide - Paxi Network DeFi

## Quick Deployment Steps

1. **Build all contracts**: `./build_all.sh`
2. **Deploy LP Locker**: Initialize with admin address and emergency delay.
3. **Deploy Reward Controller**: Initialize with admin and the LP Locker contract address.
4. **Deploy PRC20 Staking**: Initialize with admin.
5. **Configure LP Locker**:
   - Update config to set the Reward Controller address.
   - Whitelist desired LP tokens.
6. **Configure Reward Controller**:
   - Create reward pools.
   - Deposit rewards into pools.
7. **Configure PRC20 Staking**:
   - Create rooms with specific stake/NFT configs.
   - Add reward pools to rooms.
   - Fund the reward pools.

## Verification
Ensure cross-contract queries (Reward Controller -> LP Locker) are working by registering a stake.
Verify NFT boosters by staking in a room with an NFT requirement.

See `docs/API.md` for message examples.

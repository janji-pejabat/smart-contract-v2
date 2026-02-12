# 🏦 LP Platform v2.0 - Professional LP Locker + Reward Engine

Production-ready DeFi platform for Paxi Network with dual-contract architecture.

## 🎯 Overview

Platform ini terdiri dari **2 smart contracts**:

1. **LP Locker** - Lock LP tokens dengan time-lock mechanism
2. **Reward Controller** - Distribute rewards ke LP lockers

### Key Features

- ✅ **Secure LP Locking** - LP tokens (CW20) di-lock on-chain
- ✅ **Multi-Token Rewards** - Support CW20 + native PAXI rewards
- ✅ **Configurable APR** - Emission per second + bonus multiplier
- ✅ **Lock & Earn** - User dapat claim rewards tanpa unlock LP
- ✅ **Emergency Safety** - 3-day delay emergency unlock
- ✅ **Migration Support** - Upgradeable contracts
- ✅ **Audit-Ready** - Reentrancy protection, overflow checks

## 🚀 Quick Start

### 1. Clone & Build

```bash
git clone <your-repo>
cd lp-platform-v2
chmod +x build_all.sh
./build_all.sh
```

### 2. Deploy to Testnet

```bash
# Deploy LP Locker
paxid tx wasm store artifacts/lp_locker.wasm \
  --from admin --gas auto --gas-adjustment 1.3 \
  --chain-id paxitest-1 --node https://rpc-testnet.paxi.network:443

# Get code_id from output, then instantiate
paxid tx wasm instantiate <CODE_ID> \
  '{"admin":"paxi1...","emergency_unlock_delay":259200}' \
  --from admin --label "LP Locker v2" \
  --admin paxi1... --gas auto

# Deploy Reward Controller
paxid tx wasm store artifacts/reward_controller.wasm \
  --from admin --gas auto --gas-adjustment 1.3

paxid tx wasm instantiate <CODE_ID> \
  '{"admin":"paxi1...","lp_locker_contract":"paxi1...locker-addr"}' \
  --from admin --label "Reward Controller v2" \
  --admin paxi1... --gas auto
```

### 3. Configure Platform

```bash
# Whitelist LP token
paxid tx wasm execute <LOCKER_ADDR> \
  '{"whitelist_lp":{"lp_token":"paxi1...lp-token","min_lock_duration":604800,"max_lock_duration":31536000,"bonus_multiplier":"1.0"}}' \
  --from admin --gas auto

# Create reward pool
paxid tx wasm execute <REWARD_ADDR> \
  '{"create_reward_pool":{"reward_token":{"cw20":"paxi1...reward-token"},"emission_per_second":"100000000","start_time":1234567890}}' \
  --from admin --gas auto

# Deposit rewards
paxid tx wasm execute <REWARD_TOKEN> \
  '{"send":{"contract":"<REWARD_ADDR>","amount":"1000000000000","msg":"eyJkZXBvc2l0X3Jld2FyZHMiOnsicG9vbF9pZCI6MH19"}}' \
  --from admin --gas auto
```

## 📖 User Flow

### Lock LP Tokens

```bash
# User sends LP tokens to locker contract
paxid tx wasm execute <LP_TOKEN_ADDR> \
  '{"send":{"contract":"<LOCKER_ADDR>","amount":"1000000","msg":"<base64_encoded_hook>"}}' \
  --from user --gas auto

# Hook message (base64 encoded):
# {"lock_lp":{"unlock_time":1735689600}}
```

### Register for Rewards

> **Note**: Registration is automatic for new lockers. Use this only for lockers created before the reward controller was active.

```bash
paxid tx wasm execute <REWARD_ADDR> \
  '{"register_stake":{"locker_id":1}}' \
  --from user --gas auto
```

### Claim Rewards

```bash
paxid tx wasm execute <REWARD_ADDR> \
  '{"claim_rewards":{"pool_ids":[0,1]}}' \
  --from user --gas auto
```

### Unlock LP

```bash
# After unlock_time has passed
paxid tx wasm execute <LOCKER_ADDR> \
  '{"unlock_lp":{"locker_id":1}}' \
  --from user --gas auto
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    USER / DEV TOKEN                      │
└────────────┬────────────────────────────┬────────────────┘
             │                            │
             │ Lock LP                    │ Claim Reward
             │                            │
     ┌───────▼──────────┐        ┌────────▼─────────────┐
     │                  │◄───────┤                      │
     │  LP LOCKER       │  Query │  REWARD CONTROLLER   │
     │  CONTRACT        │  Hook  │  CONTRACT            │
     │                  ├───────►│                      │
     └──────┬───────────┘        └──────┬───────────────┘
            │                           │
            │ Transfer LP               │ Transfer Reward
            │                           │
     ┌──────▼───────────┐        ┌──────▼───────────────┐
     │  CW20 LP TOKEN   │        │  CW20 REWARD TOKEN   │
     └──────────────────┘        └──────────────────────┘
```

## 🔐 Security Features

- **Reentrancy Protection** - CEI pattern, no callback vulnerabilities
- **Whitelist Enforcement** - Only approved LP tokens can be locked
- **Time-lock Safety** - Cannot unlock before `unlock_time`
- **Emergency Delay** - 3-day mandatory delay for emergency unlock
- **Overflow Protection** - All math uses `checked_*` operations
- **Flash Lock Protection** - Cannot claim rewards from unlocked LP
- **Double Claim Prevention** - Per-user reward tracking with cooldown
- **Admin Safeguards** - Cannot pause unlock operations

## 📊 Bonus Multiplier System

Lock duration determines reward multiplier:

| Lock Duration | Multiplier | APR Boost |
|--------------|------------|-----------|
| 0-30 days    | 1.0x       | Base APR  |
| 31-90 days   | 1.2x       | +20%      |
| 91-180 days  | 1.5x       | +50%      |
| 181-365 days | 2.0x       | +100%     |
| 365+ days    | 2.5x       | +150%     |

## 🧪 Testing

```bash
# Run all tests
cd contracts/lp-locker && cargo test
cd ../reward-controller && cargo test

# Integration tests
cargo test --test integration
```

## 📦 Deployment Checklist

- [ ] Build contracts (`./build_all.sh`)
- [ ] Deploy to testnet
- [ ] Test lock/unlock flow (minimum 2 weeks)
- [ ] Test reward distribution
- [ ] Whitelist production LP tokens
- [ ] Configure reward pools
- [ ] Deposit initial rewards
- [ ] Deploy to mainnet
- [ ] Update frontend config
- [ ] Announce launch

## 🔄 Migration

Contracts support migration via `MigrateMsg`:

```bash
paxid tx wasm migrate <CONTRACT_ADDR> <NEW_CODE_ID> \
  '{"v1_to_v2":{"reward_controller":"paxi1..."}}' \
  --from admin --gas auto
```

## 📄 License

MIT License - see LICENSE file

## 🤝 Contributing

1. Fork repository
2. Create feature branch
3. Run tests
4. Submit PR

## 📞 Support

- Documentation: [docs/](docs/)
- Issues: GitHub Issues
- Discord: [Paxi Network](https://discord.gg/paxi)

---

**Built with ❤️ for Paxi Network DeFi ecosystem**

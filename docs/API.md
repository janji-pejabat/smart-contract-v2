# Paxi Network DeFi API Reference

This documentation provides a comprehensive guide to interacting with the Paxi Network DeFi smart contracts: **LP Locker**, **Reward Controller**, and **PRC20 Staking**.

---

## 1. LP Locker Contract

The LP Locker contract allows users to lock their LP (CW20) tokens for a specified duration. Admin can whitelist specific tokens and set bonus multipliers.

### 1.1 Execute Messages

#### LockLP (via CW20 Send)
To lock tokens, users must call `Send` on the CW20 contract, pointing to the locker contract.
```json
{
  "send": {
    "contract": "locker_contract_addr",
    "amount": "1000000",
    "msg": "eyJsb2NrX2xwIjp7InVubG9ja190aW1lIjoxNzM1Njg5NjAwLCJtZXRhZGF0YSI6IlByb2plY3QgWmxvY2sifX0="
  }
}
```
**Base64 Decoded Hook:**
```json
{
  "lock_lp": {
    "unlock_time": 1735689600,
    "metadata": "Optional project info"
  }
}
```
- **unlock_time**: Unix timestamp (seconds). Must be > current time and within whitelisted duration range.
- **metadata**: (Optional) String for project identification.

#### UnlockLP
Withdraws LP tokens after the lock has expired.
```json
{
  "unlock_lp": {
    "locker_id": 1
  }
}
```

#### ExtendLock
Increases the unlock time of an existing locker.
```json
{
  "extend_lock": {
    "locker_id": 1,
    "new_unlock_time": 1767225600
  }
}
```

#### RequestEmergencyUnlock
Starts a cooldown timer for an emergency withdrawal.
```json
{
  "request_emergency_unlock": {
    "locker_id": 1
  }
}
```

#### ExecuteEmergencyUnlock
Withdraws tokens after the emergency delay has passed.
```json
{
  "execute_emergency_unlock": {
    "locker_id": 1
  }
}
```

#### Admin: WhitelistLP
Whitelists a CW20 token for locking.
```json
{
  "whitelist_lp": {
    "lp_token": "paxi1...",
    "min_lock_duration": 86400,
    "max_lock_duration": 31536000,
    "bonus_multiplier": "1.0"
  }
}
```

#### Admin: UpdateConfig
```json
{
  "update_config": {
    "admin": "paxi1...new_admin",
    "reward_controller": "paxi1...controller",
    "emergency_unlock_delay": 432000,
    "platform_fee_bps": 100
  }
}
```

### 1.2 Query Messages

#### Config
```json
{"config":{}}
```

#### Locker
```json
{
  "locker": {
    "locker_id": 1
  }
}
```

#### LockersByOwner
```json
{
  "lockers_by_owner": {
    "owner": "paxi1...",
    "start_after": 0,
    "limit": 10
  }
}
```

#### WhitelistedLP
```json
{
  "whitelisted_lp": {
    "lp_token": "paxi1..."
  }
}
```

#### AllWhitelistedLPs
```json
{
  "all_whitelisted_lps": {
    "start_after": null,
    "limit": 10
  }
}
```

#### TotalLockedByLP
```json
{
  "total_locked_by_lp": {
    "lp_token": "paxi1..."
  }
}
```

---

## 2. Reward Controller Contract

The Reward Controller manages rewards for LP lockers. It supports multiple reward pools per LP token and features dynamic APR scaling.

### 2.1 Execute Messages

#### RegisterStake
Notifies the reward controller that a new LP lock has been created.
```json
{
  "register_stake": {
    "locker_id": 1
  }
}
```

#### UnregisterStake
Claims final rewards and removes the stake record.
```json
{
  "unregister_stake": {
    "locker_id": 1
  }
}
```

#### ClaimRewards
Claims accrued rewards from specified pools.
```json
{
  "claim_rewards": {
    "pool_ids": [0, 1]
  }
}
```

#### Admin: CreateRewardPool
```json
{
  "create_reward_pool": {
    "reward_token": { "cw20": "paxi1...token" },
    "emission_per_second": "100",
    "start_time": 1735689600,
    "end_time": 1767225600
  }
}
```

#### DepositRewards
Funds a reward pool. Can be called with funds for native tokens or via a CW20 hook.
```json
{
  "deposit_rewards": {
    "pool_id": 0
  }
}
```

### 2.2 Query Messages

#### Config
```json
{"config":{}}
```

#### RewardPool
```json
{
  "reward_pool": { "pool_id": 0 }
}
```

#### AllRewardPools
```json
{
  "all_reward_pools": {
    "start_after": null,
    "limit": 10
  }
}
```

#### UserStake
```json
{
  "user_stake": {
    "user": "paxi1...",
    "locker_id": 1
  }
}
```

#### PendingRewards
Calculates current claimable rewards.
```json
{
  "pending_rewards": {
    "user": "paxi1...",
    "pool_id": 0
  }
}
```

---

## 3. PRC20 Staking Contract

The PRC20 Staking contract provides isolated "Rooms" for staking PRC20 tokens. It supports multi-token rewards, AND/OR staking rules, and NFT-gated boosters.

### 3.1 Core Features
- **Room-based Isolation**: Each room has its own staking rules and reward pools.
- **Multi-Reward**: Earn multiple different tokens simultaneously in one room.
- **AND/OR Rules**: Partner rooms can require multiple specific tokens to be staked (AND rule) or any one from a list (OR rule).
- **NFT Integration**: Boost your APR by holding Paxi Network NFTs.
- **Auto-Compound**: Premium feature gated by NFT ownership or minimum stake thresholds.
- **Reward Solvency**: Emissions automatically stop if the reward pool is exhausted.

### 3.2 Execute Messages

#### Stake (via CW20 Send)
Stake tokens into a room.
```json
{
  "send": {
    "contract": "staking_contract_addr",
    "amount": "5000",
    "msg": "eyJzdGFrZSI6eyJyb29tX2lkIjoxfX0="
  }
}
```
**Base64 Decoded Hook:**
```json
{
  "stake": { "room_id": 1 }
}
```

#### Unstake
Withdraw staked tokens. Subject to cooldown and early withdrawal penalties.
```json
{
  "unstake": {
    "room_id": 1,
    "amount": "1000",
    "token_address": "paxi1...token"
  }
}
```

#### ClaimRewards
Claim all accumulated rewards in a room.
```json
{
  "claim_rewards": { "room_id": 1 }
}
```

#### ToggleAutoCompound
Opt-in/out of reward auto-compounding.
```json
{
  "toggle_auto_compound": {
    "room_id": 1,
    "enabled": true
  }
}
```

#### Compound (Manual)
Trigger compounding of rewards into the principal stake.
```json
{
  "compound": { "room_id": 1 }
}
```

### 3.3 Admin Messages

#### CreateRoom
```json
{
  "create_room": {
    "name": "Super Room",
    "stake_config": {
      "stake_tokens": ["paxi1...tokenA"],
      "is_and_rule": false,
      "min_stake_amount": "100"
    },
    "nft_config": {
      "nft_address": "paxi1...nft",
      "required_for_staking": false,
      "tier_multipliers": [
        { "tier_name": "Gold", "multiplier": "1.5", "auto_compound_unlocked": true }
      ]
    },
    "auto_compound_config": {
      "enabled": true,
      "min_stake_threshold": "1000",
      "nft_required": true
    },
    "early_withdraw_penalty": "0.1",
    "cooldown_period": 86400
  }
}
```

#### AddRewardPool
```json
{
  "add_reward_pool": {
    "room_id": 1,
    "reward_token": { "cw20": "paxi1...reward" },
    "emission_per_second": "100"
  }
}
```

### 3.4 Query Messages

#### Config
```json
{"config":{}}
```

#### Room
```json
{
  "room": { "room_id": 1 }
}
```

#### Rooms
```json
{
  "rooms": {
    "start_after": null,
    "limit": 10
  }
}
```

#### UserPosition
```json
{
  "user_position": {
    "room_id": 1,
    "user": "paxi1..."
  }
}
```

#### PendingRewards
```json
{
  "pending_rewards": {
    "room_id": 1,
    "user": "paxi1..."
  }
}
```

#### APREstimate
```json
{
  "apr_estimate": { "room_id": 1 }
}
```

#### Eligibility
Check if a user can stake or auto-compound.
```json
{
  "eligibility": {
    "room_id": 1,
    "user": "paxi1..."
  }
}
```

---

## 4. Technical Specifications

### 4.1 Reward Mathematics
The protocol uses a **Synthetix-style MasterChef algorithm** for reward distribution.
- Rewards are updated whenever a room state changes (stake, claim, compound).
- `acc_reward_per_share` is calculated as: `previous_value + (emission * elapsed_time / total_staked_weight)`.
- **Solvency Check**: Emissions are capped by the remaining balance in the reward pool. If `emission * elapsed_time > pool_balance`, only the remaining balance is distributed, and the APR effectively drops to zero.

### 4.2 Feature Gating
Advanced features like **Auto-Compound** are restricted based on:
1. **NFT Ownership**: Verified via read-only queries to Paxi Network NFT contracts.
2. **Stake Thresholds**: Total staked amount across all room tokens must meet the admin-defined minimum.
3. **AND Rule**: In partner rooms, `total_staked_weight` remains zero until ALL required tokens meet the minimum stake threshold.

### 4.3 State Behavior
- **Paused Contract**: Stake and Compound operations are blocked. Unstake and Claim remain available to ensure user safety.
- **Snapshot Consistency**: User reward state is always "caught up" to the current room state before any modification to the user's principal stake.

# PRC20 Staking Contract: Comprehensive Implementation Guide

This document provides a standalone, comprehensive reference for the **PRC20 Staking** smart contract. It details all supported features, execution messages, query operations, and the technical logic governing the protocol.

---

## 1. Overview & Core Features

The PRC20 Staking contract is a modular, enterprise-grade staking protocol designed for the Paxi Network. It allows users to stake PRC20 (CW20) tokens in isolated "Rooms" to earn multiple reward tokens simultaneously.

### Key Features:
- **Room-Based Architecture**: Each room is a sovereign staking environment with its own configuration, reward pools, and access rules.
- **Multi-Reward Distribution**: A single room can distribute multiple different reward tokens (CW20 or Native) at independent emission rates.
- **AND/OR Staking Rules**: Supports complex staking requirements. A room can require a user to stake all listed tokens (AND) or any single token (OR) to begin earning rewards.
- **Paxi Network NFT Integration**: Read-only integration with Paxi NFTs provides tiered APR boosters and unlocks premium features.
- **Auto-Compound (Premium)**: eligible users can opt-in to have their rewards automatically reinvested into their principal stake.
- **Reward Solvency & Inflation Protection**: Emissions are mathematically capped by the actual balance of the reward pools, preventing protocol insolvency.
- **Withdrawal Management**: Configurable cooldown periods and early withdrawal penalties protect the protocol from short-term churn.

---

## 2. Technical Architecture & Logic

### 2.1 Reward Accrual Engine
The contract implements a **Synthetix-style MasterChef algorithm** for multi-token reward distribution.

- **Global State**: For each reward token in a room, the contract tracks `acc_reward_per_share`.
- **Accrual Logic**: Whenever any user interacts with a room, the `acc_reward_per_share` is updated using the elapsed time since the last interaction and the current total staked weight.
- **Solvency Capping**: The emission is capped by the remaining pool balance. If the calculated emission exceeds the available funds, only the remaining balance is accrued, and the emission effective rate drops to zero until the pool is replenished.

### 2.2 User Weight & NFT Boosters
A user's reward share is determined by their `effective_weight`:
`effective_weight = (Sum of staked tokens) * nft_multiplier`

- **AND Rule**: If a room is configured with an "AND" rule, the `effective_weight` remains **zero** until the user has staked the `min_stake_amount` for **every** token required by the room.
- **Multiplier Caching**: The `nft_multiplier` is queried from the NFT contract and cached in the user's position during every interaction (stake, claim, compound) to optimize gas costs.

### 2.3 State Transitions
1. **Interaction Start**: Update room's global `acc_reward_per_share`.
2. **Reward Sync**: Calculate user's pending rewards based on their *old* weight and add them to `pending_rewards`.
3. **Multiplier Refresh**: Re-query NFT ownership to update the user's `nft_multiplier`.
4. **Principal Update**: Adjust staked amounts (add or subtract tokens).
5. **Weight Sync**: Recalculate and save the user's *new* `effective_weight` and update the room's `total_staked_weight`.

---

## 3. Execution Messages

### 3.1 User Operations

#### Stake (via CW20 Send)
To stake, use the `Send` message on the PRC20/CW20 token contract.
```json
{
  "send": {
    "contract": "staking_contract_addr",
    "amount": "1000000",
    "msg": "eyJzdGFrZSI6eyJyb29tX2lkIjoxfX0="
  }
}
```
**Hook Message (Decoded):**
```json
{ "stake": { "room_id": 1 } }
```

#### Unstake
Withdraw staked tokens.
```json
{
  "unstake": {
    "room_id": 1,
    "amount": "500000",
    "token_address": "paxi1stake_token_addr..."
  }
}
```
- **Validations**: Fails if `cooldown_period` has not elapsed since the last stake/interaction.
- **Penalty**: If `early_withdraw_penalty` is set, the penalty amount is deducted and remains in the contract; the user receives the remainder.

#### ClaimRewards
Collect all accumulated rewards in a room.
```json
{ "claim_rewards": { "room_id": 1 } }
```

#### ToggleAutoCompound
Opt-in or out of automatic reward reinvestment.
```json
{
  "toggle_auto_compound": {
    "room_id": 1,
    "enabled": true
  }
}
```
- **Requirements**: Fails if the user does not meet the room's `auto_compound_config` (min stake or NFT requirement).

#### Compound (Manual)
Manually trigger the compounding of current rewards into the stake.
```json
{ "compound": { "room_id": 1 } }
```

### 3.2 Administrative Operations

#### CreateRoom
Initializes a new staking isolated environment.
```json
{
  "create_room": {
    "name": "Super Partner Room",
    "stake_config": {
      "stake_tokens": ["paxi1tokenA...", "paxi1tokenB..."],
      "is_and_rule": true,
      "min_stake_amount": "1000000"
    },
    "nft_config": {
      "nft_address": "paxi1nft_addr...",
      "required_for_staking": false,
      "tier_multipliers": [
        { "tier_name": "Diamond", "multiplier": "2.0", "auto_compound_unlocked": true },
        { "tier_name": "Gold", "multiplier": "1.5", "auto_compound_unlocked": true }
      ]
    },
    "auto_compound_config": {
      "enabled": true,
      "min_stake_threshold": "5000000",
      "nft_required": true
    },
    "early_withdraw_penalty": "0.05",
    "cooldown_period": 604800
  }
}
```

#### AddRewardPool
Adds a new token to be earned in the room.
```json
{
  "add_reward_pool": {
    "room_id": 1,
    "reward_token": { "cw20": "paxi1reward_token..." },
    "emission_per_second": "500"
  }
}
```

#### UpdateRewardPool
Adjusts the emission rate of an existing reward pool.
```json
{
  "update_reward_pool": {
    "room_id": 1,
    "reward_token": { "cw20": "paxi1reward_token..." },
    "emission_per_second": "250"
  }
}
```

#### FundRewardPool
Deposits tokens into a reward pool. Can be called with funds for Native tokens.
```json
{ "fund_reward_pool": { "room_id": 1 } }
```
*Note: For CW20 reward tokens, use `Send` with the `FundPool` hook.*

---

## 4. Query Operations

### 4.1 Room Information
**Request:**
```json
{ "room": { "room_id": 1 } }
```
**Response:**
```json
{
  "room": {
    "id": 1,
    "name": "Super Room",
    "total_staked_weight": "15000000",
    "paused": false,
    "reward_configs": [
      { "reward_token": {"cw20": "paxi1..."}, "emission_per_second": "100", "total_deposited": "1000000", "total_claimed": "50000" }
    ],
    ...
  }
}
```

### 4.2 User Position
**Request:**
```json
{ "user_position": { "room_id": 1, "user": "paxi1user..." } }
```
**Response:**
```json
{
  "position": {
    "staked_amounts": [["paxi1tokenA...", "5000000"]],
    "pending_rewards": [[{"cw20": "paxi1..."}, "25000"]],
    "nft_multiplier": "1.5",
    "auto_compound_enabled": true,
    "staked_at": 1700000000,
    "last_interaction": 1705000000
  }
}
```

### 4.3 Pending Rewards
Returns the real-time pending rewards, accounting for time passed since the last transaction.
**Request:**
```json
{ "pending_rewards": { "room_id": 1, "user": "paxi1user..." } }
```
**Response:**
```json
{
  "rewards": [
    [{"cw20": "paxi1rewardA..."}, "5000"],
    [{"native": "upaxi"}, "1200"]
  ]
}
```

### 4.4 APR Estimate
Calculates dynamic APR based on current pool balance and total staked weight.
**Request:**
```json
{ "apr_estimate": { "room_id": 1 } }
```
**Response:**
```json
{
  "aprs": [
    [{"cw20": "paxi1..."}, "0.245"],
    [{"native": "upaxi"}, "0.12"]
  ]
}
```

---

## 5. Protocol Constraints & Behaviors

### 5.1 Paused State Behavior
The contract supports a global pause and per-room pause.
- **When Paused**: `Stake` and `Compound` (reinvesting rewards) are disabled to prevent state changes during volatile periods or maintenance.
- **Safety**: `Unstake` and `ClaimRewards` **remain enabled** even when paused. Users can always exit their positions and withdraw their property.

### 5.2 Validation Rules
- **Cooldown**: The `cooldown_period` is enforced on every withdrawal. Any stake or reward interaction resets the `last_interaction` timer.
- **Arithmetic**: Every calculation (even reward per share) uses `checked_` variants. In the rare event of an overflow, the transaction fails rather than producing an incorrect state.
- **Asset Type**: `AssetInfo` supports both `Cw20` and `Native` tokens, ensuring compatibility with all Paxi Network assets.

### 5.3 Special Operations
- **Revocation**: The protocol does not support administrative revocation of user funds. Once staked, only the user can initiate a withdrawal.
- **Config Updates**: Admin updates to room parameters (like penalties or emission rates) only affect **future** reward accruals. Historical rewards are locked in the `pending_rewards` state at the moment of update.

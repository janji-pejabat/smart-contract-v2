# Paxi Network DeFi Protocol: Comprehensive Implementation Guide

This guide serves as the standalone technical and operational documentation for the Paxi Network DeFi protocol. It covers the core smart contracts, their mathematical foundations, and a detailed API reference for developers and operators.

---

## 1. Protocol Architecture Overview

The Paxi Network DeFi suite consists of three interconnected smart contracts designed for secure asset management and reward distribution:

1.  **LP Locker**: Custodial contract for time-locking CW20/PRC20 Liquidity Provider tokens.
2.  **Reward Controller**: An APR-based distribution engine that calculates and emits rewards for LP Locker participants.
3.  **PRC20 Staking**: A room-based, multi-reward staking protocol for isolated and partner-governed staking environments.

### 1.1 Core Philosophy
-   **Security First**: All math uses checked arithmetic. Check-Effect-Interaction (CEI) patterns are strictly enforced.
-   **Transparency**: No hidden admin functions. Reward solvency is guaranteed via on-chain balance checks.
-   **Modularity**: Isolated "Rooms" and multiple reward pools allow for flexible protocol growth without upgrading the core contract logic.

---

## 2. LP Locker Contract

The LP Locker allows users to commit their LP tokens for a fixed duration in exchange for reward eligibility.

### 2.1 Key Features
-   **Whitelisting**: Only admin-approved tokens can be locked.
-   **Emergency Delay**: A safety mechanism allowing users to withdraw early after a mandatory 3-day (default) request period.
-   **Extension**: Users can extend their locks to improve their reward multipliers.

### 2.2 Execution Messages

#### LockLP (via CW20 Send)
**Usage**: Called by sending tokens from a CW20 contract to the Locker.
```json
{
  "send": {
    "contract": "paxi1lockeraddr...",
    "amount": "100000000",
    "msg": "eyJsb2NrX2xwIjp7InVubG9ja190aW1lIjoxNzM1Njg5NjAwLCJtZXRhZGF0YSI6IlN0YWtlIGluIGxwa2ltIn19"
  }
}
```
**Hook Structure (Decoded):**
```json
{
  "lock_lp": {
    "unlock_time": 1735689600,
    "metadata": "Optional lock info"
  }
}
```
-   **Constraints**: `unlock_time` must be greater than current time + `min_lock_duration` and less than `max_lock_duration`.

#### UnlockLP
**Usage**: Withdraws tokens after `unlock_time`.
```json
{ "unlock_lp": { "locker_id": 42 } }
```

#### RequestEmergencyUnlock
**Usage**: Initiates the emergency withdrawal cooldown.
```json
{ "request_emergency_unlock": { "locker_id": 42 } }
```

#### ExecuteEmergencyUnlock
**Usage**: Finalizes withdrawal after the `emergency_unlock_delay` has passed.
```json
{ "execute_emergency_unlock": { "locker_id": 42 } }
```

### 2.3 Query Operations

#### Locker Info
**Request**:
```json
{ "locker": { "locker_id": 42 } }
```
**Response**:
```json
{
  "id": 42,
  "owner": "paxi1...",
  "lp_token": "paxi1...",
  "amount": "1000000",
  "locked_at": 1700000000,
  "unlock_time": 1735689600,
  "extended_count": 0,
  "emergency_unlock_requested": null,
  "metadata": "My Lock"
}
```

---

## 3. Reward Controller Contract

The Reward Controller manages emissions for users holding active locks in the LP Locker.

### 3.1 Reward Mathematics (MasterChef)
The contract uses a time-weighted distribution algorithm:
1.  **Global Index Update**: `acc_reward_per_share += (elapsed_seconds * emission_per_second) / total_staked_amount`.
2.  **User Reward Accrual**: `user_pending += user_staked * (global_index - user_last_index)`.
3.  **Solvency Check**: Emissions are capped by the actual balance of the contract. The protocol will never promise rewards it cannot pay.

### 3.2 Execution Messages

#### RegisterStake
**Usage**: Syncs a new LP lock to the reward system.
```json
{ "register_stake": { "locker_id": 42 } }
```

#### ClaimRewards
**Usage**: Distributes pending rewards to the user.
```json
{ "claim_rewards": { "pool_ids": [0, 1] } }
```

### 3.3 Query Operations

#### PendingRewards
**Request**:
```json
{ "pending_rewards": { "user": "paxi1...", "pool_id": 0 } }
```
**Response**:
```json
{
  "pool_id": 0,
  "pending_amount": "5000000"
}
```

---

## 4. PRC20 Staking Contract

The PRC20 Staking contract provides a "Room" based architecture for flexible, isolated staking environments.

### 4.1 Key Features
-   **Multi-Token Rewards**: Each room can emit multiple reward tokens simultaneously.
-   **AND/OR Rule**: Partner rooms can require users to stake ALL tokens in a list (AND) or any single token (OR).
-   **NFT Boosters**: HOLDing Paxi Network NFTs applies a multiplier to the user's effective stake weight.
-   **Auto-Compound**: A premium feature that automatically reinvests rewards into the principal stake.

### 4.2 Execution Messages

#### Stake (via CW20 Send)
**Usage**: Sends tokens to a specific staking room.
```json
{
  "send": {
    "contract": "paxi1staking...",
    "amount": "5000000",
    "msg": "eyJzdGFrZSI6eyJyb29tX2lkIjoxfX0="
  }
}
```
**Hook Structure (Decoded):**
```json
{ "stake": { "room_id": 1 } }
```

#### Unstake
**Usage**: Withdraws principal. Subject to `cooldown_period` and `early_withdraw_penalty`.
```json
{
  "unstake": {
    "room_id": 1,
    "amount": "1000000",
    "token_address": "paxi1..."
  }
}
```

#### ToggleAutoCompound
**Usage**: Opt-in to premium auto-compounding.
```json
{ "toggle_auto_compound": { "room_id": 1, "enabled": true } }
```
-   **Eligibility**: Requires user to meet minimum stake OR hold a required Paxi NFT (admin defined).

### 4.3 Query Operations

#### APREstimate
**Request**:
```json
{ "apr_estimate": { "room_id": 1 } }
```
**Response**:
```json
{
  "aprs": [
    ["paxi1rewardA...", "0.15"],
    ["paxi1rewardB...", "0.08"]
  ]
}
```

#### User Position
**Request**:
```json
{ "user_position": { "room_id": 1, "user": "paxi1..." } }
```
**Response**:
```json
{
  "position": {
    "staked_amounts": [["paxi1stake...", "1000000"]],
    "pending_rewards": [["paxi1reward...", "50000"]],
    "nft_multiplier": "1.25",
    "auto_compound_enabled": true
  }
}
```

---

## 5. Security & Governance

### 5.1 Admin Privileges
Admins can:
-   Whitelist tokens and create rooms/pools.
-   Pause contracts in case of emergency.
-   Adjust emission rates (forward-looking only).

Admins **CANNOT**:
-   Withdraw user-locked/staked tokens.
-   Reduce a user's already accrued rewards.
-   Bypass the emergency unlock delay.

### 5.2 Contract Pausing
When a contract is **PAUSED**:
-   `Stake`, `Deposit`, and `Register` operations are blocked.
-   `Unstake`, `Withdraw`, and `Claim` operations remain **ENABLED** to ensure users always have access to their funds.

### 5.3 Slashing and Penalties
-   **Early Withdraw Penalty**: Configurable per room. Deducts a percentage of the withdrawn amount if performed before the room-specific cooldown.
-   **Fees**: The LP Locker can charge a platform fee (in BPS) on lock/unlock operations, directed to the treasury.

---

## 6. Implementation Guide for Developers

### 6.1 Integration Flow
1.  **Frontend**: Always query `Eligibility` before allowing a user to stake or enable auto-compound.
2.  **Indexing**: Track `locker_id` and `room_id` events to maintain a history of user interactions.
3.  **Refreshes**: The `nft_multiplier` is cached for gas efficiency. Encourage users to perform any small action (like a claim) if they recently acquired a new NFT booster to refresh their multiplier.

### 6.2 Technical State Transitions
When a user stakes:
1.  `update_room_rewards()` is called to catch up global state.
2.  User rewards are calculated based on their *old* weight and added to `pending_rewards`.
3.  `nft_multiplier` is re-queried from the NFT contract.
4.  Principal stake is updated.
5.  User's `effective_weight` is recalculated for future rewards.

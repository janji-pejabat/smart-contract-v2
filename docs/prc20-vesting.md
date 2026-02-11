# PRC20 Token Vesting - Technical Documentation & User Guide

The **PRC20 Token Vesting** contract is a secure, production-grade system for managing the phased release of PRC20 (CW20-compatible) tokens. This guide provides a comprehensive overview of features, implementation examples, and technical logic.

## Table of Contents
1. [Core Features](#core-features)
2. [Vesting Models & JSON Examples](#vesting-models--json-examples)
3. [Contract Lifecycle & Execution](#contract-lifecycle--execution)
4. [Querying State](#querying-state)
5. [Important Validations & Behaviors](#important-validations--behaviors)
6. [Technical Math Definitions](#technical-math-definitions)

---

## Core Features

- **Standardized PRC20 Support**: Works with any CW20-compliant token.
- **Time-Based Logic**: Uses `env.block.time` for deterministic unlocks (resistant to block height variations).
- **Multiple Release Strategies**: Support for continuous linear release, cliffs, and custom milestones.
- **Custodial Security**: Tokens are held in escrow by the contract until claimed by the beneficiary.
- **Batch Processing**: Gas-efficient batch creation and multi-position claiming.
- **Administrative Oversight**: Optional revocation for specific positions and emergency pause functionality.
- **Secondary Indexing**: Efficient lookups by beneficiary address or category.

---

## Vesting Models & JSON Examples

### 1. Pure Linear Vesting
Tokens unlock smoothly every second (or according to interval) from start to end.

```json
{
  "linear": {
    "start_time": 1700000000,
    "end_time": 1731536000,
    "release_interval": 1
  }
}
```

### 2. Cliff + Linear Vesting
No tokens are released until the `cliff_time`. At the cliff, the accumulated amount from `start_time` is released immediately, followed by linear vesting until `end_time`.

```json
{
  "linear": {
    "start_time": 1700000000,
    "end_time": 1731536000,
    "cliff_time": 1715768000,
    "release_interval": 3600
  }
}
```
*Note: `release_interval` of 3600 means tokens unlock in hourly chunks.*

### 3. Custom Milestones (Step Vesting)
Tokens unlock in discrete amounts at specific timestamps.

```json
{
  "custom": {
    "milestones": [
      { "timestamp": 1700000000, "amount": "200000000" },
      { "timestamp": 1710000000, "amount": "300000000" },
      { "timestamp": 1720000000, "amount": "500000000" }
    ]
  }
}
```
*Constraint: The sum of milestone amounts MUST exactly match the total tokens sent in the creation transaction.*

---

## Contract Lifecycle & Execution

### 1. Creation (via CW20 Send)
You cannot create a vesting by calling the contract directly. You must use the token's `send` method.

**Example: Creating a single vesting**
```json
{
  "create_vesting": {
    "beneficiary": "paxi1...",
    "category": "team",
    "revocable": true,
    "schedule": { "linear": { ... } }
  }
}
```

**Example: Batch creating vestings**
Total amount sent must be "1500".
```json
{
  "batch_create_vesting": {
    "vestings": [
      {
        "beneficiary": "paxi1_user_a...",
        "amount": "1000",
        "category": "seed",
        "revocable": false,
        "schedule": { ... }
      },
      {
        "beneficiary": "paxi1_user_b...",
        "amount": "500",
        "category": "seed",
        "revocable": true,
        "schedule": { ... }
      }
    ]
  }
}
```

### 2. Claiming Tokens
Beneficiaries claim their available tokens.

```json
{
  "claim": {
    "ids": [1, 2, 15]
  }
}
```

### 3. Revoking (Admin Only)
Only positions created with `revocable: true` can be revoked.

```json
{
  "revoke": {
    "id": 1
  }
}
```

### 4. Governance & Maintenance
```json
// Transfer admin rights
{ "update_admin": { "admin": "paxi1..." } }

// Emergency Pause
{ "set_paused": { "paused": true } }
```

---

## Querying State

### Config
Returns admin address and pause status.
```json
{ "config": {} }
```

### Single Vesting Details
Returns full details including claimable amount.
```json
{ "vesting": { "id": 1 } }
```

### Claimable Amount Only
Returns just the `Uint128` amount available for claiming.
```json
{ "claimable_amount": { "id": 1 } }
```

### List by Beneficiary / Category
Supports pagination via `start_after` (last ID seen) and `limit`.
```json
{
  "vestings_by_beneficiary": {
    "beneficiary": "paxi1...",
    "start_after": 5,
    "limit": 10
  }
}
```

### Global Statistics
Aggregated data for a specific token address.
```json
{
  "global_stats": {
    "token_address": "paxi1_token..."
  }
}
```

---

## Important Validations & Behaviors

### 1. Milestone Integrity
When using the `Custom` schedule, the contract enforces:
- **Order**: Milestones must be provided in ascending order of timestamp.
- **Matching**: The sum of `amount` in the milestone list must be **exactly equal** to the `amount` of tokens deposited for that position. If they don't match, the transaction reverts.

### 2. Pause Mechanism
When the contract is **Paused**:
- **Blocked**: All state-changing operations including `Receive` (creation), `Claim`, `Revoke`, and `UpdateAdmin`.
- **Allowed**: All Queries.
*Note: The admin can still unpause the contract using `set_paused: { paused: false }`.*

### 3. Revocation Freeze
When an admin revokes a vesting position:
1.  The contract calculates the `vested_amount` at that exact second.
2.  The `unvested_amount` (Total - Vested) is transferred back to the admin.
3.  The vesting position's `total_amount` is updated to the `vested_amount`.
4.  The `schedule` is replaced with a `Custom` schedule containing a single milestone at the current time.
5.  **Result**: No more tokens will ever vest for this position, but the beneficiary can still claim any `vested_but_unclaimed` tokens at their convenience.

---

## Technical Math Definitions

### Calculation of "Vested" Amount

**For Linear Schedules**:
1.  **If `CurrentTime < CliffTime`** (or `StartTime` if no cliff): Vested = 0.
2.  **If `CurrentTime >= EndTime`**: Vested = Total.
3.  **Otherwise**:
    - $Elapsed = CurrentTime - StartTime$
    - $Duration = EndTime - StartTime$
    - $EffectiveElapsed = \lfloor \frac{Elapsed}{Interval} \rfloor \times Interval$
    - $Vested = Total \times \frac{EffectiveElapsed}{Duration}$

**For Custom Schedules**:
- $Vested = \sum MilestoneAmounts$ where $MilestoneTimestamp \le CurrentTime$.

### Calculation of "Claimable" Amount
The amount a user actually receives when calling `Claim` is:
$Claimable = VestedAmount - ReleasedAmount$
where `ReleasedAmount` is the sum of all tokens already successfully claimed from that position.

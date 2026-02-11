# PRC20 Token Vesting - User Guide

The **PRC20 Token Vesting** contract is a production-grade system designed to manage the gradual release of PRC20 (CW20-compatible) tokens. It supports multiple vesting models, administrative controls, and efficient batch operations.

## Table of Contents
1. [Vesting Models](#vesting-models)
2. [Contract Deployment](#contract-deployment)
3. [Core Operations](#core-operations)
   - [Creating a Vesting Schedule](#creating-a-vesting-schedule)
   - [Batch Creation](#batch-creation)
   - [Claiming Tokens](#claiming-tokens)
   - [Revoking Vesting](#revoking-vesting)
4. [Administrative Functions](#administrative-functions)
5. [Querying State](#querying-state)
6. [Technical Details](#technical-details)

---

## Vesting Models

The contract supports two primary types of vesting schedules:

### 1. Linear Vesting (with optional Cliff)
Tokens are released continuously over a specified duration.
- **Start Time**: When the vesting period begins.
- **End Time**: When all tokens are fully vested.
- **Cliff Time (Optional)**: A specific point in time before which no tokens are released. At the cliff, all tokens that would have vested linearly since the start are unlocked at once.
- **Release Interval**: The frequency (in seconds) at which tokens are unlocked. Set to `1` for per-second continuous vesting.

### 2. Custom Milestones
Tokens are released at specific, pre-defined timestamps.
- **Milestones**: A list of `(timestamp, amount)` pairs.
- Useful for one-time unlocks or complex non-linear schedules.

---

## Contract Deployment

### Instantiation
To deploy the contract, you must provide the admin address.

**Message:**
```json
{
  "admin": "paxi1..."
}
```

---

## Core Operations

### Creating a Vesting Schedule
To create a vesting schedule, tokens MUST be transferred to the vesting contract using the CW20 `send` method. The vesting contract will process the `Receive` hook.

**CW20 Send Payload:**
```json
{
  "send": {
    "contract": "<VESTING_CONTRACT_ADDR>",
    "amount": "1000000000",
    "msg": "<BASE64_ENCODED_HOOK_MSG>"
  }
}
```

**Hook Message (`msg` before encoding):**
```json
{
  "create_vesting": {
    "beneficiary": "paxi1_beneficiary...",
    "category": "team",
    "revocable": true,
    "schedule": {
      "linear": {
        "start_time": 1700000000,
        "end_time": 1731536000,
        "cliff_time": 1715768000,
        "release_interval": 1
      }
    }
  }
}
```

### Batch Creation
You can create multiple vesting schedules for different beneficiaries in a single CW20 transfer. The total amount transferred must match the sum of all individual vesting amounts.

**Hook Message (`msg` before encoding):**
```json
{
  "batch_create_vesting": {
    "vestings": [
      {
        "beneficiary": "paxi1_user1...",
        "amount": "500000000",
        "category": "seed",
        "revocable": false,
        "schedule": { ... }
      },
      {
        "beneficiary": "paxi1_user2...",
        "amount": "500000000",
        "category": "advisor",
        "revocable": true,
        "schedule": { ... }
      }
    ]
  }
}
```

### Claiming Tokens
Beneficiaries (or anyone on their behalf) can claim vested tokens at any time.

**Message:**
```json
{
  "claim": {
    "ids": [1, 5, 12]
  }
}
```
*Tokens are always sent to the registered beneficiary address of the vesting position.*

### Revoking Vesting
Admins can revoke positions marked as `revocable`.
- **Vested tokens** remain in the contract and are claimable by the beneficiary.
- **Unvested tokens** are immediately returned to the admin.
- Once revoked, the total amount of the vesting is adjusted to the vested amount, and the schedule is frozen.

**Message:**
```json
{
  "revoke": {
    "id": 1
  }
}
```

---

## Administrative Functions

### Update Admin
Change the contract administrator.
```json
{
  "update_admin": {
    "admin": "paxi1_new_admin..."
  }
}
```

### Pause / Unpause
Pause all state-changing operations (except claiming, which remains active to ensure trust).
```json
{
  "set_paused": {
    "paused": true
  }
}
```

---

## Querying State

### Get Vesting Details
```json
{ "vesting": { "id": 1 } }
```

### List by Beneficiary
```json
{
  "vestings_by_beneficiary": {
    "beneficiary": "paxi1...",
    "start_after": 0,
    "limit": 10
  }
}
```

### Check Claimable Amount
```json
{ "claimable_amount": { "id": 1 } }
```

### Global Statistics
Get total vested and claimed amounts for a specific token address.
```json
{
  "global_stats": {
    "token_address": "paxi1_token_contract..."
  }
}
```

---

## Technical Details

### Linear Vesting Formula
$Vested = Total \times \frac{CurrentTime - StartTime}{EndTime - StartTime}$

*Notes:*
- If `CurrentTime < CliffTime`, $Vested = 0$.
- If `ReleaseInterval > 1`, the time is rounded down to the nearest interval: $EffectiveElapsed = \lfloor \frac{Elapsed}{Interval} \rfloor \times Interval$.

### Security Measures
- **Check-Effect-Interaction**: State is updated before tokens are transferred.
- **Overflow Protection**: Uses `Uint128` with `multiply_ratio` for high-precision math without overflow.
- **Revocation Freeze**: Revocation uses a `Custom` schedule with a single milestone to ensure no further vesting occurs while preserving the beneficiary's right to claim what they already earned.

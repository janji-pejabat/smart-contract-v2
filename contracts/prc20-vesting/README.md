# PRC20 Token Vesting Smart Contract

A professional, production-grade token vesting system for PRC20 (CW20-compatible) tokens.

## Features

- **Multiple Vesting Models**:
  - Cliff Vesting
  - Linear Vesting
  - Cliff + Linear Vesting
  - Custom Schedules (Multiple Milestones)
- **Flexible Configuration**:
  - Support for revocable and non-revocable vesting positions.
  - Vesting categories (Team, Advisor, Seed, etc.).
  - Custom release intervals.
- **Batch Operations**:
  - Batch create vesting schedules from a single CW20 transfer.
  - Batch claim vested tokens.
- **Security**:
  - Admin-only revocation for revocable vestings.
  - Pause mechanism for emergency situations.
  - Time-based vesting (block time).
  - Strict parameter validation.

## Architecture

- `src/contract.rs`: Core execution logic (instantiate, execute, query, migrate).
- `src/msg.rs`: Entry point message definitions and hook messages.
- `src/state.rs`: Storage structures and indexes.
- `src/vesting.rs`: Vesting mathematics and validation.
- `src/error.rs`: Custom contract errors.

## Usage

### 1. Creating a Vesting Schedule

Tokens are deposited into the contract using the CW20 `Receive` hook.

```json
{
  "create_vesting": {
    "beneficiary": "paxi1...",
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

### 2. Claiming Tokens

Beneficiaries can claim their vested tokens at any time.

```json
{
  "claim": {
    "ids": [1, 2, 3]
  }
}
```

### 3. Revoking Vesting (Admin Only)

If a vesting position is marked as `revocable`, the admin can revoke it. Vested tokens remain claimable by the beneficiary, while unvested tokens are returned to the admin.

```json
{
  "revoke": {
    "id": 1
  }
}
```

## Testing

Run unit tests:

```bash
cargo test
```

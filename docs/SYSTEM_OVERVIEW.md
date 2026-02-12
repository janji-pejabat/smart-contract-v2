# LP Platform v2.0 - System Architecture & Documentation

## 1. High-Level Flow Diagram

```text
[ USER ]
   |
   | (1) CW20 Transfer + Hook
   v
[ LP LOCKER ] ----------------------------> [ CW20 LP TOKEN ]
   |                                            (LP Custody)
   | (2) OnLock Hook (WasmMsg)
   v
[ REWARD CONTROLLER ] <-------------------- [ ADMIN ]
   |    (Pool Management & APR)                 | (Set APR, Deposit Rewards)
   |                                            |
   | (3) Reward Math (Global Index)             |
   |                                            |
   | (4) ClaimRewards (CosmosMsg)               |
   v                                            v
[ USER ] <--------------------------------- [ NATIVE/CW20 REWARDS ]
```

### Detailed Message Flow:
1. **Locking**: User sends LP tokens to `LP Locker`. Locker deducts platform fee (if any), creates a `Locker` record, and sends `OnLock` hook to `Reward Controller`.
2. **Accrual**: `Reward Controller` tracks the `Locker` and its `effective_amount` (Amount * Bonus Multiplier). It uses a Global Index per pool that increases over time based on APR.
3. **Claiming**: User calls `ClaimRewards` on `Reward Controller`. It updates the user's accrued rewards based on the time-weighted index and sends tokens to the user.
4. **Unlocking**: After `unlock_time`, user calls `UnlockLP` on `LP Locker`. Locker sends `OnUnlock` hook to `Reward Controller` (finalizing rewards), deducts fee, and returns LP to user.

## 2. Smart Contract Best Practices Applied

- **Check-Effect-Interaction (CEI)**: All state updates (e.g., removing lockers, updating reward indices) are performed BEFORE sending any messages (WasmMsg or BankMsg).
- **Separation of Concerns**: `LP Locker` handles token custody and time-locks; `Reward Controller` handles incentive math. Neither depends on the other's internal state beyond the hook interface.
- **Checked Arithmetic**: All mathematical operations use `checked_*` or `mul_floor` methods to prevent overflow/underflow.
- **Access Control**: Strict admin roles for pool management and configuration. The `Reward Controller` only accepts hooks from the configured `LP Locker` address.
- **Migration Support**: Both contracts implement `MigrateMsg` and use `cw2` versioning for safe upgrades.
- **Fee Management**: Platform fees are configurable by admin and deducted automatically during lock/unlock.

## 3. Audit Readiness Checklist

- [x] **Arithmetic**: No unchecked math operations.
- [x] **Reentrancy**: CEI pattern followed in all execution paths.
- [x] **Time Manipulation**: Reward accrual and lock times use `env.block.time`. Minimal exposure to miner-controlled timestamps.
- [x] **Rounding Errors**: Using `mul_floor` and `Decimal` for precision. Small rounding remainders in rewards are kept in the pool.
- [x] **Admin Abuse**: Admin cannot withdraw user LP tokens. Admin can only manage reward pools and whitelist tokens.
- [x] **Gas Limits**: Reward claiming and queries use pagination where appropriate (though reward claiming for many pools should be monitored).
- [x] **Input Validation**: All addresses are validated, amounts are checked for zero, and durations are bound by whitelist constraints.
- [x] **Hook Security**: `Reward Controller` verifies that `info.sender` is the trusted `LP Locker` contract for all lifecycle hooks.

## 4. UX Notes for Frontend

- **Bonus Multipliers**: Display the expected multiplier to users before they lock.
- **Pending Rewards**: Use the `PendingRewards` query to show real-time accrual (it accounts for time since the last on-chain update).
- **Claim Intervals**: Ensure users are aware of the minimum claim interval (default 1 hour).
- **Fees**: Transparently show the platform fee deduction on both Lock and Unlock operations.

# Paxi Network LP Locker & Reward Platform Report

## 1. System Confirmation & Review

### [A] LP Custody & Locking Logic
- **True LP Locking**: The `lp-locker` contract strictly accepts CW20 LP tokens via the `Receive` interface.
- **No Vesting**: There is no logic to partially release LP tokens over time. The `amount` returned on unlock is exactly the `amount` deposited.
- **Custody Safety**: LP tokens are held in the contract's balance. Access is restricted by `locker_id` and `owner` mapping.

### [B] Hidden Risks & Security Identification
- **Vesting-like behavior**: None found. The contract does not mint/burn or interact with AMM pools.
- **Drain Risks**:
    - *Locker*: Protected by ownership checks and time-lock enforcement.
    - *Reward Controller*: Protected by `total_deposited` vs `total_claimed` checks. APR updates are gated by admin.
    - *Reentrancy*: All state changes happen before external calls (CEI pattern).

---

## 2. Clean Architecture: Separated Concerns

The system is split into two independent contracts to minimize the attack surface on the primary LP custody:

1.  **Locker Contract**: Focuses on **CUSTODY**. It doesn't know about APR or rewards. It only emits hooks (`OnLock`, `OnExtend`, `OnUnlock`).
2.  **Reward Controller**: Focuses on **INCENTIVES**. It tracks user stakes and calculates rewards based on data provided by the Locker.

### Message Flow Diagram
```text
User            CW20 Token          LP Locker         Reward Controller
  |                 |                   |                     |
  |--- Send(Msg) -->|                   |                     |
  |                 |--- ReceiveMsg --->|                     |
  |                 |                   |--- OnLock Hook ---->| (Store stake)
  |                 |                   |<--- Response -------|
  |                 |                   |                     |
  |---------------------- ClaimRewards ---------------------->| (Calculate index)
  |                                     |                     | (Transfer Reward)
  |                                     |                     |
  |---------------------- UnlockLP -------------------------->|
  |                 |                   |--- OnUnlock Hook -->| (Auto-Claim)
  |<-- Transfer ----|                   |                     | (Remove stake)
```

---

## 3. Advanced Features Implemented

- **Multi-Token Rewards**: Supports Native (PAXI) and CW20 reward assets.
- **Bonus Multipliers**: Incentivizes longer locks (1.0x to 2.5x) based on duration.
- **Referral System**: 5% commission for referrers to drive growth.
- **Dynamic APR**: Automated balancing. Boosts APR when TVL is low to attract liquidity; reduces when TVL is high to manage inflation.
- **Scalable Indexing**: Uses secondary indices to avoid $O(N)$ scans, ensuring the platform stays fast as users grow.

---

## 4. Smart Contract Best Practices Applied

1.  **Check-Effect-Interaction (CEI)**: Always update internal state (accrued rewards, claimed amounts) before making a bank transfer or CW20 call.
2.  **Explicit Errors**: Used custom `ContractError` for clear debugging and gas savings.
3.  **Checked Math**: All calculations use `checked_add`, `checked_sub`, and `multiply_ratio` to prevent overflow/underflow.
4.  **Admin Safety**: Admin cannot withdraw user-locked LP or shorten lock times.
5.  **State Versioning**: Standard CW2 headers included for safe future migrations.

---

## 5. Audit Readiness Checklist

- [x] All math is checked/saturating.
- [x] No `panic!` in production code paths.
- [x] Admin roles are strictly limited to configuration.
- [x] `AssetInfo` handles both Native and CW20 consistently.
- [x] Hook calls from Locker to Reward Controller are authenticated by contract address.
- [x] Referral cycles (self-referral) are prevented.
- [x] Total supply/TVL invariants are maintained across all hooks.

---

## 6. UX & Frontend Integration Notes

- **Querying Rewards**: Use `PendingRewards` query for real-time display. It accounts for time elapsed since the last block.
- **Lock Extension**: Always check if the new `unlock_time` is greater than the current one; the contract will reject any decrease.
- **Platform Fees**: Fees are subtracted on both Lock and Unlock. Frontend should display the "Net Amount" to avoid user confusion.
- **Referrals**: Encourage users to register a referral *before* their first lock to maximize rewards.

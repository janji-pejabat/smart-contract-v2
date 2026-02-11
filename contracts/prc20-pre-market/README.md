# 🛡️ Paxi Pre-Market: PRC20 OTC Smart Contract

The **Paxi Pre-Market** contract is a professional-grade, escrow-based OTC (Over-The-Counter) trading protocol for PRC20 tokens on the Paxi Network. It allows sellers to list tokens with complex, multi-round schedules while ensuring atomic settlement and security for both buyers and sellers.

---

## 🎯 Overview

This contract facilitates the trading of PRC20 tokens before they are listed on public DEXs. It eliminates the need for trust between parties by acting as a decentralized escrow.

### Core Workflow:
1. **Escrow**: Seller sends tokens to the contract to create a listing.
2. **Scheduling**: Seller defines one or more sale rounds (e.g., Private Sale, Public Sale).
3. **Atomic Trade**: Buyer pays in native **PAXI** tokens.
4. **Settlement**: The contract immediately transfers PRC20 to the buyer and PAXI (minus fees) to the seller.

---

## 🚀 Key Features

### 1. Multi-Round Scheduling
Each listing can have multiple distinct rounds. Each round supports independent:
- **Pricing**: Change the price per token dynamically over time.
- **Time Windows**: Define exact start and end times for each phase.
- **Whitelisting**: Restrict access to specific addresses for private rounds.
- **Wallet Limits**: Prevent whales from buying out the entire allocation.

### 2. High-Performance Queries
Uses `IndexedMap` to allow O(log N) complexity when searching for listings by **Seller** or **Token Address**. This ensures the protocol remains gas-efficient even with thousands of active listings.

### 3. Integrated Fee Logic
- **Platform Fee**: Configurable percentage taken by the protocol.
- **Referral System**: Referrers receive **5%** of the platform fee.
- **Creator Royalties**: Support for additional royalties paid to a project or creator address.

### 4. Escrow Security
Tokens are locked in the contract at the moment of listing creation. Sellers can only withdraw their tokens by cancelling the unsold portion of their listing. The admin has **NO** ability to withdraw escrowed tokens.

---

## ⚙️ Technical Explanation

### 1. Buy Execution Flow
When a buyer executes a `Buy` message:
1. **Round Detection**: The contract identifies the currently active round based on `env.block.time`.
2. **Permission Check**: Verifies if the buyer is whitelisted (if the round is private).
3. **Limit Check**: Ensures the purchase doesn't exceed the round's `max_wallet_limit` or the listing's `max_buy`.
4. **Payment Verification**: Confirms that the exact amount of native PAXI has been sent.
5. **Fee Distribution**:
    - Calculates Platform Fee, Referral (if applicable), and Royalty.
    - Sends fees via `BankMsg::Send`.
    - Sends the remaining balance to the seller.
6. **Token Delivery**: Transfers the PRC20 tokens from escrow to the buyer via `WasmMsg::Execute`.

### 2. Math & Precision
The contract uses `Uint128` and `checked_math` for all operations. Pricing calculations use `multiply_ratio` to ensure precision and prevent overflow.

---

## 🛠️ Execution Messages

### 1. Create Listing (via PRC20/CW20 `Send`)
To create a listing, you must send tokens to the contract using the CW20 `Send` interface. The `msg` field must contain a base64-encoded `Cw20HookMsg::CreateListing`.

**Request Structure:**
```json
{
  "create_listing": {
    "min_buy": "1000000",
    "max_buy": "50000000",
    "metadata": "Strategic Seed Round",
    "royalty_address": "paxi1...",
    "royalty_bps": 200,
    "rounds": [
      {
        "name": "Whitelisted Round",
        "start_time": 1740000000,
        "end_time": 1740100000,
        "price_per_token": "50",
        "max_wallet_limit": "10000000",
        "whitelist": ["paxi1buyerA...", "paxi1buyerB..."]
      },
      {
        "name": "Public Round",
        "start_time": 1740100001,
        "end_time": 1740200000,
        "price_per_token": "75",
        "max_wallet_limit": null,
        "whitelist": null
      }
    ]
  }
}
```

### 2. Buy
Atomic purchase of tokens using native PAXI.
```json
{
  "buy": {
    "listing_id": 42,
    "amount": "5000000",
    "referrer": "paxi1referraladdr..."
  }
}
```

### 3. Cancel Listing
Stops the listing and returns all remaining PRC20 tokens to the seller. Only allowed for the seller.
```json
{
  "cancel_listing": {
    "listing_id": 42
  }
}
```

### 4. Update Config (Admin Only)
```json
{
  "update_config": {
    "platform_fee_bps": 150,
    "fee_receiver": "paxi1newtreasury...",
    "paused": false
  }
}
```

---

## 🔍 Query Messages

### 1. Get Listing Details
Returns all data for a specific listing, including current round status.
```json
{ "listing": { "id": 42 } }
```

### 2. Search Listings by Seller
Uses indexed lookup for high performance.
```json
{
  "listings_by_seller": {
    "seller": "paxi1...",
    "limit": 10,
    "start_after": null
  }
}
```

### 3. Search Listings by Token
Find all listings for a specific PRC20 token.
```json
{
  "listings_by_token": {
    "token": "paxi1tokenaddr...",
    "limit": 10
  }
}
```

### 4. Buyer Purchase History
Retrieves the history of all successful trades for a specific wallet.
```json
{
  "buyer_purchase_history": {
    "buyer": "paxi1...",
    "limit": 5
  }
}
```

---

## 🛡️ Validations & Behaviors

| Feature | Behavior |
| :--- | :--- |
| **Paused State** | While the contract is paused, `CreateListing` and `Buy` are disabled. Sellers can still `Cancel` their listings. |
| **Fee Cap** | Platform fees cannot exceed 50% (5000 bps) to prevent administrative abuse. |
| **Self-Buying** | Sellers are prohibited from buying their own listings to prevent wash trading. |
| **Round Continuity** | The contract validates that rounds are chronological and do not overlap in time during creation. |
| **Token Blacklist** | The admin can blacklist specific PRC20 addresses. Blacklisted tokens cannot be listed for sale. |

---

## 📊 Volume Statistics
The contract tracks global protocol health through the `Stats` query:
- `total_volume_paxi`: Total value traded through the protocol.
- `total_trades`: Total count of successful buy transactions.
- `total_fees_collected`: Cumulative platform fees generated.

# API Reference

## LP Locker Contract

### Execute Messages

#### LockLP (via CW20 Send)
```json
{
  "send": {
    "contract": "locker_address",
    "amount": "1000000",
    "msg": "<base64_encoded_hook>"
  }
}

// Hook message:
{
  "lock_lp": {
    "unlock_time": 1735689600,
    "metadata": "Optional project info"
  }
}
```

#### UnlockLP
```json
{
  "unlock_lp": {
    "locker_id": 1
  }
}
```

#### ExtendLock
```json
{
  "extend_lock": {
    "locker_id": 1,
    "new_unlock_time": 1767225600
  }
}
```

### Query Messages

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
    "start_after": null,
    "limit": 10
  }
}
```

## Reward Controller Contract

### Execute Messages

#### RegisterStake (Optional)
Note: Only needed for lockers created before the reward controller deployment.
```json
{
  "register_stake": {
    "locker_id": 1
  }
}
```

#### ClaimRewards
```json
{
  "claim_rewards": {
    "pool_ids": [0, 1]
  }
}
```

### Query Messages

#### PendingRewards
```json
{
  "pending_rewards": {
    "user": "paxi1...",
    "pool_id": 0
  }
}
```

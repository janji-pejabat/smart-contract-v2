use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, Decimal, Addr, CosmosMsg, WasmMsg, BankMsg, Coin,
    Storage,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    RewardPoolResponse, UserStakeResponse, PendingRewardsResponse,
    LockerHookMsg, Cw20HookMsg,
};
use crate::state::{
    RewardConfig, RewardPool, UserStake, UserReward, AssetInfo,
    CONFIG, POOLS, USER_STAKES, USER_REWARDS, TOTAL_STAKED,
    USER_STAKED_LOCKERS,
};

const CONTRACT_NAME: &str = "crates.io:reward-controller";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;
    let lp_locker = deps.api.addr_validate(&msg.lp_locker_contract)?;

    let config = RewardConfig {
        admin,
        lp_locker_contract: lp_locker,
        paused: false,
        claim_interval: msg.claim_interval.unwrap_or(3600), // 1 hour default
        next_pool_id: 0,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute("lp_locker_contract", msg.lp_locker_contract))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::LockerHook(hook_msg) => execute_locker_hook(deps, env, info, hook_msg),
        ExecuteMsg::ClaimRewards { locker_id, pool_ids } => {
            execute_claim_rewards(deps, env, info, locker_id, pool_ids)
        }
        ExecuteMsg::CreateRewardPool {
            lp_token,
            reward_token,
            apr,
        } => execute_create_pool(deps, env, info, lp_token, reward_token, apr),
        ExecuteMsg::UpdateRewardPool {
            pool_id,
            apr,
            enabled,
        } => execute_update_pool(deps, env, info, pool_id, apr, enabled),
        ExecuteMsg::DepositRewards { pool_id } => {
            execute_deposit_rewards(deps, env, info, pool_id)
        }
        ExecuteMsg::WithdrawRewards { pool_id, amount } => {
            execute_withdraw_rewards(deps, env, info, pool_id, amount)
        }
        ExecuteMsg::UpdateConfig {
            admin,
            lp_locker_contract,
            claim_interval,
        } => execute_update_config(deps, info, admin, lp_locker_contract, claim_interval),
        ExecuteMsg::Pause {} => execute_pause(deps, info),
        ExecuteMsg::Resume {} => execute_resume(deps, info),
    }
}

fn execute_receive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    wrapper: cw20::Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: Cw20HookMsg = cosmwasm_std::from_json(&wrapper.msg)?;
    match msg {
        Cw20HookMsg::DepositRewards { pool_id } => {
            let mut pool = POOLS.load(deps.storage, pool_id)?;
            match &pool.reward_token {
                AssetInfo::Cw20(addr) => {
                    if addr != info.sender {
                        return Err(ContractError::Unauthorized {});
                    }
                }
                AssetInfo::Native(_) => return Err(ContractError::Unauthorized {}),
            }
            pool.total_deposited = pool.total_deposited.checked_add(wrapper.amount).map_err(cosmwasm_std::StdError::from)?;
            POOLS.save(deps.storage, pool_id, &pool)?;

            Ok(Response::new()
                .add_attribute("action", "deposit_rewards")
                .add_attribute("amount", wrapper.amount))
        }
    }
}

fn execute_locker_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: LockerHookMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.lp_locker_contract {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        LockerHookMsg::OnLock { locker_id, owner, lp_token, amount, unlock_time } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let lp_token_addr = deps.api.addr_validate(&lp_token)?;
            let duration = unlock_time.checked_sub(env.block.time.seconds()).unwrap_or(0);
            let bonus_multiplier = calculate_bonus_multiplier(duration);

            let stake = UserStake {
                user: owner_addr.clone(),
                locker_id,
                lp_token: lp_token_addr.clone(),
                lp_amount: amount,
                bonus_multiplier,
            };
            USER_STAKES.save(deps.storage, locker_id, &stake)?;
            USER_STAKED_LOCKERS.save(deps.storage, (&owner_addr, locker_id), &true)?;

            // Find all pools for this LP token and initialize user reward records
            let pool_ids = get_pools_for_lp(deps.as_ref(), &lp_token_addr)?;
            for pool_id in pool_ids {
                let pool = update_pool(deps.storage, &env, pool_id)?;
                USER_REWARDS.save(deps.storage, (locker_id, pool_id), &UserReward {
                    locker_id,
                    pool_id,
                    reward_per_token_paid: pool.reward_per_token_stored,
                    rewards_accrued: Uint128::zero(),
                    last_claim_time: env.block.time.seconds(),
                })?;
            }

            TOTAL_STAKED.update(deps.storage, &lp_token_addr, |total| -> StdResult<_> {
                Ok(total.unwrap_or_default().checked_add(amount).map_err(cosmwasm_std::StdError::from)?)
            })?;
        }
        LockerHookMsg::OnExtend { locker_id, new_unlock_time } => {
            let mut stake = USER_STAKES.load(deps.storage, locker_id)?;
            let duration = new_unlock_time.checked_sub(env.block.time.seconds()).unwrap_or(0);
            let new_multiplier = calculate_bonus_multiplier(duration);

            // Update all user reward records before changing multiplier
            let pool_ids = get_pools_for_lp(deps.as_ref(), &stake.lp_token)?;
            for pool_id in pool_ids {
                update_user_reward(deps.storage, &env, locker_id, pool_id)?;
            }

            stake.bonus_multiplier = new_multiplier;
            USER_STAKES.save(deps.storage, locker_id, &stake)?;
        }
        LockerHookMsg::OnUnlock { locker_id, owner: _ } => {
            let stake = USER_STAKES.load(deps.storage, locker_id)?;

            // Final update of rewards before removing stake
            let pool_ids = get_pools_for_lp(deps.as_ref(), &stake.lp_token)?;
            for pool_id in pool_ids {
                update_user_reward(deps.storage, &env, locker_id, pool_id)?;
            }

            USER_STAKED_LOCKERS.remove(deps.storage, (&stake.user, locker_id));
            USER_STAKES.remove(deps.storage, locker_id);
            TOTAL_STAKED.update(deps.storage, &stake.lp_token, |total| -> StdResult<_> {
                Ok(total.unwrap_or_default().checked_sub(stake.lp_amount).map_err(cosmwasm_std::StdError::from)?)
            })?;
        }
    }

    Ok(Response::new().add_attribute("action", "locker_hook"))
}

fn calculate_bonus_multiplier(duration: u64) -> Decimal {
    if duration >= 365 * 86400 {
        Decimal::from_ratio(25u128, 10u128) // 2.5x
    } else if duration >= 181 * 86400 {
        Decimal::from_ratio(20u128, 10u128) // 2.0x
    } else if duration >= 91 * 86400 {
        Decimal::from_ratio(15u128, 10u128) // 1.5x
    } else if duration >= 31 * 86400 {
        Decimal::from_ratio(12u128, 10u128) // 1.2x
    } else {
        Decimal::one() // 1.0x
    }
}

fn get_pools_for_lp(deps: Deps, lp_token: &Addr) -> StdResult<Vec<u64>> {
    let pools: Vec<u64> = POOLS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            if let Ok((id, pool)) = item {
                if pool.lp_token == *lp_token {
                    return Some(id);
                }
            }
            None
        })
        .collect();
    Ok(pools)
}

fn update_pool(storage: &mut dyn Storage, env: &Env, pool_id: u64) -> Result<RewardPool, ContractError> {
    let mut pool = POOLS.load(storage, pool_id)?;
    if pool.last_update >= env.block.time.seconds() {
        return Ok(pool);
    }

    let time_elapsed = env.block.time.seconds().saturating_sub(pool.last_update);
    if time_elapsed > 0 && pool.enabled {
        let year_seconds = 365 * 24 * 3600u128;
        let apr_increase = pool.apr.checked_mul(Decimal::from_ratio(time_elapsed as u128, year_seconds)).map_err(cosmwasm_std::StdError::from)?;
        pool.reward_per_token_stored = pool.reward_per_token_stored.checked_add(apr_increase).map_err(cosmwasm_std::StdError::from)?;
    }

    pool.last_update = env.block.time.seconds();
    POOLS.save(storage, pool_id, &pool)?;
    Ok(pool)
}

fn update_user_reward(
    storage: &mut dyn Storage,
    env: &Env,
    locker_id: u64,
    pool_id: u64,
) -> Result<(), ContractError> {
    let pool = update_pool(storage, env, pool_id)?;
    let stake = USER_STAKES.may_load(storage, locker_id)?;

    if let Some(stake) = stake {
        // SECURITY FIX: Verify stake LP token matches pool LP token
        if stake.lp_token != pool.lp_token {
            return Err(ContractError::Unauthorized {});
        }

        let mut user_reward = USER_REWARDS
            .may_load(storage, (locker_id, pool_id))?
            .unwrap_or(UserReward {
                locker_id,
                pool_id,
                reward_per_token_paid: pool.reward_per_token_stored,
                rewards_accrued: Uint128::zero(),
                last_claim_time: env.block.time.seconds(),
            });

        let pending = calculate_pending_for_user(&stake, &pool, &user_reward)?;
        user_reward.rewards_accrued = user_reward.rewards_accrued.checked_add(pending).map_err(cosmwasm_std::StdError::from)?;
        user_reward.reward_per_token_paid = pool.reward_per_token_stored;
        USER_REWARDS.save(storage, (locker_id, pool_id), &user_reward)?;
    }
    Ok(())
}

fn calculate_pending_for_user(
    stake: &UserStake,
    pool: &RewardPool,
    user_reward: &UserReward,
) -> StdResult<Uint128> {
    let reward_per_token_diff = pool.reward_per_token_stored.checked_sub(user_reward.reward_per_token_paid)?;
    if reward_per_token_diff.is_zero() {
        return Ok(Uint128::zero());
    }

    let effective_amount = stake.lp_amount.mul_floor(stake.bonus_multiplier);
    let pending = effective_amount.mul_floor(reward_per_token_diff);
    Ok(pending)
}

fn execute_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
    pool_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let stake = USER_STAKES.load(deps.storage, locker_id)?;

    if stake.user != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut total_claimed = Uint128::zero();

    for pool_id in pool_ids {
        update_user_reward(deps.storage, &env, locker_id, pool_id)?;

        let mut user_reward = USER_REWARDS.load(deps.storage, (locker_id, pool_id))?;
        let mut pool = POOLS.load(deps.storage, pool_id)?;

        let amount = user_reward.rewards_accrued;
        if amount.is_zero() {
            continue;
        }

        if env.block.time.seconds().saturating_sub(user_reward.last_claim_time) < config.claim_interval {
            return Err(ContractError::ClaimTooSoon {});
        }

        // Check if pool has enough rewards
        let available = pool.total_deposited.saturating_sub(pool.total_claimed);
        if amount > available {
            // Partial claim if pool is running low? Or fail?
            // Better to fail or claim available. Let's fail for now to be safe.
            return Err(ContractError::InsufficientRewards {});
        }

        user_reward.rewards_accrued = Uint128::zero();
        user_reward.last_claim_time = env.block.time.seconds();
        USER_REWARDS.save(deps.storage, (locker_id, pool_id), &user_reward)?;

        pool.total_claimed = pool.total_claimed.checked_add(amount).map_err(cosmwasm_std::StdError::from)?;
        POOLS.save(deps.storage, pool_id, &pool)?;

        let transfer_msg = match &pool.reward_token {
            AssetInfo::Cw20(addr) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount,
                })?,
                funds: vec![],
            }),
            AssetInfo::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount,
                }],
            }),
        };

        messages.push(transfer_msg);
        total_claimed = total_claimed.checked_add(amount).map_err(cosmwasm_std::StdError::from)?;
    }

    if total_claimed.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim_rewards")
        .add_attribute("total_claimed", total_claimed))
}

fn execute_create_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token: String,
    reward_token: AssetInfo,
    apr: Decimal,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let lp_token_addr = deps.api.addr_validate(&lp_token)?;

    let pool_id = config.next_pool_id;
    config.next_pool_id += 1;
    CONFIG.save(deps.storage, &config)?;

    let pool = RewardPool {
        pool_id,
        lp_token: lp_token_addr,
        reward_token,
        total_deposited: Uint128::zero(),
        total_claimed: Uint128::zero(),
        apr,
        last_update: env.block.time.seconds(),
        reward_per_token_stored: Decimal::zero(),
        enabled: true,
    };

    POOLS.save(deps.storage, pool_id, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "create_reward_pool")
        .add_attribute("pool_id", pool_id.to_string()))
}

fn execute_update_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_id: u64,
    apr: Option<Decimal>,
    enabled: Option<bool>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Update pool index before changing parameters
    let mut pool = update_pool(deps.storage, &env, pool_id)?;

    if let Some(new_apr) = apr {
        pool.apr = new_apr;
    }

    if let Some(status) = enabled {
        pool.enabled = status;
    }

    POOLS.save(deps.storage, pool_id, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "update_pool")
        .add_attribute("pool_id", pool_id.to_string()))
}

fn execute_deposit_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool_id: u64,
) -> Result<Response, ContractError> {
    let mut pool = POOLS.load(deps.storage, pool_id)?;

    // For CW20 tokens, this would be called via Receive hook
    // For native tokens, check info.funds
    let deposit_amount = if !info.funds.is_empty() {
        info.funds[0].amount
    } else {
        Uint128::zero()
    };

    pool.total_deposited = pool.total_deposited.checked_add(deposit_amount).map_err(cosmwasm_std::StdError::from)?;
    POOLS.save(deps.storage, pool_id, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "deposit_rewards")
        .add_attribute("amount", deposit_amount))
}

fn execute_withdraw_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool_id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut pool = POOLS.load(deps.storage, pool_id)?;

    let available = pool.total_deposited.checked_sub(pool.total_claimed).map_err(cosmwasm_std::StdError::from)?;
    if amount > available {
        return Err(ContractError::InsufficientRewards {});
    }

    pool.total_deposited = pool.total_deposited.checked_sub(amount).map_err(cosmwasm_std::StdError::from)?;
    POOLS.save(deps.storage, pool_id, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "withdraw_rewards")
        .add_attribute("amount", amount))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    lp_locker_contract: Option<String>,
    claim_interval: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(addr) = admin {
        config.admin = deps.api.addr_validate(&addr)?;
    }

    if let Some(addr) = lp_locker_contract {
        config.lp_locker_contract = deps.api.addr_validate(&addr)?;
    }

    if let Some(interval) = claim_interval {
        config.claim_interval = interval;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_pause(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.paused = true;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "pause"))
}

fn execute_resume(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.paused = false;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "resume"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::RewardPool { pool_id } => to_json_binary(&query_pool(deps, pool_id)?),
        QueryMsg::AllRewardPools { start_after, limit } => {
            to_json_binary(&query_all_pools(deps, start_after, limit)?)
        }
        QueryMsg::UserStake { user, locker_id } => {
            to_json_binary(&query_user_stake(deps, user, locker_id)?)
        }
        QueryMsg::PendingRewards { user, pool_id } => {
            to_json_binary(&query_pending_rewards(deps, env, user, pool_id)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin,
        lp_locker_contract: config.lp_locker_contract,
        paused: config.paused,
        claim_interval: config.claim_interval,
        next_pool_id: config.next_pool_id,
    })
}

fn query_pool(deps: Deps, pool_id: u64) -> StdResult<RewardPoolResponse> {
    let pool = POOLS.load(deps.storage, pool_id)?;
    Ok(RewardPoolResponse {
        pool_id: pool.pool_id,
        lp_token: pool.lp_token,
        reward_token: pool.reward_token,
        total_deposited: pool.total_deposited,
        total_claimed: pool.total_claimed,
        apr: pool.apr,
        enabled: pool.enabled,
    })
}

fn query_all_pools(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<RewardPoolResponse>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(|id| cw_storage_plus::Bound::exclusive(id));

    POOLS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, pool) = item?;
            Ok(RewardPoolResponse {
                pool_id: pool.pool_id,
                lp_token: pool.lp_token,
                reward_token: pool.reward_token,
                total_deposited: pool.total_deposited,
                total_claimed: pool.total_claimed,
                apr: pool.apr,
                enabled: pool.enabled,
            })
        })
        .collect()
}

fn query_user_stake(deps: Deps, _user: String, locker_id: u64) -> StdResult<UserStakeResponse> {
    let stake = USER_STAKES.load(deps.storage, locker_id)?;

    Ok(UserStakeResponse {
        user: stake.user,
        locker_id: stake.locker_id,
        lp_token: stake.lp_token,
        lp_amount: stake.lp_amount,
        bonus_multiplier: stake.bonus_multiplier,
    })
}

fn query_pending_rewards(
    deps: Deps,
    env: Env,
    user: String,
    pool_id: u64,
) -> StdResult<PendingRewardsResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let mut total_pending = Uint128::zero();

    // SCALABILITY FIX: Use USER_STAKED_LOCKERS index instead of range scan
    let lockers: Vec<u64> = USER_STAKED_LOCKERS
        .prefix(&user_addr)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            if let Ok((locker_id, _)) = item {
                return Some(locker_id);
            }
            None
        })
        .collect();

    for locker_id in lockers {
        if let Ok(pending) = calculate_pending_rewards_for_locker(deps, &env, locker_id, pool_id) {
            total_pending = total_pending.checked_add(pending).map_err(cosmwasm_std::StdError::from)?;
        }
    }

    Ok(PendingRewardsResponse {
        pool_id,
        pending_amount: total_pending,
    })
}

fn calculate_pending_rewards_for_locker(
    deps: Deps,
    env: &Env,
    locker_id: u64,
    pool_id: u64,
) -> StdResult<Uint128> {
    let mut pool = POOLS.load(deps.storage, pool_id)?;
    let stake = USER_STAKES.load(deps.storage, locker_id)?;
    let user_reward = USER_REWARDS
        .may_load(deps.storage, (locker_id, pool_id))?
        .unwrap_or(UserReward {
            locker_id,
            pool_id,
            reward_per_token_paid: pool.reward_per_token_stored,
            rewards_accrued: Uint128::zero(),
            last_claim_time: 0,
        });

    let time_elapsed = env.block.time.seconds().saturating_sub(pool.last_update);
    if time_elapsed > 0 && pool.enabled {
        let year_seconds = 365 * 24 * 3600u128;
        let apr_increase = pool.apr.checked_mul(Decimal::from_ratio(time_elapsed as u128, year_seconds)).map_err(cosmwasm_std::StdError::from)?;
        pool.reward_per_token_stored = pool.reward_per_token_stored.checked_add(apr_increase).map_err(cosmwasm_std::StdError::from)?;
    }

    let pending = calculate_pending_for_user(&stake, &pool, &user_reward)?;
    Ok(user_reward.rewards_accrued.checked_add(pending).map_err(cosmwasm_std::StdError::from)?)
}

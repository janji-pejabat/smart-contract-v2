use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, PendingRewardsResponse, QueryMsg,
    RewardPoolResponse, UserStakeResponse,
};
use crate::state::{
    AssetInfo, RewardConfig, RewardPool, UserReward, UserStake, CONFIG, POOLS, TOTAL_STAKED,
    USER_REWARDS, USER_STAKES,
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
    TOTAL_STAKED.save(deps.storage, &Uint128::zero())?;

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
        ExecuteMsg::RegisterStake { locker_id } => {
            execute_register_stake(deps, env, info, locker_id)
        }
        ExecuteMsg::UnregisterStake { locker_id } => {
            execute_unregister_stake(deps, env, info, locker_id)
        }
        ExecuteMsg::ClaimRewards { pool_ids } => execute_claim_rewards(deps, env, info, pool_ids),
        ExecuteMsg::CreateRewardPool {
            reward_token,
            emission_per_second,
            start_time,
            end_time,
        } => execute_create_pool(
            deps,
            env,
            info,
            reward_token,
            emission_per_second,
            start_time,
            end_time,
        ),
        ExecuteMsg::UpdateRewardPool {
            pool_id,
            emission_per_second,
            end_time,
            enabled,
        } => execute_update_pool(
            deps,
            env,
            info,
            pool_id,
            emission_per_second,
            end_time,
            enabled,
        ),
        ExecuteMsg::DepositRewards { pool_id } => execute_deposit_rewards(deps, env, info, pool_id),
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

fn execute_register_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.paused {
        return Err(ContractError::Paused {});
    }

    // Query LP locker to verify locker exists and get details
    // TODO: Implement actual query to lp-locker contract
    // For now, we'll create a simple stake record

    let stake = UserStake {
        user: info.sender.clone(),
        locker_id,
        lp_amount: Uint128::from(1000000u128), // TODO: Get from locker query
        lock_start: env.block.time.seconds(),
        lock_duration: 86400 * 30,        // TODO: Calculate from locker
        bonus_multiplier: Decimal::one(), // TODO: Get from whitelist
    };

    USER_STAKES.save(deps.storage, (&info.sender, locker_id), &stake)?;

    // Update total staked
    TOTAL_STAKED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_add(stake.lp_amount)?)
    })?;

    Ok(Response::new()
        .add_attribute("action", "register_stake")
        .add_attribute("user", info.sender)
        .add_attribute("locker_id", locker_id.to_string()))
}

fn execute_unregister_stake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    locker_id: u64,
) -> Result<Response, ContractError> {
    let stake = USER_STAKES.load(deps.storage, (&info.sender, locker_id))?;

    USER_STAKES.remove(deps.storage, (&info.sender, locker_id));

    // Update total staked
    TOTAL_STAKED.update(deps.storage, |total| -> StdResult<_> {
        Ok(total.checked_sub(stake.lp_amount)?)
    })?;

    Ok(Response::new()
        .add_attribute("action", "unregister_stake")
        .add_attribute("locker_id", locker_id.to_string()))
}

fn execute_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut total_claimed = Uint128::zero();

    for pool_id in pool_ids {
        let pool = POOLS.load(deps.storage, pool_id)?;

        if !pool.enabled {
            continue;
        }

        // Calculate pending rewards
        let pending = calculate_pending_rewards(deps.as_ref(), &info.sender, pool_id)?;

        if pending.is_zero() {
            continue;
        }

        // Update user reward state
        let mut user_reward = USER_REWARDS
            .may_load(deps.storage, (&info.sender, pool_id))?
            .unwrap_or(UserReward {
                user: info.sender.clone(),
                pool_id,
                reward_per_token_paid: Decimal::zero(),
                rewards_accrued: Uint128::zero(),
                last_claim_time: 0,
            });

        // Check claim interval
        if env.block.time.seconds() - user_reward.last_claim_time < config.claim_interval {
            return Err(ContractError::ClaimTooSoon {});
        }

        user_reward.rewards_accrued = Uint128::zero();
        user_reward.last_claim_time = env.block.time.seconds();
        USER_REWARDS.save(deps.storage, (&info.sender, pool_id), &user_reward)?;

        // Create transfer message
        let transfer_msg = match &pool.reward_token {
            AssetInfo::Cw20(addr) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: pending,
                })?,
                funds: vec![],
            }),
            AssetInfo::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: pending,
                }],
            }),
        };

        messages.push(transfer_msg);
        total_claimed = total_claimed.checked_add(pending)?;
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
    reward_token: AssetInfo,
    emission_per_second: Uint128,
    start_time: u64,
    end_time: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let pool_id = config.next_pool_id;
    config.next_pool_id += 1;
    CONFIG.save(deps.storage, &config)?;

    let pool = RewardPool {
        pool_id,
        reward_token,
        total_deposited: Uint128::zero(),
        total_claimed: Uint128::zero(),
        emission_per_second,
        start_time,
        end_time,
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
    _env: Env,
    info: MessageInfo,
    pool_id: u64,
    emission_per_second: Option<Uint128>,
    end_time: Option<u64>,
    enabled: Option<bool>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut pool = POOLS.load(deps.storage, pool_id)?;

    if let Some(emission) = emission_per_second {
        pool.emission_per_second = emission;
    }

    if let Some(time) = end_time {
        pool.end_time = Some(time);
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

    pool.total_deposited = pool.total_deposited.checked_add(deposit_amount)?;
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

    let available = pool.total_deposited.checked_sub(pool.total_claimed)?;
    if amount > available {
        return Err(ContractError::InsufficientRewards {});
    }

    pool.total_deposited = pool.total_deposited.checked_sub(amount)?;
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
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
            to_json_binary(&query_pending_rewards(deps, user, pool_id)?)
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
        reward_token: pool.reward_token,
        total_deposited: pool.total_deposited,
        total_claimed: pool.total_claimed,
        emission_per_second: pool.emission_per_second,
        start_time: pool.start_time,
        end_time: pool.end_time,
        enabled: pool.enabled,
    })
}

fn query_all_pools(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<RewardPoolResponse>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(cw_storage_plus::Bound::exclusive);

    POOLS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, pool) = item?;
            Ok(RewardPoolResponse {
                pool_id: pool.pool_id,
                reward_token: pool.reward_token,
                total_deposited: pool.total_deposited,
                total_claimed: pool.total_claimed,
                emission_per_second: pool.emission_per_second,
                start_time: pool.start_time,
                end_time: pool.end_time,
                enabled: pool.enabled,
            })
        })
        .collect()
}

fn query_user_stake(deps: Deps, user: String, locker_id: u64) -> StdResult<UserStakeResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let stake = USER_STAKES.load(deps.storage, (&user_addr, locker_id))?;

    Ok(UserStakeResponse {
        user: stake.user,
        locker_id: stake.locker_id,
        lp_amount: stake.lp_amount,
        lock_start: stake.lock_start,
        lock_duration: stake.lock_duration,
        bonus_multiplier: stake.bonus_multiplier,
    })
}

fn query_pending_rewards(
    deps: Deps,
    user: String,
    pool_id: u64,
) -> StdResult<PendingRewardsResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let pending = calculate_pending_rewards(deps, &user_addr, pool_id)?;

    Ok(PendingRewardsResponse {
        pool_id,
        pending_amount: pending,
    })
}

// Helper function
fn calculate_pending_rewards(deps: Deps, user: &Addr, pool_id: u64) -> StdResult<Uint128> {
    // Simplified calculation - actual implementation would be more complex
    let user_reward = USER_REWARDS
        .may_load(deps.storage, (user, pool_id))?
        .unwrap_or(UserReward {
            user: user.clone(),
            pool_id,
            reward_per_token_paid: Decimal::zero(),
            rewards_accrued: Uint128::zero(),
            last_claim_time: 0,
        });

    Ok(user_reward.rewards_accrued)
}

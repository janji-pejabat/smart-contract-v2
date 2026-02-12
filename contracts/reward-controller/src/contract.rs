use cosmwasm_std::{
    entry_point, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, MessageInfo, Response, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockerHookMsg, PendingRewardsResponse,
    QueryMsg, ReferrerBalanceInfo, ReferrerBalancesResponse, RewardPoolResponse, UserStakeResponse,
};
use crate::state::{
    AssetInfo, DynamicAPRConfig, RewardConfig, RewardPool, UserReward, UserStake, CONFIG, LP_POOLS,
    POOLS, REFERRALS, REFERRER_BALANCES, TOTAL_STAKED, USER_REWARDS, USER_STAKED_LOCKERS,
    USER_STAKES,
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
        referral_commission_bps: 500, // 5% default
        batch_limit: 20,
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
        ExecuteMsg::ClaimRewards {
            locker_id,
            pool_ids,
        } => execute_claim_rewards(deps, env, info, locker_id, pool_ids),
        ExecuteMsg::BatchClaimRewards {
            locker_ids,
            pool_ids,
        } => execute_batch_claim_rewards(deps, env, info, locker_ids, pool_ids),
        ExecuteMsg::RegisterReferral { referrer } => {
            execute_register_referral(deps, info, referrer)
        }
        ExecuteMsg::ClaimReferralRewards {} => execute_claim_referral_rewards(deps, env, info),
        ExecuteMsg::CreateRewardPool {
            lp_token,
            reward_token,
            apr,
            dynamic_config,
        } => execute_create_pool(deps, env, info, lp_token, reward_token, apr, dynamic_config),
        ExecuteMsg::UpdateRewardPool {
            pool_id,
            apr,
            dynamic_config,
            enabled,
        } => execute_update_pool(deps, env, info, pool_id, apr, dynamic_config, enabled),
        ExecuteMsg::DepositRewards { pool_id } => execute_deposit_rewards(deps, env, info, pool_id),
        ExecuteMsg::WithdrawRewards { pool_id, amount } => {
            execute_withdraw_rewards(deps, env, info, pool_id, amount)
        }
        ExecuteMsg::UpdateConfig {
            admin,
            lp_locker_contract,
            claim_interval,
            referral_commission_bps,
            batch_limit,
        } => execute_update_config(
            deps,
            info,
            admin,
            lp_locker_contract,
            claim_interval,
            referral_commission_bps,
            batch_limit,
        ),
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
            pool.total_deposited = pool
                .total_deposited
                .checked_add(wrapper.amount)
                .map_err(cosmwasm_std::StdError::from)?;
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
        LockerHookMsg::OnLock {
            locker_id,
            owner,
            lp_token,
            amount,
            locked_at,
            unlock_time,
        } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let lp_token_addr = deps.api.addr_validate(&lp_token)?;
            let duration = unlock_time.saturating_sub(locked_at);
            let bonus_multiplier = calculate_bonus_multiplier(duration);

            let stake = UserStake {
                user: owner_addr.clone(),
                locker_id,
                lp_token: lp_token_addr.clone(),
                lp_amount: amount,
                locked_at,
                bonus_multiplier,
            };
            USER_STAKES.save(deps.storage, locker_id, &stake)?;
            USER_STAKED_LOCKERS.save(deps.storage, (&owner_addr, locker_id), &true)?;

            // Find all pools for this LP token and initialize user reward records
            let pool_ids = get_pools_for_lp(deps.as_ref(), &lp_token_addr)?;
            for pool_id in pool_ids {
                let pool = update_pool(deps.storage, &env, pool_id)?;
                USER_REWARDS.save(
                    deps.storage,
                    (locker_id, pool_id),
                    &UserReward {
                        locker_id,
                        pool_id,
                        reward_per_token_paid: pool.reward_per_token_stored,
                        rewards_accrued: Uint128::zero(),
                        last_claim_time: env.block.time.seconds(),
                    },
                )?;
            }

            TOTAL_STAKED.update(deps.storage, &lp_token_addr, |total| -> StdResult<_> {
                total
                    .unwrap_or_default()
                    .checked_add(amount)
                    .map_err(cosmwasm_std::StdError::from)
            })?;
        }
        LockerHookMsg::OnExtend {
            locker_id,
            new_unlock_time,
        } => {
            let mut stake = USER_STAKES.load(deps.storage, locker_id)?;
            let total_duration = new_unlock_time.saturating_sub(stake.locked_at);
            let new_multiplier = calculate_bonus_multiplier(total_duration);

            // Update all user reward records before changing multiplier
            let pool_ids = get_pools_for_lp(deps.as_ref(), &stake.lp_token)?;
            for pool_id in pool_ids {
                update_user_reward(deps.storage, &env, locker_id, pool_id)?;
            }

            stake.bonus_multiplier = new_multiplier;
            USER_STAKES.save(deps.storage, locker_id, &stake)?;
        }
        LockerHookMsg::OnUnlock {
            locker_id,
            owner: _,
        } => {
            let stake = USER_STAKES.load(deps.storage, locker_id)?;

            // Final update of rewards before removing stake
            let pool_ids = get_pools_for_lp(deps.as_ref(), &stake.lp_token)?;
            let mut messages: Vec<CosmosMsg> = vec![];
            let referrer = REFERRALS.may_load(deps.storage, &stake.user)?;

            for pool_id in pool_ids {
                update_user_reward(deps.storage, &env, locker_id, pool_id)?;

                let mut user_reward = USER_REWARDS.load(deps.storage, (locker_id, pool_id))?;
                let amount = user_reward.rewards_accrued;

                if !amount.is_zero() {
                    let mut pool = POOLS.load(deps.storage, pool_id)?;
                    let available = pool.total_deposited.saturating_sub(pool.total_claimed);

                    if amount <= available {
                        user_reward.rewards_accrued = Uint128::zero();
                        user_reward.last_claim_time = env.block.time.seconds();
                        USER_REWARDS.save(deps.storage, (locker_id, pool_id), &user_reward)?;

                        pool.total_claimed = pool
                            .total_claimed
                            .checked_add(amount)
                            .map_err(cosmwasm_std::StdError::from)?;
                        POOLS.save(deps.storage, pool_id, &pool)?;

                        // Handle referral commission
                        let mut user_amount = amount;
                        if let Some(ref_addr) = &referrer {
                            let commission =
                                amount.multiply_ratio(config.referral_commission_bps, 10000u128);
                            if !commission.is_zero() {
                                user_amount = amount
                                    .checked_sub(commission)
                                    .map_err(cosmwasm_std::StdError::from)?;
                                REFERRER_BALANCES.update(
                                    deps.storage,
                                    (ref_addr, pool.reward_token.to_key()),
                                    |bal: Option<Uint128>| -> StdResult<_> {
                                        Ok(bal.unwrap_or_default().checked_add(commission)?)
                                    },
                                )?;
                            }
                        }

                        let transfer_msg = match &pool.reward_token {
                            AssetInfo::Cw20(addr) => CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: addr.to_string(),
                                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                                    recipient: stake.user.to_string(),
                                    amount: user_amount,
                                })?,
                                funds: vec![],
                            }),
                            AssetInfo::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                                to_address: stake.user.to_string(),
                                amount: vec![cosmwasm_std::Coin {
                                    denom: denom.clone(),
                                    amount: user_amount,
                                }],
                            }),
                        };
                        messages.push(transfer_msg);
                    }
                }
            }

            USER_STAKED_LOCKERS.remove(deps.storage, (&stake.user, locker_id));
            USER_STAKES.remove(deps.storage, locker_id);
            TOTAL_STAKED.update(deps.storage, &stake.lp_token, |total| -> StdResult<_> {
                total
                    .unwrap_or_default()
                    .checked_sub(stake.lp_amount)
                    .map_err(cosmwasm_std::StdError::from)
            })?;

            return Ok(Response::new()
                .add_messages(messages)
                .add_attribute("action", "locker_hook")
                .add_attribute("sub_action", "on_unlock_auto_claim"));
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
    Ok(LP_POOLS
        .may_load(deps.storage, lp_token)?
        .unwrap_or_default())
}

fn update_pool(
    storage: &mut dyn Storage,
    env: &Env,
    pool_id: u64,
) -> Result<RewardPool, ContractError> {
    let mut pool = POOLS.load(storage, pool_id)?;

    // Handle dynamic APR adjustment if configured
    if let Some(config) = &pool.dynamic_config {
        let tvl = TOTAL_STAKED
            .may_load(storage, &pool.lp_token)?
            .unwrap_or_default();
        let mut adjusted_apr = config.base_apr;

        if tvl < config.tvl_threshold_low {
            // TVL rendah -> APR meningkat (boost by adjustment_factor)
            adjusted_apr = config
                .base_apr
                .checked_add(config.adjustment_factor)
                .map_err(cosmwasm_std::StdError::from)?;
        } else if tvl > config.tvl_threshold_high {
            // TVL tinggi -> APR menurun (reduce by adjustment_factor)
            adjusted_apr = config.base_apr.saturating_sub(config.adjustment_factor);
        }
        pool.apr = adjusted_apr;
    }

    if pool.last_update >= env.block.time.seconds() {
        POOLS.save(storage, pool_id, &pool)?;
        return Ok(pool);
    }

    let time_elapsed = env.block.time.seconds().saturating_sub(pool.last_update);
    if time_elapsed > 0 && pool.enabled {
        let year_seconds = 365 * 24 * 3600u128;
        let apr_increase = pool
            .apr
            .checked_mul(Decimal::from_ratio(time_elapsed as u128, year_seconds))
            .map_err(cosmwasm_std::StdError::from)?;
        pool.reward_per_token_stored = pool
            .reward_per_token_stored
            .checked_add(apr_increase)
            .map_err(cosmwasm_std::StdError::from)?;
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
        user_reward.rewards_accrued = user_reward
            .rewards_accrued
            .checked_add(pending)
            .map_err(cosmwasm_std::StdError::from)?;
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
    let reward_per_token_diff = pool
        .reward_per_token_stored
        .checked_sub(user_reward.reward_per_token_paid)?;
    if reward_per_token_diff.is_zero() {
        return Ok(Uint128::zero());
    }

    let effective_amount = stake.lp_amount.mul_floor(stake.bonus_multiplier);
    let pending = effective_amount.mul_floor(reward_per_token_diff);
    Ok(pending)
}

fn execute_batch_claim_rewards(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_ids: Vec<u64>,
    pool_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if locker_ids.len() > config.batch_limit as usize {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Batch limit exceeded",
        )));
    }

    let mut response = Response::new().add_attribute("action", "batch_claim_rewards");
    for locker_id in locker_ids {
        let res = execute_claim_rewards(
            deps.branch(),
            env.clone(),
            info.clone(),
            locker_id,
            pool_ids.clone(),
        )?;
        for msg in res.messages {
            response = response.add_submessage(msg);
        }
        response = response.add_attributes(res.attributes);
    }
    Ok(response)
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

    let referrer = REFERRALS.may_load(deps.storage, &info.sender)?;

    for pool_id in pool_ids {
        update_user_reward(deps.storage, &env, locker_id, pool_id)?;

        let mut user_reward = USER_REWARDS.load(deps.storage, (locker_id, pool_id))?;
        let mut pool = POOLS.load(deps.storage, pool_id)?;

        let amount = user_reward.rewards_accrued;
        if amount.is_zero() {
            continue;
        }

        if env
            .block
            .time
            .seconds()
            .saturating_sub(user_reward.last_claim_time)
            < config.claim_interval
        {
            return Err(ContractError::ClaimTooSoon {});
        }

        // Check if pool has enough rewards
        let available = pool.total_deposited.saturating_sub(pool.total_claimed);
        if amount > available {
            return Err(ContractError::InsufficientRewards {});
        }

        user_reward.rewards_accrued = Uint128::zero();
        user_reward.last_claim_time = env.block.time.seconds();
        USER_REWARDS.save(deps.storage, (locker_id, pool_id), &user_reward)?;

        pool.total_claimed = pool
            .total_claimed
            .checked_add(amount)
            .map_err(cosmwasm_std::StdError::from)?;
        POOLS.save(deps.storage, pool_id, &pool)?;

        // Handle referral commission
        let mut user_amount = amount;
        if let Some(ref_addr) = &referrer {
            let commission = amount.multiply_ratio(config.referral_commission_bps, 10000u128);
            if !commission.is_zero() {
                user_amount = amount
                    .checked_sub(commission)
                    .map_err(cosmwasm_std::StdError::from)?;
                REFERRER_BALANCES.update(
                    deps.storage,
                    (ref_addr, pool.reward_token.to_key()),
                    |bal: Option<Uint128>| -> StdResult<_> {
                        Ok(bal.unwrap_or_default().checked_add(commission)?)
                    },
                )?;
            }
        }

        let transfer_msg = match &pool.reward_token {
            AssetInfo::Cw20(addr) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: user_amount,
                })?,
                funds: vec![],
            }),
            AssetInfo::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: user_amount,
                }],
            }),
        };

        messages.push(transfer_msg);
        total_claimed = total_claimed
            .checked_add(user_amount)
            .map_err(cosmwasm_std::StdError::from)?;
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
    dynamic_config: Option<DynamicAPRConfig>,
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
        lp_token: lp_token_addr.clone(),
        reward_token,
        total_deposited: Uint128::zero(),
        total_claimed: Uint128::zero(),
        apr,
        dynamic_config,
        last_update: env.block.time.seconds(),
        reward_per_token_stored: Decimal::zero(),
        enabled: true,
    };

    POOLS.save(deps.storage, pool_id, &pool)?;

    // Update LP_POOLS index
    LP_POOLS.update(deps.storage, &lp_token_addr, |pools| -> StdResult<_> {
        let mut pools = pools.unwrap_or_default();
        pools.push(pool_id);
        Ok(pools)
    })?;

    Ok(Response::new()
        .add_attribute("action", "create_reward_pool")
        .add_attribute("pool_id", pool_id.to_string()))
}

fn execute_register_referral(
    deps: DepsMut,
    info: MessageInfo,
    referrer: String,
) -> Result<Response, ContractError> {
    let referee = info.sender;
    let referrer_addr = deps.api.addr_validate(&referrer)?;

    if referee == referrer_addr {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Cannot refer yourself",
        )));
    }

    if REFERRALS.has(deps.storage, &referee) {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Referral already registered",
        )));
    }

    REFERRALS.save(deps.storage, &referee, &referrer_addr)?;

    Ok(Response::new()
        .add_attribute("action", "register_referral")
        .add_attribute("referrer", referrer)
        .add_attribute("referee", referee))
}

fn execute_claim_referral_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let referrer = info.sender;
    let mut messages: Vec<CosmosMsg> = vec![];

    let balances: Vec<(String, Uint128)> = REFERRER_BALANCES
        .prefix(&referrer)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item: StdResult<(String, Uint128)>| item.ok())
        .collect();

    for (asset_key, amount) in balances {
        if amount.is_zero() {
            continue;
        }

        REFERRER_BALANCES.remove(deps.storage, (&referrer, asset_key.clone()));
        let asset = AssetInfo::from_key(asset_key);

        let transfer_msg = match asset {
            AssetInfo::Cw20(addr) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: referrer.to_string(),
                    amount,
                })?,
                funds: vec![],
            }),
            AssetInfo::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                to_address: referrer.to_string(),
                amount: vec![Coin { denom, amount }],
            }),
        };
        messages.push(transfer_msg);
    }

    if messages.is_empty() {
        return Err(ContractError::NoRewards {});
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim_referral_rewards")
        .add_attribute("referrer", referrer))
}

fn execute_update_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_id: u64,
    apr: Option<Decimal>,
    dynamic_config: Option<Option<DynamicAPRConfig>>,
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

    if let Some(new_config) = dynamic_config {
        pool.dynamic_config = new_config;
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

    pool.total_deposited = pool
        .total_deposited
        .checked_add(deposit_amount)
        .map_err(cosmwasm_std::StdError::from)?;
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

    let available = pool
        .total_deposited
        .checked_sub(pool.total_claimed)
        .map_err(cosmwasm_std::StdError::from)?;
    if amount > available {
        return Err(ContractError::InsufficientRewards {});
    }

    pool.total_deposited = pool
        .total_deposited
        .checked_sub(amount)
        .map_err(cosmwasm_std::StdError::from)?;
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
    referral_commission_bps: Option<u16>,
    batch_limit: Option<u32>,
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

    if let Some(bps) = referral_commission_bps {
        if bps > 10000 {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "Invalid commission bps",
            )));
        }
        config.referral_commission_bps = bps;
    }

    if let Some(limit) = batch_limit {
        config.batch_limit = limit;
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
        QueryMsg::ReferrerBalances {
            referrer,
            start_after,
            limit,
        } => to_json_binary(&query_referrer_balances(
            deps,
            referrer,
            start_after,
            limit,
        )?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    _deps: DepsMut,
    _env: Env,
    _msg: crate::msg::MigrateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("action", "migrate"))
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin,
        lp_locker_contract: config.lp_locker_contract,
        paused: config.paused,
        claim_interval: config.claim_interval,
        next_pool_id: config.next_pool_id,
        referral_commission_bps: config.referral_commission_bps,
        batch_limit: config.batch_limit,
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
        dynamic_config: pool.dynamic_config,
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
                lp_token: pool.lp_token,
                reward_token: pool.reward_token,
                total_deposited: pool.total_deposited,
                total_claimed: pool.total_claimed,
                apr: pool.apr,
                dynamic_config: pool.dynamic_config,
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
            total_pending = total_pending
                .checked_add(pending)
                .map_err(cosmwasm_std::StdError::from)?;
        }
    }

    Ok(PendingRewardsResponse {
        pool_id,
        pending_amount: total_pending,
    })
}

fn query_referrer_balances(
    deps: Deps,
    referrer: String,
    start_after: Option<AssetInfo>,
    limit: Option<u32>,
) -> StdResult<ReferrerBalancesResponse> {
    let referrer_addr = deps.api.addr_validate(&referrer)?;
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(|a| cw_storage_plus::Bound::exclusive(a.to_key()));

    let balances: Vec<ReferrerBalanceInfo> = REFERRER_BALANCES
        .prefix(&referrer_addr)
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item: StdResult<(String, Uint128)>| {
            let (asset_key, amount) = item?;
            Ok(ReferrerBalanceInfo {
                asset: AssetInfo::from_key(asset_key),
                amount,
            })
        })
        .collect::<StdResult<_>>()?;

    Ok(ReferrerBalancesResponse {
        referrer: referrer_addr,
        balances,
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
        let apr_increase = pool
            .apr
            .checked_mul(Decimal::from_ratio(time_elapsed as u128, year_seconds))
            .map_err(cosmwasm_std::StdError::from)?;
        pool.reward_per_token_stored = pool
            .reward_per_token_stored
            .checked_add(apr_increase)
            .map_err(cosmwasm_std::StdError::from)?;
    }

    let pending = calculate_pending_for_user(&stake, &pool, &user_reward)?;
    user_reward
        .rewards_accrued
        .checked_add(pending)
        .map_err(cosmwasm_std::StdError::from)
}

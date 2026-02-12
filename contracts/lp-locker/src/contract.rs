use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, LockerHookMsg, LockerResponse,
    LockersResponse, MigrateMsg, QueryMsg, TotalLockedResponse, WhitelistedLPResponse,
};
use crate::state::{
    Config, Locker, WhitelistedLP, CONFIG, LOCKERS, TOTAL_LOCKED, USER_LOCKERS, USER_LP_HISTORY,
    WHITELISTED_LPS,
};

const CONTRACT_NAME: &str = "crates.io:lp-locker";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_PLATFORM_FEE_BPS: u16 = 500; // 5% maximum

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;

    let config = Config {
        admin,
        reward_controller: None,
        emergency_unlock_delay: msg.emergency_unlock_delay,
        platform_fee_bps: 0,
        batch_limit: 20,
        paused: false,
        next_locker_id: 0,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute(
            "emergency_unlock_delay",
            msg.emergency_unlock_delay.to_string(),
        ))
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
        ExecuteMsg::UnlockLP { locker_id } => execute_unlock_lp(deps, env, info, locker_id),
        ExecuteMsg::BatchUnlock { locker_ids } => execute_batch_unlock(deps, env, info, locker_ids),
        ExecuteMsg::ExtendLock {
            locker_id,
            new_unlock_time,
        } => execute_extend_lock(deps, env, info, locker_id, new_unlock_time),
        ExecuteMsg::BatchExtendLock { locks } => execute_batch_extend_lock(deps, env, info, locks),
        ExecuteMsg::RequestEmergencyUnlock { locker_id } => {
            execute_request_emergency_unlock(deps, env, info, locker_id)
        }
        ExecuteMsg::ExecuteEmergencyUnlock { locker_id } => {
            execute_emergency_unlock(deps, env, info, locker_id)
        }
        ExecuteMsg::UpdateConfig {
            admin,
            reward_controller,
            emergency_unlock_delay,
            platform_fee_bps,
            batch_limit,
        } => execute_update_config(
            deps,
            info,
            admin,
            reward_controller,
            emergency_unlock_delay,
            platform_fee_bps,
            batch_limit,
        ),
        ExecuteMsg::WhitelistLP {
            lp_token,
            name,
            symbol,
            min_lock_duration,
            max_lock_duration,
            bonus_multiplier,
        } => execute_whitelist_lp(
            deps,
            info,
            lp_token,
            name,
            symbol,
            min_lock_duration,
            max_lock_duration,
            bonus_multiplier,
        ),
        ExecuteMsg::RemoveLP { lp_token } => execute_remove_lp(deps, info, lp_token),
        ExecuteMsg::Pause {} => execute_pause(deps, info),
        ExecuteMsg::Resume {} => execute_resume(deps, info),
    }
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.paused {
        return Err(ContractError::Paused {});
    }

    let lp_token = info.sender;
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    let amount = wrapper.amount;

    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let msg: Cw20HookMsg = from_json(&wrapper.msg)?;

    match msg {
        Cw20HookMsg::LockLP {
            unlock_time,
            metadata,
        } => execute_lock_lp(deps, env, sender, lp_token, amount, unlock_time, metadata),
    }
}

fn execute_lock_lp(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    lp_token: Addr,
    amount: Uint128,
    unlock_time: u64,
    metadata: Option<String>,
) -> Result<Response, ContractError> {
    let mut whitelist = WHITELISTED_LPS
        .may_load(deps.storage, &lp_token)?
        .ok_or(ContractError::LPNotWhitelisted {})?;

    if !whitelist.enabled {
        return Err(ContractError::LPNotWhitelisted {});
    }

    let current_time = env.block.time.seconds();
    let lock_duration =
        unlock_time
            .checked_sub(current_time)
            .ok_or(ContractError::InvalidUnlockTime {
                min: whitelist.min_lock_duration,
                max: whitelist.max_lock_duration,
            })?;

    if lock_duration < whitelist.min_lock_duration || lock_duration > whitelist.max_lock_duration {
        return Err(ContractError::InvalidUnlockTime {
            min: whitelist.min_lock_duration,
            max: whitelist.max_lock_duration,
        });
    }

    let mut config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<WasmMsg> = vec![];
    let mut lock_amount = amount;
    if config.platform_fee_bps > 0 {
        let fee_amount = amount.multiply_ratio(config.platform_fee_bps, 10000u128);
        if !fee_amount.is_zero() {
            lock_amount = amount
                .checked_sub(fee_amount)
                .map_err(cosmwasm_std::StdError::from)?;
            messages.push(WasmMsg::Execute {
                contract_addr: lp_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: config.admin.to_string(),
                    amount: fee_amount,
                })?,
                funds: vec![],
            });
        }
    }

    let locker_id = config.next_locker_id;
    config.next_locker_id += 1;
    CONFIG.save(deps.storage, &config)?;

    let locker = Locker {
        id: locker_id,
        owner: sender.clone(),
        lp_token: lp_token.clone(),
        amount: lock_amount,
        locked_at: current_time,
        unlock_time,
        extended_count: 0,
        emergency_unlock_requested: None,
        metadata,
    };

    LOCKERS.save(deps.storage, locker_id, &locker)?;
    USER_LOCKERS.save(deps.storage, (&sender, locker_id), &true)?;

    TOTAL_LOCKED.update(deps.storage, &lp_token, |total| -> StdResult<_> {
        Ok(total.unwrap_or_default().checked_add(lock_amount)?)
    })?;

    whitelist.total_locked_all_time = whitelist
        .total_locked_all_time
        .checked_add(lock_amount)
        .map_err(cosmwasm_std::StdError::from)?;
    if !USER_LP_HISTORY.has(deps.storage, (&sender, &lp_token)) {
        USER_LP_HISTORY.save(deps.storage, (&sender, &lp_token), &true)?;
        whitelist.user_count += 1;
    }
    WHITELISTED_LPS.save(deps.storage, &lp_token, &whitelist)?;

    if let Some(reward_controller) = config.reward_controller {
        messages.push(WasmMsg::Execute {
            contract_addr: reward_controller.to_string(),
            msg: to_json_binary(&LockerHookMsg::OnLock {
                locker_id,
                owner: sender.to_string(),
                lp_token: lp_token.to_string(),
                amount: lock_amount,
                locked_at: current_time,
                unlock_time,
            })?,
            funds: vec![],
        });
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "lock_lp")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("owner", sender)
        .add_attribute("lp_token", lp_token)
        .add_attribute("amount", lock_amount)
        .add_attribute("unlock_time", unlock_time.to_string()))
}

fn execute_batch_unlock(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_ids: Vec<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if locker_ids.len() > config.batch_limit as usize {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Batch limit exceeded",
        )));
    }

    let mut response = Response::new().add_attribute("action", "batch_unlock");
    for locker_id in locker_ids {
        let res = execute_unlock_lp(deps.branch(), env.clone(), info.clone(), locker_id)?;
        response = response.add_submessages(res.messages);
        response = response.add_attributes(res.attributes);
    }
    Ok(response)
}

fn execute_unlock_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
) -> Result<Response, ContractError> {
    let locker = LOCKERS.load(deps.storage, locker_id)?;
    let config = CONFIG.load(deps.storage)?;

    if locker.owner != info.sender {
        return Err(ContractError::NotOwner {});
    }

    let current_time = env.block.time.seconds();
    if current_time < locker.unlock_time {
        return Err(ContractError::StillLocked(locker.unlock_time));
    }

    LOCKERS.remove(deps.storage, locker_id);
    USER_LOCKERS.remove(deps.storage, (&locker.owner, locker_id));

    TOTAL_LOCKED.update(deps.storage, &locker.lp_token, |total| -> StdResult<_> {
        Ok(total.unwrap_or_default().checked_sub(locker.amount)?)
    })?;

    WHITELISTED_LPS.update(deps.storage, &locker.lp_token, |wl| -> StdResult<_> {
        let mut wl = wl.ok_or(cosmwasm_std::StdError::generic_err("Whitelist not found"))?;
        wl.total_unlocked_all_time = wl.total_unlocked_all_time.checked_add(locker.amount)?;
        Ok(wl)
    })?;

    let mut messages: Vec<WasmMsg> = vec![];
    let mut return_amount = locker.amount;

    if config.platform_fee_bps > 0 {
        let fee_amount = locker
            .amount
            .multiply_ratio(config.platform_fee_bps, 10000u128);
        if !fee_amount.is_zero() {
            return_amount = locker
                .amount
                .checked_sub(fee_amount)
                .map_err(cosmwasm_std::StdError::from)?;
            messages.push(WasmMsg::Execute {
                contract_addr: locker.lp_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: config.admin.to_string(),
                    amount: fee_amount,
                })?,
                funds: vec![],
            });
        }
    }

    messages.push(WasmMsg::Execute {
        contract_addr: locker.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: locker.owner.to_string(),
            amount: return_amount,
        })?,
        funds: vec![],
    });

    if let Some(reward_controller) = config.reward_controller {
        messages.push(WasmMsg::Execute {
            contract_addr: reward_controller.to_string(),
            msg: to_json_binary(&LockerHookMsg::OnUnlock {
                locker_id,
                owner: locker.owner.to_string(),
            })?,
            funds: vec![],
        });
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "unlock_lp")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("owner", locker.owner)
        .add_attribute("amount", return_amount))
}

fn execute_batch_extend_lock(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locks: Vec<(u64, u64)>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if locks.len() > config.batch_limit as usize {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Batch limit exceeded",
        )));
    }

    let mut response = Response::new().add_attribute("action", "batch_extend_lock");
    for (locker_id, new_unlock_time) in locks {
        let res = execute_extend_lock(
            deps.branch(),
            env.clone(),
            info.clone(),
            locker_id,
            new_unlock_time,
        )?;
        response = response.add_submessages(res.messages);
        response = response.add_attributes(res.attributes);
    }
    Ok(response)
}

fn execute_extend_lock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
    new_unlock_time: u64,
) -> Result<Response, ContractError> {
    let mut locker = LOCKERS.load(deps.storage, locker_id)?;

    if locker.owner != info.sender {
        return Err(ContractError::NotOwner {});
    }

    if new_unlock_time <= locker.unlock_time {
        return Err(ContractError::InvalidExtension {});
    }

    let whitelist = WHITELISTED_LPS.load(deps.storage, &locker.lp_token)?;
    let current_time = env.block.time.seconds();
    let new_duration = new_unlock_time.saturating_sub(current_time);

    if new_duration > whitelist.max_lock_duration {
        return Err(ContractError::InvalidUnlockTime {
            min: whitelist.min_lock_duration,
            max: whitelist.max_lock_duration,
        });
    }

    let old_unlock_time = locker.unlock_time;
    locker.unlock_time = new_unlock_time;
    locker.extended_count += 1;

    LOCKERS.save(deps.storage, locker_id, &locker)?;

    let mut response = Response::new()
        .add_attribute("action", "extend_lock")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("old_unlock_time", old_unlock_time.to_string())
        .add_attribute("new_unlock_time", new_unlock_time.to_string());

    let config = CONFIG.load(deps.storage)?;
    if let Some(reward_controller) = config.reward_controller {
        response = response.add_message(WasmMsg::Execute {
            contract_addr: reward_controller.to_string(),
            msg: to_json_binary(&LockerHookMsg::OnExtend {
                locker_id,
                new_unlock_time,
            })?,
            funds: vec![],
        });
    }

    Ok(response)
}

fn execute_request_emergency_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
) -> Result<Response, ContractError> {
    let mut locker = LOCKERS.load(deps.storage, locker_id)?;

    if locker.owner != info.sender {
        return Err(ContractError::NotOwner {});
    }

    let config = CONFIG.load(deps.storage)?;
    let execute_at = env.block.time.seconds() + config.emergency_unlock_delay;

    locker.emergency_unlock_requested = Some(execute_at);
    LOCKERS.save(deps.storage, locker_id, &locker)?;

    Ok(Response::new()
        .add_attribute("action", "request_emergency_unlock")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("execute_at", execute_at.to_string()))
}

fn execute_emergency_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
) -> Result<Response, ContractError> {
    let locker = LOCKERS.load(deps.storage, locker_id)?;

    if locker.owner != info.sender {
        return Err(ContractError::NotOwner {});
    }

    let execute_at = locker
        .emergency_unlock_requested
        .ok_or(ContractError::EmergencyNotRequested {})?;

    if env.block.time.seconds() < execute_at {
        return Err(ContractError::EmergencyDelayNotPassed(execute_at));
    }

    LOCKERS.remove(deps.storage, locker_id);
    USER_LOCKERS.remove(deps.storage, (&locker.owner, locker_id));

    TOTAL_LOCKED.update(deps.storage, &locker.lp_token, |total| -> StdResult<_> {
        Ok(total.unwrap_or_default().checked_sub(locker.amount)?)
    })?;

    let config = CONFIG.load(deps.storage)?;
    let mut messages: Vec<WasmMsg> = vec![];
    let mut return_amount = locker.amount;

    if config.platform_fee_bps > 0 {
        let fee_amount = locker
            .amount
            .multiply_ratio(config.platform_fee_bps, 10000u128);
        if !fee_amount.is_zero() {
            return_amount = locker
                .amount
                .checked_sub(fee_amount)
                .map_err(cosmwasm_std::StdError::from)?;
            messages.push(WasmMsg::Execute {
                contract_addr: locker.lp_token.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: config.admin.to_string(),
                    amount: fee_amount,
                })?,
                funds: vec![],
            });
        }
    }

    messages.push(WasmMsg::Execute {
        contract_addr: locker.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: locker.owner.to_string(),
            amount: return_amount,
        })?,
        funds: vec![],
    });

    if let Some(reward_controller) = config.reward_controller {
        messages.push(WasmMsg::Execute {
            contract_addr: reward_controller.to_string(),
            msg: to_json_binary(&LockerHookMsg::OnUnlock {
                locker_id,
                owner: locker.owner.to_string(),
            })?,
            funds: vec![],
        });
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "emergency_unlock")
        .add_attribute("locker_id", locker_id.to_string()))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    reward_controller: Option<String>,
    emergency_unlock_delay: Option<u64>,
    platform_fee_bps: Option<u16>,
    batch_limit: Option<u32>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(admin_addr) = admin {
        config.admin = deps.api.addr_validate(&admin_addr)?;
    }

    if let Some(reward_addr) = reward_controller {
        config.reward_controller = Some(deps.api.addr_validate(&reward_addr)?);
    }

    if let Some(delay) = emergency_unlock_delay {
        config.emergency_unlock_delay = delay;
    }

    if let Some(fee) = platform_fee_bps {
        if fee > MAX_PLATFORM_FEE_BPS {
            return Err(ContractError::FeeTooHigh(MAX_PLATFORM_FEE_BPS));
        }
        config.platform_fee_bps = fee;
    }

    if let Some(limit) = batch_limit {
        config.batch_limit = limit;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[allow(clippy::too_many_arguments)]
fn execute_whitelist_lp(
    deps: DepsMut,
    info: MessageInfo,
    lp_token: String,
    name: String,
    symbol: String,
    min_lock_duration: u64,
    max_lock_duration: u64,
    bonus_multiplier: Decimal,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let lp_addr = deps.api.addr_validate(&lp_token)?;

    let whitelist = WhitelistedLP {
        lp_token: lp_addr.clone(),
        name,
        symbol,
        min_lock_duration,
        max_lock_duration,
        enabled: true,
        bonus_multiplier,
        total_locked_all_time: Uint128::zero(),
        total_unlocked_all_time: Uint128::zero(),
        user_count: 0,
    };

    WHITELISTED_LPS.save(deps.storage, &lp_addr, &whitelist)?;

    Ok(Response::new()
        .add_attribute("action", "whitelist_lp")
        .add_attribute("lp_token", lp_token))
}

fn execute_remove_lp(
    deps: DepsMut,
    info: MessageInfo,
    lp_token: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let lp_addr = deps.api.addr_validate(&lp_token)?;
    WHITELISTED_LPS.remove(deps.storage, &lp_addr);

    Ok(Response::new()
        .add_attribute("action", "remove_lp")
        .add_attribute("lp_token", lp_token))
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
        QueryMsg::Locker { locker_id } => to_json_binary(&query_locker(deps, locker_id)?),
        QueryMsg::LockersByOwner {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_lockers_by_owner(deps, owner, start_after, limit)?),
        QueryMsg::WhitelistedLP { lp_token } => {
            to_json_binary(&query_whitelisted_lp(deps, lp_token)?)
        }
        QueryMsg::AllWhitelistedLPs { start_after, limit } => {
            to_json_binary(&query_all_whitelisted_lps(deps, start_after, limit)?)
        }
        QueryMsg::TotalLockedByLP { lp_token } => {
            to_json_binary(&query_total_locked(deps, lp_token)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin,
        reward_controller: config.reward_controller,
        emergency_unlock_delay: config.emergency_unlock_delay,
        platform_fee_bps: config.platform_fee_bps,
        batch_limit: config.batch_limit,
        paused: config.paused,
        next_locker_id: config.next_locker_id,
    })
}

fn query_locker(deps: Deps, locker_id: u64) -> StdResult<LockerResponse> {
    let locker = LOCKERS.load(deps.storage, locker_id)?;
    Ok(LockerResponse {
        id: locker.id,
        owner: locker.owner,
        lp_token: locker.lp_token,
        amount: locker.amount,
        locked_at: locker.locked_at,
        unlock_time: locker.unlock_time,
        extended_count: locker.extended_count,
        emergency_unlock_requested: locker.emergency_unlock_requested,
        metadata: locker.metadata,
    })
}

fn query_lockers_by_owner(
    deps: Deps,
    owner: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<LockersResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(Bound::exclusive);

    let lockers: Vec<LockerResponse> = USER_LOCKERS
        .prefix(&owner_addr)
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            item.ok().and_then(|(locker_id, _)| {
                LOCKERS
                    .load(deps.storage, locker_id)
                    .ok()
                    .map(|locker| LockerResponse {
                        id: locker.id,
                        owner: locker.owner,
                        lp_token: locker.lp_token,
                        amount: locker.amount,
                        locked_at: locker.locked_at,
                        unlock_time: locker.unlock_time,
                        extended_count: locker.extended_count,
                        emergency_unlock_requested: locker.emergency_unlock_requested,
                        metadata: locker.metadata,
                    })
            })
        })
        .collect();

    Ok(LockersResponse { lockers })
}

fn query_whitelisted_lp(deps: Deps, lp_token: String) -> StdResult<WhitelistedLPResponse> {
    let lp_addr = deps.api.addr_validate(&lp_token)?;
    let whitelist = WHITELISTED_LPS.load(deps.storage, &lp_addr)?;

    Ok(WhitelistedLPResponse {
        lp_token: whitelist.lp_token,
        name: whitelist.name,
        symbol: whitelist.symbol,
        min_lock_duration: whitelist.min_lock_duration,
        max_lock_duration: whitelist.max_lock_duration,
        enabled: whitelist.enabled,
        bonus_multiplier: whitelist.bonus_multiplier,
        total_locked_all_time: whitelist.total_locked_all_time,
        total_unlocked_all_time: whitelist.total_unlocked_all_time,
        user_count: whitelist.user_count,
    })
}

fn query_all_whitelisted_lps(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<WhitelistedLPResponse>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start_addr = start_after
        .as_ref()
        .map(|s| deps.api.addr_validate(s))
        .transpose()?;
    let start = start_addr.as_ref().map(Bound::exclusive);

    WHITELISTED_LPS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, whitelist) = item?;
            Ok(WhitelistedLPResponse {
                lp_token: whitelist.lp_token,
                name: whitelist.name,
                symbol: whitelist.symbol,
                min_lock_duration: whitelist.min_lock_duration,
                max_lock_duration: whitelist.max_lock_duration,
                enabled: whitelist.enabled,
                bonus_multiplier: whitelist.bonus_multiplier,
                total_locked_all_time: whitelist.total_locked_all_time,
                total_unlocked_all_time: whitelist.total_unlocked_all_time,
                user_count: whitelist.user_count,
            })
        })
        .collect()
}

fn query_total_locked(deps: Deps, lp_token: String) -> StdResult<TotalLockedResponse> {
    let lp_addr = deps.api.addr_validate(&lp_token)?;
    let total = TOTAL_LOCKED
        .may_load(deps.storage, &lp_addr)?
        .unwrap_or_default();

    Ok(TotalLockedResponse {
        lp_token: lp_addr,
        total_amount: total,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = cw2::get_contract_version(deps.storage)?;

    if version.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidMigration {});
    }

    match msg {
        MigrateMsg::V1ToV2 { reward_controller } => {
            // 1. Migrate Config
            // We use a temporary struct to load the V1 config since V2 added `batch_limit`
            #[derive(serde::Deserialize, serde::Serialize)]
            pub struct ConfigV1 {
                pub admin: Addr,
                pub reward_controller: Option<Addr>,
                pub emergency_unlock_delay: u64,
                pub platform_fee_bps: u16,
                pub paused: bool,
                pub next_locker_id: u64,
            }

            let old_config_item: cw_storage_plus::Item<ConfigV1> =
                cw_storage_plus::Item::new("config");
            let old_config = old_config_item.load(deps.storage)?;
            let mut new_config = Config {
                admin: old_config.admin,
                reward_controller: old_config.reward_controller,
                emergency_unlock_delay: old_config.emergency_unlock_delay,
                platform_fee_bps: old_config.platform_fee_bps,
                batch_limit: 20, // Default value as per instantiate
                paused: old_config.paused,
                next_locker_id: old_config.next_locker_id,
            };

            if let Some(addr) = reward_controller {
                new_config.reward_controller = Some(deps.api.addr_validate(&addr)?);
            }
            CONFIG.save(deps.storage, &new_config)?;

            // 2. Migrate WhitelistedLPs
            // V1 WhitelistedLP didn't have name, symbol, total_locked_all_time, total_unlocked_all_time, user_count
            #[derive(serde::Deserialize, serde::Serialize)]
            pub struct WhitelistedLPV1 {
                pub lp_token: Addr,
                pub min_lock_duration: u64,
                pub max_lock_duration: u64,
                pub enabled: bool,
                pub bonus_multiplier: Decimal,
            }

            let old_lp_map: cw_storage_plus::Map<&Addr, WhitelistedLPV1> =
                cw_storage_plus::Map::new("whitelisted_lps");
            let entries: Vec<(Addr, WhitelistedLPV1)> = old_lp_map
                .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;

            for (lp_token, old_lp) in entries {
                // Query metadata from CW20 token
                let token_info: cw20::TokenInfoResponse = deps
                    .querier
                    .query_wasm_smart(&lp_token, &cw20::Cw20QueryMsg::TokenInfo {})
                    .unwrap_or(cw20::TokenInfoResponse {
                        name: "Unknown LP".to_string(),
                        symbol: "ULP".to_string(),
                        decimals: 6,
                        total_supply: Uint128::zero(),
                    });

                let new_lp = WhitelistedLP {
                    lp_token: old_lp.lp_token,
                    name: token_info.name,
                    symbol: token_info.symbol,
                    min_lock_duration: old_lp.min_lock_duration,
                    max_lock_duration: old_lp.max_lock_duration,
                    enabled: old_lp.enabled,
                    bonus_multiplier: old_lp.bonus_multiplier,
                    total_locked_all_time: TOTAL_LOCKED
                        .may_load(deps.storage, &lp_token)?
                        .unwrap_or_default(),
                    total_unlocked_all_time: Uint128::zero(),
                    user_count: 0, // Cannot easily backfill without scanning all lockers
                };
                WHITELISTED_LPS.save(deps.storage, &lp_token, &new_lp)?;
            }

            // 3. Migrate Lockers
            // V1 Locker didn't have metadata field
            #[derive(serde::Deserialize, serde::Serialize)]
            pub struct LockerV1 {
                pub id: u64,
                pub owner: Addr,
                pub lp_token: Addr,
                pub amount: Uint128,
                pub locked_at: u64,
                pub unlock_time: u64,
                pub extended_count: u8,
                pub emergency_unlock_requested: Option<u64>,
            }

            let locker_ids: Vec<u64> = LOCKERS
                .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?;

            let old_locker_map: cw_storage_plus::Map<u64, LockerV1> = cw_storage_plus::Map::new("lockers");
            for id in locker_ids {
                let old_locker = old_locker_map.load(deps.storage, id)?;
                let new_locker = Locker {
                    id: old_locker.id,
                    owner: old_locker.owner,
                    lp_token: old_locker.lp_token,
                    amount: old_locker.amount,
                    locked_at: old_locker.locked_at,
                    unlock_time: old_locker.unlock_time,
                    extended_count: old_locker.extended_count,
                    emergency_unlock_requested: old_locker.emergency_unlock_requested,
                    metadata: None,
                };
                LOCKERS.save(deps.storage, id, &new_locker)?;
            }

            cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

            Ok(Response::new()
                .add_attribute("action", "migrate")
                .add_attribute("from_version", version.version)
                .add_attribute("to_version", CONTRACT_VERSION))
        }
    }
}

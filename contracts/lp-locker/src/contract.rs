use cosmwasm_std::{
    entry_point, to_json_binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, WasmMsg, Addr, Decimal,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, LockerResponse, LockersResponse,
    QueryMsg, WhitelistedLPResponse, TotalLockedResponse, Cw20HookMsg, MigrateMsg,
};
use crate::state::{
    Config, Locker, WhitelistedLP, CONFIG, LOCKERS, USER_LOCKERS, WHITELISTED_LPS, TOTAL_LOCKED,
};

const CONTRACT_NAME: &str = "crates.io:lp-locker";
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

    let config = Config {
        admin,
        reward_controller: None,
        emergency_unlock_delay: msg.emergency_unlock_delay,
        platform_fee_bps: 0, // Can be updated later
        paused: false,
        next_locker_id: 0,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute("emergency_unlock_delay", msg.emergency_unlock_delay.to_string()))
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
        ExecuteMsg::ExtendLock { locker_id, new_unlock_time } => {
            execute_extend_lock(deps, env, info, locker_id, new_unlock_time)
        }
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
        } => execute_update_config(
            deps,
            info,
            admin,
            reward_controller,
            emergency_unlock_delay,
            platform_fee_bps,
        ),
        ExecuteMsg::WhitelistLP {
            lp_token,
            min_lock_duration,
            max_lock_duration,
            bonus_multiplier,
        } => execute_whitelist_lp(
            deps,
            info,
            lp_token,
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

    // Parse hook message
    let msg: Cw20HookMsg = from_json(&wrapper.msg)?;

    match msg {
        Cw20HookMsg::LockLP { unlock_time, metadata } => {
            execute_lock_lp(deps, env, sender, lp_token, amount, unlock_time, metadata)
        }
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
    // Validate LP token is whitelisted
    let whitelist = WHITELISTED_LPS
        .may_load(deps.storage, &lp_token)?
        .ok_or(ContractError::LPNotWhitelisted {})?;

    if !whitelist.enabled {
        return Err(ContractError::LPNotWhitelisted {});
    }

    // Validate unlock time
    let current_time = env.block.time.seconds();
    let lock_duration = unlock_time.checked_sub(current_time)
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

    // Create locker
    let mut config = CONFIG.load(deps.storage)?;
    let locker_id = config.next_locker_id;
    config.next_locker_id += 1;
    CONFIG.save(deps.storage, &config)?;

    let locker = Locker {
        id: locker_id,
        owner: sender.clone(),
        lp_token: lp_token.clone(),
        amount,
        locked_at: current_time,
        unlock_time,
        extended_count: 0,
        emergency_unlock_requested: None,
        metadata,
    };

    LOCKERS.save(deps.storage, locker_id, &locker)?;
    USER_LOCKERS.save(deps.storage, (&sender, locker_id), &true)?;

    // Update total locked
    TOTAL_LOCKED.update(
        deps.storage,
        &lp_token,
        |total| -> StdResult<_> {
            Ok(total.unwrap_or_default().checked_add(amount)?)
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "lock_lp")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("owner", sender)
        .add_attribute("lp_token", lp_token)
        .add_attribute("amount", amount)
        .add_attribute("unlock_time", unlock_time.to_string()))
}

fn execute_unlock_lp(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    locker_id: u64,
) -> Result<Response, ContractError> {
    let locker = LOCKERS.load(deps.storage, locker_id)?;

    // Verify owner
    if locker.owner != info.sender {
        return Err(ContractError::NotOwner {});
    }

    // Check unlock time (NOT affected by pause)
    let current_time = env.block.time.seconds();
    if current_time < locker.unlock_time {
        return Err(ContractError::StillLocked(locker.unlock_time));
    }

    // Remove locker
    LOCKERS.remove(deps.storage, locker_id);
    USER_LOCKERS.remove(deps.storage, (&locker.owner, locker_id));

    // Update total locked
    TOTAL_LOCKED.update(
        deps.storage,
        &locker.lp_token,
        |total| -> StdResult<_> {
            Ok(total.unwrap_or_default().checked_sub(locker.amount)?)
        },
    )?;

    // Transfer LP tokens back
    let transfer_msg = WasmMsg::Execute {
        contract_addr: locker.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: locker.owner.to_string(),
            amount: locker.amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "unlock_lp")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("owner", locker.owner)
        .add_attribute("amount", locker.amount))
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

    // Validate against whitelist
    let whitelist = WHITELISTED_LPS.load(deps.storage, &locker.lp_token)?;
    let current_time = env.block.time.seconds();
    let new_duration = new_unlock_time.checked_sub(current_time).unwrap_or(0);

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

    Ok(Response::new()
        .add_attribute("action", "extend_lock")
        .add_attribute("locker_id", locker_id.to_string())
        .add_attribute("old_unlock_time", old_unlock_time.to_string())
        .add_attribute("new_unlock_time", new_unlock_time.to_string()))
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

    let execute_at = locker.emergency_unlock_requested
        .ok_or(ContractError::EmergencyNotRequested {})?;

    if env.block.time.seconds() < execute_at {
        return Err(ContractError::EmergencyDelayNotPassed(execute_at));
    }

    // Remove locker
    LOCKERS.remove(deps.storage, locker_id);
    USER_LOCKERS.remove(deps.storage, (&locker.owner, locker_id));

    // Update total locked
    TOTAL_LOCKED.update(
        deps.storage,
        &locker.lp_token,
        |total| -> StdResult<_> {
            Ok(total.unwrap_or_default().checked_sub(locker.amount)?)
        },
    )?;

    // Transfer LP tokens back
    let transfer_msg = WasmMsg::Execute {
        contract_addr: locker.lp_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: locker.owner.to_string(),
            amount: locker.amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(transfer_msg)
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
        config.platform_fee_bps = fee;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_whitelist_lp(
    deps: DepsMut,
    info: MessageInfo,
    lp_token: String,
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
        min_lock_duration,
        max_lock_duration,
        enabled: true,
        bonus_multiplier,
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
        QueryMsg::LockersByOwner { owner, start_after, limit } => {
            to_json_binary(&query_lockers_by_owner(deps, owner, start_after, limit)?)
        }
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
    let start = start_after.map(|id| Bound::exclusive((&owner_addr, id)));

    let lockers: Vec<LockerResponse> = USER_LOCKERS
        .prefix(&owner_addr)
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .filter_map(|item| {
            item.ok().and_then(|(locker_id, _)| {
                LOCKERS.load(deps.storage, locker_id).ok().map(|locker| {
                    LockerResponse {
                        id: locker.id,
                        owner: locker.owner,
                        lp_token: locker.lp_token,
                        amount: locker.amount,
                        locked_at: locker.locked_at,
                        unlock_time: locker.unlock_time,
                        extended_count: locker.extended_count,
                        emergency_unlock_requested: locker.emergency_unlock_requested,
                        metadata: locker.metadata,
                    }
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
        min_lock_duration: whitelist.min_lock_duration,
        max_lock_duration: whitelist.max_lock_duration,
        enabled: whitelist.enabled,
        bonus_multiplier: whitelist.bonus_multiplier,
    })
}

fn query_all_whitelisted_lps(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<WhitelistedLPResponse>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.as_ref().map(|s| {
        deps.api.addr_validate(s).map(|addr| Bound::exclusive(&addr))
    }).transpose()?;

    WHITELISTED_LPS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, whitelist) = item?;
            Ok(WhitelistedLPResponse {
                lp_token: whitelist.lp_token,
                min_lock_duration: whitelist.min_lock_duration,
                max_lock_duration: whitelist.max_lock_duration,
                enabled: whitelist.enabled,
                bonus_multiplier: whitelist.bonus_multiplier,
            })
        })
        .collect()
}

fn query_total_locked(deps: Deps, lp_token: String) -> StdResult<TotalLockedResponse> {
    let lp_addr = deps.api.addr_validate(&lp_token)?;
    let total = TOTAL_LOCKED.may_load(deps.storage, &lp_addr)?.unwrap_or_default();
    
    Ok(TotalLockedResponse {
        lp_token: lp_addr,
        total_amount: total,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let version = cw2::get_contract_version(deps.storage)?;
    
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidMigration {});
    }

    match msg {
        MigrateMsg::V1ToV2 { reward_controller } => {
            let mut config = CONFIG.load(deps.storage)?;
            
            if let Some(addr) = reward_controller {
                config.reward_controller = Some(deps.api.addr_validate(&addr)?);
                CONFIG.save(deps.storage, &config)?;
            }

            cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

            Ok(Response::new()
                .add_attribute("action", "migrate")
                .add_attribute("from_version", version.version)
                .add_attribute("to_version", CONTRACT_VERSION))
        }
    }
}

// Helper function
use cosmwasm_std::from_json;
use cw_storage_plus::Bound;

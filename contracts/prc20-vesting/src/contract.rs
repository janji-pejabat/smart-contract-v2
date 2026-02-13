#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, GlobalStatsResponse, InstantiateMsg, QueryMsg,
    VestingCreation, VestingResponse, VestingSchedule,
};
use crate::state::{
    Config, GlobalStats, VestingAccount, BENEFICIARY_VESTINGS, CATEGORY_VESTINGS, CONFIG,
    GLOBAL_STATS, VESTING_ACCOUNTS, VESTING_COUNT,
};
use crate::vesting::{calculate_vested_amount, validate_schedule};

const CONTRACT_NAME: &str = "crates.io:prc20-vesting";
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
        paused: false,
    };
    CONFIG.save(deps.storage, &config)?;
    VESTING_COUNT.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", msg.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.paused && !matches!(msg, ExecuteMsg::SetPaused { .. }) {
        return Err(ContractError::Paused {});
    }

    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Claim { ids } => execute_claim(deps, env, info, ids),
        ExecuteMsg::Revoke { id } => execute_revoke(deps, env, info, id),
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, info, admin),
        ExecuteMsg::SetPaused { paused } => execute_set_paused(deps, info, paused),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let hook_msg: Cw20HookMsg = from_json(&cw20_msg.msg)?;
    let token_address = info.sender;

    match hook_msg {
        Cw20HookMsg::CreateVesting {
            beneficiary,
            schedule,
            category,
            revocable,
        } => create_vesting(
            deps,
            token_address,
            cw20_msg.amount,
            beneficiary,
            schedule,
            category,
            revocable,
        ),
        Cw20HookMsg::BatchCreateVesting { vestings } => {
            batch_create_vesting(deps, token_address, cw20_msg.amount, vestings)
        }
    }
}

fn create_vesting(
    deps: DepsMut,
    token_address: Addr,
    amount: Uint128,
    beneficiary: String,
    schedule: VestingSchedule,
    category: String,
    revocable: bool,
) -> Result<Response, ContractError> {
    let beneficiary_addr = deps.api.addr_validate(&beneficiary)?;

    validate_schedule(&schedule, amount)
        .map_err(|e| ContractError::InvalidSchedule { reason: e })?;

    let id = VESTING_COUNT.load(deps.storage)? + 1;
    VESTING_COUNT.save(deps.storage, &id)?;

    let vesting = VestingAccount {
        id,
        beneficiary: beneficiary_addr.clone(),
        token_address: token_address.clone(),
        total_amount: amount,
        released_amount: Uint128::zero(),
        revoked: false,
        category: category.clone(),
        revocable,
        schedule,
    };

    VESTING_ACCOUNTS.save(deps.storage, id, &vesting)?;
    BENEFICIARY_VESTINGS.save(deps.storage, (&beneficiary_addr, id), &true)?;
    CATEGORY_VESTINGS.save(deps.storage, (&category, id), &true)?;

    let mut stats = GLOBAL_STATS
        .may_load(deps.storage, &token_address)?
        .unwrap_or(GlobalStats {
            total_vested: Uint128::zero(),
            total_claimed: Uint128::zero(),
        });
    stats.total_vested += amount;
    GLOBAL_STATS.save(deps.storage, &token_address, &stats)?;

    Ok(Response::new()
        .add_attribute("action", "create_vesting")
        .add_attribute("id", id.to_string())
        .add_attribute("beneficiary", beneficiary)
        .add_attribute("amount", amount)
        .add_attribute("token", token_address))
}

fn batch_create_vesting(
    deps: DepsMut,
    token_address: Addr,
    total_amount: Uint128,
    vestings: Vec<VestingCreation>,
) -> Result<Response, ContractError> {
    let mut sum = Uint128::zero();
    let mut count = 0u64;
    let mut id_count = VESTING_COUNT.load(deps.storage)?;
    let mut stats = GLOBAL_STATS
        .may_load(deps.storage, &token_address)?
        .unwrap_or(GlobalStats {
            total_vested: Uint128::zero(),
            total_claimed: Uint128::zero(),
        });

    for v in vestings {
        sum += v.amount;
        let beneficiary_addr = deps.api.addr_validate(&v.beneficiary)?;
        validate_schedule(&v.schedule, v.amount)
            .map_err(|e| ContractError::InvalidSchedule { reason: e })?;

        id_count += 1;
        let vesting = VestingAccount {
            id: id_count,
            beneficiary: beneficiary_addr.clone(),
            token_address: token_address.clone(),
            total_amount: v.amount,
            released_amount: Uint128::zero(),
            revoked: false,
            category: v.category.clone(),
            revocable: v.revocable,
            schedule: v.schedule,
        };

        VESTING_ACCOUNTS.save(deps.storage, id_count, &vesting)?;
        BENEFICIARY_VESTINGS.save(deps.storage, (&beneficiary_addr, id_count), &true)?;
        CATEGORY_VESTINGS.save(deps.storage, (&v.category, id_count), &true)?;
        stats.total_vested += v.amount;
        count += 1;
    }

    if sum != total_amount {
        return Err(ContractError::InvalidInput {
            reason: format!(
                "Sum of vestings ({}) does not match received amount ({})",
                sum, total_amount
            ),
        });
    }

    VESTING_COUNT.save(deps.storage, &id_count)?;
    GLOBAL_STATS.save(deps.storage, &token_address, &stats)?;

    Ok(Response::new()
        .add_attribute("action", "batch_create_vesting")
        .add_attribute("count", count.to_string())
        .add_attribute("total_amount", total_amount)
        .add_attribute("first_id", (id_count - count + 1).to_string())
        .add_attribute("last_id", id_count.to_string()))
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ids: Vec<u64>,
) -> Result<Response, ContractError> {
    let mut msgs = vec![];
    let mut total_claimed = Uint128::zero();

    for id in &ids {
        let mut vesting = VESTING_ACCOUNTS
            .may_load(deps.storage, *id)?
            .ok_or(ContractError::VestingNotFound {})?;

        // Only beneficiary or admin can trigger claim?
        // Usually anyone can trigger claim for someone else, but it goes to beneficiary.
        // Let's restrict it to beneficiary for now, or just allow anyone.
        // If we allow anyone, we must ensure it goes to vesting.beneficiary.

        let vested = calculate_vested_amount(
            &vesting.schedule,
            vesting.total_amount,
            env.block.time.seconds(),
        )?;
        let claimable = vested.saturating_sub(vesting.released_amount);

        if claimable.is_zero() {
            continue;
        }

        vesting.released_amount += claimable;
        VESTING_ACCOUNTS.save(deps.storage, *id, &vesting)?;

        let mut stats = GLOBAL_STATS
            .may_load(deps.storage, &vesting.token_address)?
            .unwrap_or(GlobalStats {
                total_vested: Uint128::zero(),
                total_claimed: Uint128::zero(),
            });
        stats.total_claimed += claimable;
        total_claimed += claimable;
        GLOBAL_STATS.save(deps.storage, &vesting.token_address, &stats)?;

        msgs.push(WasmMsg::Execute {
            contract_addr: vesting.token_address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: vesting.beneficiary.to_string(),
                amount: claimable,
            })?,
            funds: vec![],
        });
    }

    if msgs.is_empty() {
        return Err(ContractError::NothingToClaim {});
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim")
        .add_attribute("beneficiary", info.sender)
        .add_attribute("total_claimed", total_claimed)
        .add_attribute(
            "ids",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(","),
        ))
}

pub fn execute_revoke(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut vesting = VESTING_ACCOUNTS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::VestingNotFound {})?;

    if !vesting.revocable {
        return Err(ContractError::NotRevocable {});
    }

    if vesting.revoked {
        return Err(ContractError::VestingAlreadyRevoked {});
    }

    let vested = calculate_vested_amount(
        &vesting.schedule,
        vesting.total_amount,
        env.block.time.seconds(),
    )?;
    let unvested = vesting.total_amount.saturating_sub(vested);

    vesting.revoked = true;
    let original_total = vesting.total_amount;
    vesting.total_amount = vested; // Vested amount is the new total
    vesting.schedule = crate::msg::VestingSchedule::Custom {
        milestones: vec![crate::msg::Milestone {
            timestamp: env.block.time.seconds(),
            amount: vested,
        }],
    };

    VESTING_ACCOUNTS.save(deps.storage, id, &vesting)?;

    let mut stats = GLOBAL_STATS
        .may_load(deps.storage, &vesting.token_address)?
        .unwrap_or(GlobalStats {
            total_vested: Uint128::zero(),
            total_claimed: Uint128::zero(),
        });
    stats.total_vested = stats.total_vested.saturating_sub(unvested);
    GLOBAL_STATS.save(deps.storage, &vesting.token_address, &stats)?;

    let mut msgs = vec![];
    if !unvested.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: vesting.token_address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: config.admin.to_string(),
                amount: unvested,
            })?,
            funds: vec![],
        });
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "revoke")
        .add_attribute("id", id.to_string())
        .add_attribute("unvested", unvested)
        .add_attribute("new_total", vested)
        .add_attribute("original_total", original_total))
}

pub fn execute_update_admin(
    deps: DepsMut,
    info: MessageInfo,
    admin: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let new_admin = deps.api.addr_validate(&admin)?;
    config.admin = new_admin;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_admin")
        .add_attribute("new_admin", admin))
}

pub fn execute_set_paused(
    deps: DepsMut,
    info: MessageInfo,
    paused: bool,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.paused = paused;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "set_paused")
        .add_attribute("paused", paused.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Vesting { id } => to_json_binary(&query_vesting(deps, env, id)?),
        QueryMsg::VestingsByBeneficiary {
            beneficiary,
            start_after,
            limit,
        } => to_json_binary(&query_vestings_by_beneficiary(
            deps,
            env,
            beneficiary,
            start_after,
            limit,
        )?),
        QueryMsg::VestingsByCategory {
            category,
            start_after,
            limit,
        } => to_json_binary(&query_vestings_by_category(
            deps,
            env,
            category,
            start_after,
            limit,
        )?),
        QueryMsg::ClaimableAmount { id } => to_json_binary(&query_claimable_amount(deps, env, id)?),
        QueryMsg::GlobalStats { token_address } => {
            to_json_binary(&query_global_stats(deps, token_address)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        admin: config.admin,
        paused: config.paused,
    })
}

fn query_vesting(deps: Deps, env: Env, id: u64) -> StdResult<VestingResponse> {
    let vesting = VESTING_ACCOUNTS.load(deps.storage, id)?;
    let vested = calculate_vested_amount(
        &vesting.schedule,
        vesting.total_amount,
        env.block.time.seconds(),
    )?;
    let claimable = vested.saturating_sub(vesting.released_amount);

    Ok(VestingResponse {
        id: vesting.id,
        beneficiary: vesting.beneficiary,
        token_address: vesting.token_address,
        total_amount: vesting.total_amount,
        released_amount: vesting.released_amount,
        revoked: vesting.revoked,
        category: vesting.category,
        revocable: vesting.revocable,
        schedule: vesting.schedule,
        claimable_amount: claimable,
    })
}

fn query_vestings_by_beneficiary(
    deps: Deps,
    env: Env,
    beneficiary: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<VestingResponse>> {
    let beneficiary_addr = deps.api.addr_validate(&beneficiary)?;
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(Bound::exclusive);

    let vestings: StdResult<Vec<VestingResponse>> = BENEFICIARY_VESTINGS
        .prefix(&beneficiary_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|id_res| {
            let id = id_res?;
            query_vesting(deps, env.clone(), id)
        })
        .collect();

    vestings
}

fn query_vestings_by_category(
    deps: Deps,
    env: Env,
    category: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<VestingResponse>> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(Bound::exclusive);

    let vestings: StdResult<Vec<VestingResponse>> = CATEGORY_VESTINGS
        .prefix(&category)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|id_res| {
            let id = id_res?;
            query_vesting(deps, env.clone(), id)
        })
        .collect();

    vestings
}

fn query_claimable_amount(deps: Deps, env: Env, id: u64) -> StdResult<Uint128> {
    let vesting = VESTING_ACCOUNTS.load(deps.storage, id)?;
    let vested = calculate_vested_amount(
        &vesting.schedule,
        vesting.total_amount,
        env.block.time.seconds(),
    )?;
    Ok(vested.saturating_sub(vesting.released_amount))
}

fn query_global_stats(deps: Deps, token_address: String) -> StdResult<GlobalStatsResponse> {
    let token_addr = deps.api.addr_validate(&token_address)?;
    let stats = GLOBAL_STATS
        .may_load(deps.storage, &token_addr)?
        .unwrap_or(GlobalStats {
            total_vested: Uint128::zero(),
            total_claimed: Uint128::zero(),
        });
    let count = VESTING_COUNT.load(deps.storage)?;
    Ok(GlobalStatsResponse {
        total_vested: stats.total_vested,
        total_claimed: stats.total_claimed,
        active_vesting_count: count,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Binary) -> Result<Response, ContractError> {
    Ok(Response::default())
}

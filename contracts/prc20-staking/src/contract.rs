use cosmwasm_std::{
    entry_point, from_json, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    APREstimateResponse, ConfigResponse, Cw20HookMsg, EligibilityResponse, ExecuteMsg,
    InstantiateMsg, PendingRewardsResponse, QueryMsg, RoomResponse, UserPositionResponse,
};
use crate::state::{
    AssetInfo, Config, RewardConfig, Room, UserPosition, CONFIG, ROOMS, USER_POSITIONS,
};

const CONTRACT_NAME: &str = "crates.io:prc20-staking";
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
        next_room_id: 1,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin))
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
        ExecuteMsg::Unstake {
            room_id,
            amount,
            token_address,
        } => execute_unstake(deps, env, info, room_id, amount, token_address),
        ExecuteMsg::ClaimRewards { room_id } => execute_claim_rewards(deps, env, info, room_id),
        ExecuteMsg::ToggleAutoCompound { room_id, enabled } => {
            execute_toggle_auto_compound(deps, env, info, room_id, enabled)
        }
        ExecuteMsg::Compound { room_id } => execute_compound(deps, env, info, room_id),
        ExecuteMsg::CreateRoom {
            name,
            stake_config,
            nft_config,
            auto_compound_config,
            early_withdraw_penalty,
            cooldown_period,
        } => execute_create_room(
            deps,
            env,
            info,
            name,
            stake_config,
            nft_config,
            auto_compound_config,
            early_withdraw_penalty,
            cooldown_period,
        ),
        ExecuteMsg::UpdateRoom {
            room_id,
            paused,
            early_withdraw_penalty,
            cooldown_period,
        } => execute_update_room(
            deps,
            info,
            room_id,
            paused,
            early_withdraw_penalty,
            cooldown_period,
        ),
        ExecuteMsg::AddRewardPool {
            room_id,
            reward_token,
            emission_per_second,
        } => execute_add_reward_pool(deps, env, info, room_id, reward_token, emission_per_second),
        ExecuteMsg::UpdateRewardPool {
            room_id,
            reward_token,
            emission_per_second,
        } => {
            execute_update_reward_pool(deps, env, info, room_id, reward_token, emission_per_second)
        }
        ExecuteMsg::FundRewardPool { room_id } => {
            execute_fund_reward_pool(deps, env, info, room_id)
        }
        ExecuteMsg::UpdateConfig { admin, paused } => {
            execute_update_config(deps, info, admin, paused)
        }
    }
}

// Admin Handlers
#[allow(clippy::too_many_arguments)]
fn execute_create_room(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    stake_config: crate::state::StakeConfig,
    nft_config: Option<crate::state::NFTConfig>,
    auto_compound_config: crate::state::AutoCompoundConfig,
    early_withdraw_penalty: Decimal,
    cooldown_period: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let room_id = config.next_room_id;
    config.next_room_id += 1;
    CONFIG.save(deps.storage, &config)?;

    let room = Room {
        id: room_id,
        name,
        stake_config,
        reward_configs: vec![],
        nft_config,
        auto_compound_config,
        total_staked_weight: Uint128::zero(),
        last_update_time: env.block.time.seconds(),
        early_withdraw_penalty,
        cooldown_period,
        paused: false,
    };

    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_attribute("action", "create_room")
        .add_attribute("room_id", room_id.to_string()))
}

fn execute_update_room(
    deps: DepsMut,
    info: MessageInfo,
    room_id: u64,
    paused: Option<bool>,
    early_withdraw_penalty: Option<Decimal>,
    cooldown_period: Option<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut room = ROOMS.load(deps.storage, room_id)?;

    if let Some(p) = paused {
        room.paused = p;
    }
    if let Some(penalty) = early_withdraw_penalty {
        room.early_withdraw_penalty = penalty;
    }
    if let Some(cp) = cooldown_period {
        room.cooldown_period = cp;
    }

    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_attribute("action", "update_room")
        .add_attribute("room_id", room_id.to_string()))
}

fn execute_add_reward_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    reward_token: AssetInfo,
    emission_per_second: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut room = ROOMS.load(deps.storage, room_id)?;

    // Update room rewards before changing config
    update_room_rewards(&mut room, env.block.time.seconds());

    // Check if token already exists
    if room
        .reward_configs
        .iter()
        .any(|rc| rc.reward_token == reward_token)
    {
        return Err(ContractError::InvalidRewardToken {});
    }

    room.reward_configs.push(RewardConfig {
        reward_token,
        emission_per_second,
        total_deposited: Uint128::zero(),
        total_claimed: Uint128::zero(),
        acc_reward_per_share: Decimal::zero(),
    });

    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_attribute("action", "add_reward_pool")
        .add_attribute("room_id", room_id.to_string()))
}

fn execute_update_reward_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    reward_token: AssetInfo,
    emission_per_second: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut room = ROOMS.load(deps.storage, room_id)?;
    update_room_rewards(&mut room, env.block.time.seconds());

    if let Some(rc) = room
        .reward_configs
        .iter_mut()
        .find(|rc| rc.reward_token == reward_token)
    {
        rc.emission_per_second = emission_per_second;
    } else {
        return Err(ContractError::InvalidRewardToken {});
    }

    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_attribute("action", "update_reward_pool")
        .add_attribute("room_id", room_id.to_string()))
}

fn execute_fund_reward_pool(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    room_id: u64,
) -> Result<Response, ContractError> {
    let mut room = ROOMS.load(deps.storage, room_id)?;

    // For native tokens
    if info.funds.is_empty() {
        return Err(ContractError::InsufficientStake {});
    }

    let coin = &info.funds[0];
    let asset = AssetInfo::Native(coin.denom.clone());

    if let Some(rc) = room
        .reward_configs
        .iter_mut()
        .find(|rc| rc.reward_token == asset)
    {
        rc.total_deposited = rc.total_deposited.checked_add(coin.amount)?;
    } else {
        return Err(ContractError::InvalidRewardToken {});
    }

    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_attribute("action", "fund_reward_pool")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("amount", coin.amount))
}

fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    paused: Option<bool>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(a) = admin {
        config.admin = deps.api.addr_validate(&a)?;
    }
    if let Some(p) = paused {
        config.paused = p;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// Helpers
fn update_room_rewards(room: &mut Room, current_time: u64) {
    if room.total_staked_weight.is_zero() || current_time <= room.last_update_time {
        room.last_update_time = current_time;
        return;
    }

    let duration = current_time.saturating_sub(room.last_update_time);

    for config in &mut room.reward_configs {
        // Calculate max possible reward based on pool balance
        let available = config.total_deposited.saturating_sub(config.total_claimed);
        if available.is_zero() {
            continue;
        }

        let mut reward_accrued = config.emission_per_second.multiply_ratio(duration, 1u128);
        if reward_accrued > available {
            reward_accrued = available;
        }

        if reward_accrued > Uint128::zero() {
            let share_increase = Decimal::from_ratio(reward_accrued, room.total_staked_weight);
            config.acc_reward_per_share = config
                .acc_reward_per_share
                .checked_add(share_increase)
                .unwrap_or(config.acc_reward_per_share);
        }
    }

    room.last_update_time = current_time;
}

fn calculate_user_weight(user_pos: &UserPosition, room: &Room) -> Uint128 {
    // Check AND rule
    if room.stake_config.is_and_rule {
        for token in &room.stake_config.stake_tokens {
            let user_token_stake = user_pos
                .staked_amounts
                .iter()
                .find(|(t, _)| t == token)
                .map(|(_, a)| *a)
                .unwrap_or(Uint128::zero());
            if user_token_stake < room.stake_config.min_stake_amount {
                return Uint128::zero();
            }
        }
    } else {
        // OR rule or single token
        let total_user_stake: Uint128 = user_pos.staked_amounts.iter().map(|(_, a)| *a).sum();
        if total_user_stake < room.stake_config.min_stake_amount {
            return Uint128::zero();
        }
    }

    let raw_stake: Uint128 = user_pos.staked_amounts.iter().map(|(_, amt)| *amt).sum();
    raw_stake * user_pos.nft_multiplier
}

fn update_user_rewards(user_pos: &mut UserPosition, room: &Room, weight: Uint128) {
    for reward_config in &room.reward_configs {
        let last_paid = user_pos
            .last_reward_per_share
            .iter()
            .find(|(asset, _)| asset == &reward_config.reward_token)
            .map(|(_, val)| *val)
            .unwrap_or(Decimal::zero());

        let pending = weight * (reward_config.acc_reward_per_share - last_paid);

        if let Some(pos) = user_pos
            .pending_rewards
            .iter_mut()
            .find(|(asset, _)| asset == &reward_config.reward_token)
        {
            pos.1 += pending;
        } else {
            user_pos
                .pending_rewards
                .push((reward_config.reward_token.clone(), pending));
        }

        if let Some(pos) = user_pos
            .last_reward_per_share
            .iter_mut()
            .find(|(asset, _)| asset == &reward_config.reward_token)
        {
            pos.1 = reward_config.acc_reward_per_share;
        } else {
            user_pos.last_reward_per_share.push((
                reward_config.reward_token.clone(),
                reward_config.acc_reward_per_share,
            ));
        }
    }
}

fn query_nft_multiplier(deps: Deps, user: &Addr, room: &Room) -> StdResult<Decimal> {
    let nft_config = match &room.nft_config {
        Some(config) => config,
        None => return Ok(Decimal::one()),
    };

    // Query CW721 to see if user owns any tokens
    let query_msg = cw721::Cw721QueryMsg::Tokens {
        owner: user.to_string(),
        start_after: None,
        limit: Some(1),
    };

    let res: cw721::TokensResponse = deps
        .querier
        .query_wasm_smart(&nft_config.nft_address, &query_msg)?;

    if res.tokens.is_empty() {
        if nft_config.required_for_staking {
            // This might happen if user sold NFT after staking
            return Ok(Decimal::zero());
        }
        return Ok(Decimal::one());
    }

    // Check for specific tiers by querying tokens and then metadata if needed.
    // Here we implement a tier logic based on the number of NFTs owned or specific tiers.
    // For a robust implementation, we return the highest multiplier from the tiers
    // if the user owns at least one token.
    let mut highest_multiplier = Decimal::one();
    if !res.tokens.is_empty() {
        for tier in &nft_config.tier_multipliers {
            if tier.multiplier > highest_multiplier {
                highest_multiplier = tier.multiplier;
            }
        }
    }

    Ok(highest_multiplier)
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let hook_msg: Cw20HookMsg = from_json(&msg.msg)?;
    match hook_msg {
        Cw20HookMsg::Stake { room_id } => {
            execute_stake(deps, env, info, room_id, msg.sender, msg.amount)
        }
        Cw20HookMsg::FundPool { room_id } => {
            execute_fund_pool_cw20(deps, info, room_id, msg.amount)
        }
    }
}

fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    user: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut room = ROOMS.load(deps.storage, room_id)?;
    if room.paused {
        return Err(ContractError::Paused {});
    }

    let user_addr = deps.api.addr_validate(&user)?;
    let stake_token = info.sender; // The CW20 contract that called Receive

    if !room.stake_config.stake_tokens.contains(&stake_token) {
        return Err(ContractError::InvalidStakeToken {});
    }

    let mut user_pos = USER_POSITIONS
        .may_load(deps.storage, (&user_addr, room_id))?
        .unwrap_or(UserPosition {
            room_id,
            user: user_addr.clone(),
            staked_amounts: vec![],
            last_reward_per_share: vec![],
            pending_rewards: vec![],
            staked_at: env.block.time.seconds(),
            last_interaction: env.block.time.seconds(),
            auto_compound_enabled: false,
            nft_multiplier: Decimal::one(),
        });

    // Update room rewards before modifying stake
    update_room_rewards(&mut room, env.block.time.seconds());

    // Update user rewards with CURRENT weight
    let old_weight = calculate_user_weight(&user_pos, &room);
    update_user_rewards(&mut user_pos, &room, old_weight);

    // Update NFT multiplier (refresh on interaction)
    user_pos.nft_multiplier = query_nft_multiplier(deps.as_ref(), &user_addr, &room)?;

    // Add stake
    if let Some(pos) = user_pos
        .staked_amounts
        .iter_mut()
        .find(|(t, _)| t == stake_token)
    {
        pos.1 = pos.1.checked_add(amount)?;
    } else {
        user_pos.staked_amounts.push((stake_token, amount));
    }

    // Update weights
    let new_weight = calculate_user_weight(&user_pos, &room);
    room.total_staked_weight = room
        .total_staked_weight
        .checked_sub(old_weight)?
        .checked_add(new_weight)?;

    user_pos.last_interaction = env.block.time.seconds();

    ROOMS.save(deps.storage, room_id, &room)?;
    USER_POSITIONS.save(deps.storage, (&user_addr, room_id), &user_pos)?;

    Ok(Response::new()
        .add_attribute("action", "stake")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("user", user)
        .add_attribute("amount", amount))
}

fn execute_fund_pool_cw20(
    deps: DepsMut,
    info: MessageInfo,
    room_id: u64,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut room = ROOMS.load(deps.storage, room_id)?;
    let asset = AssetInfo::Cw20(info.sender);

    if let Some(rc) = room
        .reward_configs
        .iter_mut()
        .find(|rc| rc.reward_token == asset)
    {
        rc.total_deposited = rc.total_deposited.checked_add(amount)?;
    } else {
        return Err(ContractError::InvalidRewardToken {});
    }

    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_attribute("action", "fund_reward_pool_cw20")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("amount", amount))
}

fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    amount: Uint128,
    token_address: String,
) -> Result<Response, ContractError> {
    let mut room = ROOMS.load(deps.storage, room_id)?;
    let user_addr = info.sender;
    let stake_token = deps.api.addr_validate(&token_address)?;

    let mut user_pos = USER_POSITIONS.load(deps.storage, (&user_addr, room_id))?;

    // Check cooldown
    if env.block.time.seconds() < user_pos.last_interaction + room.cooldown_period {
        return Err(ContractError::CooldownNotElapsed {});
    }

    let pos_idx = user_pos
        .staked_amounts
        .iter()
        .position(|(t, _)| t == stake_token)
        .ok_or(ContractError::InvalidStakeToken {})?;

    if user_pos.staked_amounts[pos_idx].1 < amount {
        return Err(ContractError::InsufficientStake {});
    }

    // Update room/user rewards
    update_room_rewards(&mut room, env.block.time.seconds());
    let old_weight = calculate_user_weight(&user_pos, &room);
    update_user_rewards(&mut user_pos, &room, old_weight);

    // Refresh NFT multiplier
    user_pos.nft_multiplier = query_nft_multiplier(deps.as_ref(), &user_addr, &room)?;

    // Deduct stake
    user_pos.staked_amounts[pos_idx].1 = user_pos.staked_amounts[pos_idx].1.checked_sub(amount)?;

    // Apply early withdraw penalty
    let penalty_amount = amount * room.early_withdraw_penalty;
    let transfer_amount = amount.checked_sub(penalty_amount)?;

    // Update weights
    let new_weight = calculate_user_weight(&user_pos, &room);
    room.total_staked_weight = room
        .total_staked_weight
        .checked_sub(old_weight)?
        .checked_add(new_weight)?;

    user_pos.last_interaction = env.block.time.seconds();

    ROOMS.save(deps.storage, room_id, &room)?;
    USER_POSITIONS.save(deps.storage, (&user_addr, room_id), &user_pos)?;

    let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stake_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: user_addr.to_string(),
            amount: transfer_amount,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "unstake")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("amount", amount)
        .add_attribute("penalty", penalty_amount))
}

fn execute_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
) -> Result<Response, ContractError> {
    let mut room = ROOMS.load(deps.storage, room_id)?;
    let user_addr = info.sender;
    let mut user_pos = USER_POSITIONS.load(deps.storage, (&user_addr, room_id))?;

    update_room_rewards(&mut room, env.block.time.seconds());
    let old_weight = calculate_user_weight(&user_pos, &room);
    update_user_rewards(&mut user_pos, &room, old_weight);

    // Refresh NFT multiplier
    user_pos.nft_multiplier = query_nft_multiplier(deps.as_ref(), &user_addr, &room)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut claimed_assets: Vec<String> = vec![];

    for (asset, amount) in user_pos.pending_rewards.iter_mut() {
        if amount.is_zero() {
            continue;
        }

        let claim_amount = *amount;
        *amount = Uint128::zero();

        // Check pool balance
        if let Some(rc) = room
            .reward_configs
            .iter_mut()
            .find(|rc| &rc.reward_token == asset)
        {
            rc.total_claimed = rc.total_claimed.checked_add(claim_amount)?;
        }

        let msg = match asset {
            AssetInfo::Cw20(addr) => CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: user_addr.to_string(),
                    amount: claim_amount,
                })?,
                funds: vec![],
            }),
            AssetInfo::Native(denom) => CosmosMsg::Bank(BankMsg::Send {
                to_address: user_addr.to_string(),
                amount: vec![Coin {
                    denom: denom.clone(),
                    amount: claim_amount,
                }],
            }),
        };
        messages.push(msg);
        claimed_assets.push(format!("{:?}:{}", asset, claim_amount));
    }

    if messages.is_empty() {
        return Err(ContractError::NoRewards {});
    }

    USER_POSITIONS.save(deps.storage, (&user_addr, room_id), &user_pos)?;
    ROOMS.save(deps.storage, room_id, &room)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "claim_rewards")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("claimed", claimed_assets.join(", ")))
}

fn execute_toggle_auto_compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
    enabled: bool,
) -> Result<Response, ContractError> {
    let room = ROOMS.load(deps.storage, room_id)?;
    if !room.auto_compound_config.enabled {
        return Err(ContractError::AutoCompoundNotAllowed {});
    }

    let user_addr = info.sender;
    let mut user_pos = USER_POSITIONS.load(deps.storage, (&user_addr, room_id))?;

    // Check eligibility
    let total_stake: Uint128 = user_pos.staked_amounts.iter().map(|(_, a)| *a).sum();
    let has_min_stake = total_stake >= room.auto_compound_config.min_stake_threshold;

    let nft_multiplier = query_nft_multiplier(deps.as_ref(), &user_addr, &room)?;
    let has_nft = nft_multiplier > Decimal::one(); // Simple check for now

    let eligible = if room.auto_compound_config.nft_required
        && room.auto_compound_config.min_stake_threshold > Uint128::zero()
    {
        has_min_stake && has_nft
    } else if room.auto_compound_config.nft_required {
        has_nft
    } else {
        has_min_stake
    };

    if enabled && !eligible {
        return Err(ContractError::AutoCompoundNotAllowed {});
    }

    user_pos.auto_compound_enabled = enabled;
    user_pos.last_interaction = env.block.time.seconds();
    USER_POSITIONS.save(deps.storage, (&user_addr, room_id), &user_pos)?;

    Ok(Response::new()
        .add_attribute("action", "toggle_auto_compound")
        .add_attribute("enabled", enabled.to_string()))
}

fn execute_compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    room_id: u64,
) -> Result<Response, ContractError> {
    let mut room = ROOMS.load(deps.storage, room_id)?;
    let user_addr = info.sender;
    let mut user_pos = USER_POSITIONS.load(deps.storage, (&user_addr, room_id))?;

    if !user_pos.auto_compound_enabled {
        return Err(ContractError::AutoCompoundNotAllowed {});
    }

    update_room_rewards(&mut room, env.block.time.seconds());
    let old_weight = calculate_user_weight(&user_pos, &room);
    update_user_rewards(&mut user_pos, &room, old_weight);

    // Refresh NFT multiplier
    user_pos.nft_multiplier = query_nft_multiplier(deps.as_ref(), &user_addr, &room)?;

    let mut compounded_amount = Uint128::zero();

    // Compound only if reward token is one of the stake tokens
    for (asset, amount) in user_pos.pending_rewards.iter_mut() {
        if amount.is_zero() {
            continue;
        }

        if let AssetInfo::Cw20(token_addr) = asset {
            if let Some(stake_pos) = user_pos
                .staked_amounts
                .iter_mut()
                .find(|(t, _)| t == token_addr)
            {
                stake_pos.1 += *amount;
                compounded_amount += *amount;
                *amount = Uint128::zero();
            }
        }
    }

    if compounded_amount.is_zero() {
        return Err(ContractError::NoRewards {});
    }

    // Update weights
    let new_weight = calculate_user_weight(&user_pos, &room);
    room.total_staked_weight = room
        .total_staked_weight
        .checked_sub(old_weight)?
        .checked_add(new_weight)?;

    user_pos.last_interaction = env.block.time.seconds();

    ROOMS.save(deps.storage, room_id, &room)?;
    USER_POSITIONS.save(deps.storage, (&user_addr, room_id), &user_pos)?;

    Ok(Response::new()
        .add_attribute("action", "compound")
        .add_attribute("room_id", room_id.to_string())
        .add_attribute("amount", compounded_amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Room { room_id } => to_json_binary(&query_room(deps, room_id)?),
        QueryMsg::Rooms { start_after, limit } => {
            to_json_binary(&query_rooms(deps, start_after, limit)?)
        }
        QueryMsg::UserPosition { room_id, user } => {
            to_json_binary(&query_user_position(deps, room_id, user)?)
        }
        QueryMsg::PendingRewards { room_id, user } => {
            to_json_binary(&query_pending_rewards(deps, env, room_id, user)?)
        }
        QueryMsg::APREstimate { room_id } => to_json_binary(&query_apr_estimate(deps, room_id)?),
        QueryMsg::Eligibility { room_id, user } => {
            to_json_binary(&query_eligibility(deps, room_id, user)?)
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

fn query_room(deps: Deps, room_id: u64) -> StdResult<RoomResponse> {
    let room = ROOMS.load(deps.storage, room_id)?;
    Ok(RoomResponse { room })
}

fn query_rooms(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<RoomResponse>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(cw_storage_plus::Bound::exclusive);

    ROOMS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, room) = item?;
            Ok(RoomResponse { room })
        })
        .collect()
}

fn query_user_position(deps: Deps, room_id: u64, user: String) -> StdResult<UserPositionResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let position = USER_POSITIONS.may_load(deps.storage, (&user_addr, room_id))?;
    Ok(UserPositionResponse { position })
}

fn query_pending_rewards(
    deps: Deps,
    env: Env,
    room_id: u64,
    user: String,
) -> StdResult<PendingRewardsResponse> {
    let room = ROOMS.load(deps.storage, room_id)?;
    let user_addr = deps.api.addr_validate(&user)?;
    let user_pos = USER_POSITIONS.may_load(deps.storage, (&user_addr, room_id))?;

    if let Some(mut up) = user_pos {
        let mut room_copy = room.clone();
        update_room_rewards(&mut room_copy, env.block.time.seconds());

        // Use cached multiplier for query or refresh it if we want it super accurate
        // For query, using cached is fine and safer (no gas limit issues if complex)
        let weight = calculate_user_weight(&up, &room_copy);
        update_user_rewards(&mut up, &room_copy, weight);
        Ok(PendingRewardsResponse {
            rewards: up.pending_rewards,
        })
    } else {
        Ok(PendingRewardsResponse { rewards: vec![] })
    }
}

fn query_apr_estimate(deps: Deps, room_id: u64) -> StdResult<APREstimateResponse> {
    let room = ROOMS.load(deps.storage, room_id)?;
    let mut aprs = vec![];

    if room.total_staked_weight.is_zero() {
        for rc in room.reward_configs {
            aprs.push((rc.reward_token, Decimal::zero()));
        }
    } else {
        let year_seconds = 365u128 * 24 * 3600;
        for rc in room.reward_configs {
            // Adjust emission based on remaining balance for a more accurate dynamic APR
            let available = rc.total_deposited.saturating_sub(rc.total_claimed);
            let effective_emission = if available.is_zero() {
                Uint128::zero()
            } else {
                rc.emission_per_second
            };

            let annual_emission = effective_emission.checked_mul(Uint128::from(year_seconds))?;
            let apr = Decimal::from_ratio(annual_emission, room.total_staked_weight);
            aprs.push((rc.reward_token, apr));
        }
    }

    Ok(APREstimateResponse { aprs })
}

fn query_eligibility(deps: Deps, room_id: u64, user: String) -> StdResult<EligibilityResponse> {
    let room = ROOMS.load(deps.storage, room_id)?;
    let user_addr = deps.api.addr_validate(&user)?;

    let multiplier = query_nft_multiplier(deps, &user_addr, &room)?;

    let can_stake = if let Some(nft_config) = &room.nft_config {
        if nft_config.required_for_staking {
            multiplier > Decimal::zero()
        } else {
            true
        }
    } else {
        true
    };

    let user_pos = USER_POSITIONS.may_load(deps.storage, (&user_addr, room_id))?;
    let can_auto_compound = if room.auto_compound_config.enabled {
        let total_stake: Uint128 = user_pos
            .map(|up| up.staked_amounts.iter().map(|(_, a)| *a).sum())
            .unwrap_or(Uint128::zero());
        let has_min_stake = total_stake >= room.auto_compound_config.min_stake_threshold;
        let has_nft = multiplier > Decimal::one();

        if room.auto_compound_config.nft_required
            && room.auto_compound_config.min_stake_threshold > Uint128::zero()
        {
            has_min_stake && has_nft
        } else if room.auto_compound_config.nft_required {
            has_nft
        } else {
            has_min_stake
        }
    } else {
        false
    };

    Ok(EligibilityResponse {
        can_stake,
        can_auto_compound,
        nft_multiplier: multiplier,
    })
}

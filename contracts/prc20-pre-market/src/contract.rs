use cosmwasm_std::{
    entry_point, from_json, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RoundConfig};
use crate::state::{
    listings, Config, GlobalStats, Listing, ListingStatus, Purchase, Round, CONFIG,
    NEXT_LISTING_ID, NEXT_PURCHASE_ID, STATS, TOKEN_BLACKLIST, USER_PURCHASES, USER_TOTAL_BOUGHT,
};

const CONTRACT_NAME: &str = "crates.io:prc20-pre-market";
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
    let fee_receiver = deps.api.addr_validate(&msg.fee_receiver)?;

    if msg.platform_fee_bps > 5000 {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "Fee too high (max 50%)",
        )));
    }

    let config = Config {
        admin,
        platform_fee_bps: msg.platform_fee_bps,
        fee_receiver,
        native_denom: msg.native_denom,
        paused: false,
    };

    CONFIG.save(deps.storage, &config)?;
    STATS.save(
        deps.storage,
        &GlobalStats {
            total_volume_paxi: Uint128::zero(),
            total_trades: 0,
            total_fees_collected: Uint128::zero(),
        },
    )?;
    NEXT_LISTING_ID.save(deps.storage, &1u64)?;
    NEXT_PURCHASE_ID.save(deps.storage, &1u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute("platform_fee_bps", msg.platform_fee_bps.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            admin,
            platform_fee_bps,
            fee_receiver,
            paused,
        } => execute_update_config(deps, info, admin, platform_fee_bps, fee_receiver, paused),
        ExecuteMsg::SetTokenBlacklist {
            token_address,
            blacklisted,
        } => execute_set_token_blacklist(deps, info, token_address, blacklisted),
        ExecuteMsg::Receive(msg) => execute_receive(deps, info, msg),
        ExecuteMsg::Buy {
            listing_id,
            amount,
            referrer,
        } => execute_buy(deps, env, info, listing_id, amount, referrer),
        ExecuteMsg::CancelListing { listing_id } => execute_cancel_listing(deps, info, listing_id),
        ExecuteMsg::PauseListing { listing_id } => execute_pause_listing(deps, info, listing_id),
        ExecuteMsg::ResumeListing { listing_id } => execute_resume_listing(deps, info, listing_id),
        ExecuteMsg::UpdateListingMetadata {
            listing_id,
            metadata,
        } => execute_update_listing_metadata(deps, info, listing_id, metadata),
    }
}

pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    listing_id: u64,
    amount: Uint128,
    referrer: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.paused {
        return Err(ContractError::Paused {});
    }

    let mut listing = listings().load(deps.storage, listing_id)?;
    if listing.status != ListingStatus::Active {
        return Err(ContractError::ListingNotActive {});
    }

    let current_time = env.block.time.seconds();

    // Find active round
    let round = listing
        .rounds
        .iter()
        .find(|r| current_time >= r.start_time && current_time <= r.end_time)
        .ok_or_else(|| {
            ContractError::Std(cosmwasm_std::StdError::generic_err("No active round found"))
        })?;

    if let Some(limit) = round.max_wallet_limit {
        let already_bought = USER_TOTAL_BOUGHT
            .may_load(deps.storage, (listing_id, info.sender.clone()))?
            .unwrap_or_default();
        if already_bought.checked_add(amount)? > limit {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "Wallet limit exceeded for this round",
            )));
        }
    }

    if amount < listing.min_buy {
        return Err(ContractError::BelowMinLimit {});
    }

    if amount > listing.max_buy || amount > listing.remaining_amount {
        return Err(ContractError::AboveMaxLimit {});
    }

    if info.sender == listing.seller {
        return Err(ContractError::SelfBuyNotAllowed {});
    }

    // Check whitelist for the round
    if let Some(whitelist) = &round.whitelist {
        if !whitelist.contains(&info.sender) {
            return Err(ContractError::NotOnWhitelist {});
        }
    }

    let total_paxi = amount.multiply_ratio(round.price_per_token, Uint128::one());

    // Verify payment
    let paid_paxi = info
        .funds
        .iter()
        .find(|c| c.denom == config.native_denom)
        .map(|c| c.amount)
        .unwrap_or(Uint128::zero());

    if paid_paxi < total_paxi {
        return Err(ContractError::InsufficientFunds {});
    }

    // Refund excess if any
    let mut messages: Vec<CosmosMsg> = vec![];
    if paid_paxi > total_paxi {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: config.native_denom.clone(),
                amount: paid_paxi.checked_sub(total_paxi)?,
            }],
        }));
    }

    // Calculate fees
    let platform_fee = total_paxi.multiply_ratio(config.platform_fee_bps, 10000u64);
    let royalty_fee = if listing.royalty_address.is_some() {
        total_paxi.multiply_ratio(listing.royalty_bps, 10000u64)
    } else {
        Uint128::zero()
    };

    let mut remaining_to_seller = total_paxi
        .checked_sub(platform_fee)?
        .checked_sub(royalty_fee)?;

    // Handle platform fee and referral
    if !platform_fee.is_zero() {
        if let Some(ref_str) = referrer {
            let ref_addr = deps.api.addr_validate(&ref_str)?;
            if ref_addr != info.sender && ref_addr != listing.seller {
                let referral_fee = platform_fee.multiply_ratio(500u64, 10000u64); // 5% of platform fee
                if !referral_fee.is_zero() {
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: ref_addr.to_string(),
                        amount: vec![Coin {
                            denom: config.native_denom.clone(),
                            amount: referral_fee,
                        }],
                    }));
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.fee_receiver.to_string(),
                        amount: vec![Coin {
                            denom: config.native_denom.clone(),
                            amount: platform_fee.checked_sub(referral_fee)?,
                        }],
                    }));
                } else {
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: config.fee_receiver.to_string(),
                        amount: vec![Coin {
                            denom: config.native_denom.clone(),
                            amount: platform_fee,
                        }],
                    }));
                }
            } else {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.fee_receiver.to_string(),
                    amount: vec![Coin {
                        denom: config.native_denom.clone(),
                        amount: platform_fee,
                    }],
                }));
            }
        } else {
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: config.fee_receiver.to_string(),
                amount: vec![Coin {
                    denom: config.native_denom.clone(),
                    amount: platform_fee,
                }],
            }));
        }
    }

    // Handle royalty
    if !royalty_fee.is_zero() {
        if let Some(royalty_addr) = &listing.royalty_address {
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: royalty_addr.to_string(),
                amount: vec![Coin {
                    denom: config.native_denom.clone(),
                    amount: royalty_fee,
                }],
            }));
        } else {
            // Should not happen based on logic above, but for safety
            remaining_to_seller = remaining_to_seller.checked_add(royalty_fee)?;
        }
    }

    // Pay seller
    if !remaining_to_seller.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: listing.seller.to_string(),
            amount: vec![Coin {
                denom: config.native_denom.clone(),
                amount: remaining_to_seller,
            }],
        }));
    }

    // Deliver PRC20 to buyer
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: listing.token_address.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        })?,
        funds: vec![],
    }));

    // Update listing
    listing.remaining_amount = listing.remaining_amount.checked_sub(amount)?;
    if listing.remaining_amount.is_zero() {
        listing.status = ListingStatus::Sold;
    }
    listings().save(deps.storage, listing_id, &listing)?;

    // Update user total bought
    USER_TOTAL_BOUGHT.update(
        deps.storage,
        (listing_id, info.sender.clone()),
        |old| -> StdResult<_> { Ok(old.unwrap_or_default().checked_add(amount)?) },
    )?;

    // Update stats
    let mut stats = STATS.load(deps.storage)?;
    stats.total_volume_paxi = stats.total_volume_paxi.checked_add(total_paxi)?;
    stats.total_trades += 1;
    stats.total_fees_collected = stats.total_fees_collected.checked_add(platform_fee)?;
    STATS.save(deps.storage, &stats)?;

    // Record purchase
    let purchase_id = NEXT_PURCHASE_ID.load(deps.storage)?;
    NEXT_PURCHASE_ID.save(deps.storage, &(purchase_id + 1))?;

    let purchase = Purchase {
        id: purchase_id,
        listing_id,
        buyer: info.sender.clone(),
        amount,
        total_paid: total_paxi,
        timestamp: env.block.time.seconds(),
    };
    USER_PURCHASES.save(deps.storage, (info.sender.clone(), purchase_id), &purchase)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "buy")
        .add_attribute("listing_id", listing_id.to_string())
        .add_attribute("buyer", info.sender)
        .add_attribute("amount", amount.to_string())
        .add_attribute("round", round.name.clone())
        .add_attribute("total_paxi", total_paxi.to_string()))
}

pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: Cw20HookMsg = from_json(&wrapper.msg)?;

    match msg {
        Cw20HookMsg::CreateListing {
            min_buy,
            max_buy,
            rounds,
            metadata,
            royalty_address,
            royalty_bps,
        } => execute_create_listing(
            deps,
            info,
            wrapper.sender,
            wrapper.amount,
            min_buy,
            max_buy,
            rounds,
            metadata,
            royalty_address,
            royalty_bps,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_create_listing(
    deps: DepsMut,
    info: MessageInfo,
    seller: String,
    amount: Uint128,
    min_buy: Option<Uint128>,
    max_buy: Option<Uint128>,
    rounds: Vec<RoundConfig>,
    metadata: String,
    royalty_address: Option<String>,
    royalty_bps: Option<u64>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.paused {
        return Err(ContractError::Paused {});
    }

    if rounds.is_empty() {
        return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
            "At least one round is required",
        )));
    }

    if let Some(bps) = royalty_bps {
        if bps > 5000 {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "Royalty too high (max 50%)",
            )));
        }
    }

    if amount.is_zero() {
        return Err(ContractError::InsufficientFunds {});
    }

    let token_address = info.sender.clone();
    let is_blacklisted = TOKEN_BLACKLIST
        .may_load(deps.storage, &token_address)?
        .unwrap_or(false);
    if is_blacklisted {
        return Err(ContractError::TokenBlacklisted {});
    }

    let seller_addr = deps.api.addr_validate(&seller)?;
    let listing_id = NEXT_LISTING_ID.load(deps.storage)?;
    NEXT_LISTING_ID.save(deps.storage, &(listing_id + 1))?;

    let mut validated_rounds = vec![];
    let mut last_end_time = 0;

    for r in rounds {
        if r.start_time >= r.end_time {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                format!("Round {} start_time must be before end_time", r.name),
            )));
        }
        if r.start_time < last_end_time {
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                format!("Round {} overlaps with previous round", r.name),
            )));
        }
        if r.price_per_token.is_zero() {
            return Err(ContractError::InvalidPrice {});
        }

        let whitelist = match r.whitelist {
            Some(w) => {
                let mut v = vec![];
                for addr in w {
                    v.push(deps.api.addr_validate(&addr)?);
                }
                Some(v)
            }
            None => None,
        };

        validated_rounds.push(Round {
            name: r.name,
            start_time: r.start_time,
            end_time: r.end_time,
            price_per_token: r.price_per_token,
            max_wallet_limit: r.max_wallet_limit,
            whitelist,
        });
        last_end_time = r.end_time;
    }

    let validated_royalty_address = match royalty_address {
        Some(r) => Some(deps.api.addr_validate(&r)?),
        None => None,
    };

    let listing = Listing {
        id: listing_id,
        seller: seller_addr,
        token_address: token_address.clone(),
        total_amount: amount,
        remaining_amount: amount,
        min_buy: min_buy.unwrap_or(Uint128::one()),
        max_buy: max_buy.unwrap_or(amount),
        rounds: validated_rounds,
        status: ListingStatus::Active,
        metadata,
        royalty_address: validated_royalty_address,
        royalty_bps: royalty_bps.unwrap_or(0),
    };

    listings().save(deps.storage, listing_id, &listing)?;

    Ok(Response::new()
        .add_attribute("action", "create_listing")
        .add_attribute("listing_id", listing_id.to_string())
        .add_attribute("seller", seller)
        .add_attribute("token", token_address)
        .add_attribute("amount", amount.to_string()))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
    platform_fee_bps: Option<u64>,
    fee_receiver: Option<String>,
    paused: Option<bool>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin)?;
    }
    if let Some(bps) = platform_fee_bps {
        if bps > 5000 {
            // Max 50%
            return Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
                "Fee too high",
            )));
        }
        config.platform_fee_bps = bps;
    }
    if let Some(receiver) = fee_receiver {
        config.fee_receiver = deps.api.addr_validate(&receiver)?;
    }
    if let Some(p) = paused {
        config.paused = p;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn execute_set_token_blacklist(
    deps: DepsMut,
    info: MessageInfo,
    token_address: String,
    blacklisted: bool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let token_addr = deps.api.addr_validate(&token_address)?;
    TOKEN_BLACKLIST.save(deps.storage, &token_addr, &blacklisted)?;

    Ok(Response::new()
        .add_attribute("action", "set_token_blacklist")
        .add_attribute("token", token_address)
        .add_attribute("blacklisted", blacklisted.to_string()))
}

pub fn execute_cancel_listing(
    deps: DepsMut,
    info: MessageInfo,
    listing_id: u64,
) -> Result<Response, ContractError> {
    let mut listing = listings().load(deps.storage, listing_id)?;
    if info.sender != listing.seller {
        return Err(ContractError::Unauthorized {});
    }

    if listing.status != ListingStatus::Active && listing.status != ListingStatus::Paused {
        return Err(ContractError::ListingNotActive {});
    }

    let refund_amount = listing.remaining_amount;
    listing.remaining_amount = Uint128::zero();
    listing.status = ListingStatus::Cancelled;
    listings().save(deps.storage, listing_id, &listing)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    if !refund_amount.is_zero() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: listing.token_address.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: listing.seller.to_string(),
                amount: refund_amount,
            })?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "cancel_listing")
        .add_attribute("listing_id", listing_id.to_string()))
}

pub fn execute_pause_listing(
    deps: DepsMut,
    info: MessageInfo,
    listing_id: u64,
) -> Result<Response, ContractError> {
    let mut listing = listings().load(deps.storage, listing_id)?;
    if info.sender != listing.seller {
        return Err(ContractError::Unauthorized {});
    }

    if listing.status != ListingStatus::Active {
        return Err(ContractError::ListingNotActive {});
    }

    listing.status = ListingStatus::Paused;
    listings().save(deps.storage, listing_id, &listing)?;

    Ok(Response::new()
        .add_attribute("action", "pause_listing")
        .add_attribute("listing_id", listing_id.to_string()))
}

pub fn execute_resume_listing(
    deps: DepsMut,
    info: MessageInfo,
    listing_id: u64,
) -> Result<Response, ContractError> {
    let mut listing = listings().load(deps.storage, listing_id)?;
    if info.sender != listing.seller {
        return Err(ContractError::Unauthorized {});
    }

    if listing.status != ListingStatus::Paused {
        return Err(ContractError::ListingNotActive {});
    }

    listing.status = ListingStatus::Active;
    listings().save(deps.storage, listing_id, &listing)?;

    Ok(Response::new()
        .add_attribute("action", "resume_listing")
        .add_attribute("listing_id", listing_id.to_string()))
}

pub fn execute_update_listing_metadata(
    deps: DepsMut,
    info: MessageInfo,
    listing_id: u64,
    metadata: String,
) -> Result<Response, ContractError> {
    let mut listing = listings().load(deps.storage, listing_id)?;
    if info.sender != listing.seller {
        return Err(ContractError::Unauthorized {});
    }

    listing.metadata = metadata;
    listings().save(deps.storage, listing_id, &listing)?;

    Ok(Response::new()
        .add_attribute("action", "update_listing_metadata")
        .add_attribute("listing_id", listing_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Listing { id } => to_json_binary(&listings().load(deps.storage, id)?),
        QueryMsg::Listings { start_after, limit } => {
            to_json_binary(&query_listings(deps, start_after, limit)?)
        }
        QueryMsg::ListingsBySeller {
            seller,
            start_after,
            limit,
        } => to_json_binary(&query_listings_by_seller(deps, seller, start_after, limit)?),
        QueryMsg::ListingsByToken {
            token,
            start_after,
            limit,
        } => to_json_binary(&query_listings_by_token(deps, token, start_after, limit)?),
        QueryMsg::BuyerPurchaseHistory {
            buyer,
            start_after,
            limit,
        } => to_json_binary(&query_buyer_purchase_history(
            deps,
            buyer,
            start_after,
            limit,
        )?),
        QueryMsg::Stats {} => to_json_binary(&STATS.load(deps.storage)?),
    }
}

fn query_listings(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Listing>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(cw_storage_plus::Bound::exclusive);

    listings()
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, l)| l))
        .collect()
}

fn query_listings_by_seller(
    deps: Deps,
    seller: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Listing>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let seller_addr = deps.api.addr_validate(&seller)?;
    let start = start_after.map(cw_storage_plus::Bound::exclusive);

    listings()
        .idx
        .seller
        .prefix(seller_addr)
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, l)| l))
        .collect()
}

fn query_listings_by_token(
    deps: Deps,
    token: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Listing>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let token_addr = deps.api.addr_validate(&token)?;
    let start = start_after.map(cw_storage_plus::Bound::exclusive);

    listings()
        .idx
        .token
        .prefix(token_addr)
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, l)| l))
        .collect()
}

fn query_buyer_purchase_history(
    deps: Deps,
    buyer: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Purchase>> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let buyer_addr = deps.api.addr_validate(&buyer)?;

    USER_PURCHASES
        .prefix(buyer_addr)
        .range(
            deps.storage,
            start_after.map(cw_storage_plus::Bound::<u64>::exclusive),
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit)
        .map(|item| item.map(|(_, p)| p))
        .collect()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, RoundConfig};
    use crate::state::{Config, Listing, ListingStatus};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, to_json_binary, Addr, Uint128};
    use cw20::Cw20ReceiveMsg;

    fn setup_contract(deps: cosmwasm_std::DepsMut) {
        let msg = InstantiateMsg {
            admin: "admin".to_string(),
            platform_fee_bps: 100, // 1%
            fee_receiver: "fee_receiver".to_string(),
            native_denom: "upaxi".to_string(),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: Config = from_json(&res).unwrap();
        assert_eq!(config.admin, Addr::unchecked("admin"));
        assert_eq!(config.platform_fee_bps, 100);
    }

    #[test]
    fn test_create_listing_with_rounds() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let rounds = vec![
            RoundConfig {
                name: "Private".to_string(),
                start_time: 100,
                end_time: 200,
                price_per_token: Uint128::from(10u128),
                max_wallet_limit: Some(Uint128::from(100u128)),
                whitelist: Some(vec!["allowed".to_string()]),
            },
            RoundConfig {
                name: "Public".to_string(),
                start_time: 201,
                end_time: 500,
                price_per_token: Uint128::from(20u128),
                max_wallet_limit: None,
                whitelist: None,
            },
        ];

        let hook_msg = Cw20HookMsg::CreateListing {
            min_buy: Some(Uint128::from(10u128)),
            max_buy: None,
            rounds,
            metadata: "test project".to_string(),
            royalty_address: None,
            royalty_bps: None,
        };

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "seller".to_string(),
            amount: Uint128::from(1000u128),
            msg: to_json_binary(&hook_msg).unwrap(),
        });

        let info = mock_info("token_addr", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes[1].value, "1"); // listing_id

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Listing { id: 1 }).unwrap();
        let listing: Listing = from_json(&res).unwrap();
        assert_eq!(listing.rounds.len(), 2);
        assert_eq!(listing.rounds[0].name, "Private");
        assert_eq!(listing.rounds[1].name, "Public");
    }

    #[test]
    fn test_multi_round_trading() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let rounds = vec![
            RoundConfig {
                name: "Private".to_string(),
                start_time: 100,
                end_time: 200,
                price_per_token: Uint128::from(10u128),
                max_wallet_limit: Some(Uint128::from(50u128)),
                whitelist: Some(vec!["buyer1".to_string()]),
            },
            RoundConfig {
                name: "Public".to_string(),
                start_time: 201,
                end_time: 500,
                price_per_token: Uint128::from(20u128),
                max_wallet_limit: None,
                whitelist: None,
            },
        ];

        let hook_msg = Cw20HookMsg::CreateListing {
            min_buy: None,
            max_buy: None,
            rounds,
            metadata: "test".to_string(),
            royalty_address: None,
            royalty_bps: None,
        };
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("token", &[]),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "seller".to_string(),
                amount: Uint128::from(1000u128),
                msg: to_json_binary(&hook_msg).unwrap(),
            }),
        )
        .unwrap();

        // 1. Try buying before rounds start
        let mut env = mock_env();
        env.block.time = cosmwasm_std::Timestamp::from_seconds(50);
        let msg = ExecuteMsg::Buy {
            listing_id: 1,
            amount: Uint128::from(10u128),
            referrer: None,
        };
        let err = execute(deps.as_mut(), env.clone(), mock_info("buyer1", &[]), msg).unwrap_err();
        assert!(err.to_string().contains("No active round found"));

        // 2. Buy in Private Round (buyer1 is whitelisted)
        env.block.time = cosmwasm_std::Timestamp::from_seconds(150);
        let info = mock_info(
            "buyer1",
            &[cosmwasm_std::Coin {
                denom: "upaxi".to_string(),
                amount: Uint128::from(100u128),
            }],
        );
        let msg = ExecuteMsg::Buy {
            listing_id: 1,
            amount: Uint128::from(10u128),
            referrer: None,
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "round")
                .unwrap()
                .value,
            "Private"
        );

        // 3. Try buying in Private Round as non-whitelisted
        let info2 = mock_info(
            "buyer2",
            &[cosmwasm_std::Coin {
                denom: "upaxi".to_string(),
                amount: Uint128::from(100u128),
            }],
        );
        let msg2 = ExecuteMsg::Buy {
            listing_id: 1,
            amount: Uint128::from(10u128),
            referrer: None,
        };
        let err = execute(deps.as_mut(), env.clone(), info2, msg2).unwrap_err();
        assert!(err.to_string().contains("Not on whitelist"));

        // 4. Try exceeding wallet limit in Private Round
        let info3 = mock_info(
            "buyer1",
            &[cosmwasm_std::Coin {
                denom: "upaxi".to_string(),
                amount: Uint128::from(1000u128),
            }],
        );
        let msg3 = ExecuteMsg::Buy {
            listing_id: 1,
            amount: Uint128::from(41u128),
            referrer: None,
        }; // Total 10 + 41 = 51 > 50
        let err = execute(deps.as_mut(), env.clone(), info3, msg3).unwrap_err();
        assert!(err
            .to_string()
            .contains("Wallet limit exceeded for this round"));

        // 5. Buy in Public Round (price is higher)
        env.block.time = cosmwasm_std::Timestamp::from_seconds(300);
        let info4 = mock_info(
            "buyer2",
            &[cosmwasm_std::Coin {
                denom: "upaxi".to_string(),
                amount: Uint128::from(200u128),
            }],
        );
        let msg4 = ExecuteMsg::Buy {
            listing_id: 1,
            amount: Uint128::from(10u128),
            referrer: None,
        }; // 10 * 20 = 200
        let res = execute(deps.as_mut(), env.clone(), info4, msg4).unwrap();
        assert_eq!(
            res.attributes
                .iter()
                .find(|a| a.key == "round")
                .unwrap()
                .value,
            "Public"
        );
    }

    #[test]
    fn test_cancel_listing() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let rounds = vec![RoundConfig {
            name: "Public".to_string(),
            start_time: 0,
            end_time: 1000,
            price_per_token: Uint128::from(10u128),
            max_wallet_limit: None,
            whitelist: None,
        }];
        let hook_msg = Cw20HookMsg::CreateListing {
            min_buy: None,
            max_buy: None,
            rounds,
            metadata: "test".to_string(),
            royalty_address: None,
            royalty_bps: None,
        };
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("token", &[]),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "seller".to_string(),
                amount: Uint128::from(1000u128),
                msg: to_json_binary(&hook_msg).unwrap(),
            }),
        )
        .unwrap();

        let info = mock_info("seller", &[]);
        let msg = ExecuteMsg::CancelListing { listing_id: 1 };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Listing { id: 1 }).unwrap();
        let listing: Listing = from_json(&res).unwrap();
        assert_eq!(listing.status, ListingStatus::Cancelled);
    }

    #[test]
    fn test_indexed_queries() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let rounds = vec![RoundConfig {
            name: "Public".to_string(),
            start_time: 0,
            end_time: 1000,
            price_per_token: Uint128::from(10u128),
            max_wallet_limit: None,
            whitelist: None,
        }];
        let hook_msg1 = Cw20HookMsg::CreateListing {
            min_buy: None,
            max_buy: None,
            rounds,
            metadata: "1".to_string(),
            royalty_address: None,
            royalty_bps: None,
        };
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("token1", &[]),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "seller1".to_string(),
                amount: Uint128::from(100u128),
                msg: to_json_binary(&hook_msg1).unwrap(),
            }),
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("token2", &[]),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "seller1".to_string(),
                amount: Uint128::from(100u128),
                msg: to_json_binary(&hook_msg1).unwrap(),
            }),
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("token1", &[]),
            ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "seller2".to_string(),
                amount: Uint128::from(100u128),
                msg: to_json_binary(&hook_msg1).unwrap(),
            }),
        )
        .unwrap();

        // Query by seller1
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ListingsBySeller {
                seller: "seller1".to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
        let listings: Vec<Listing> = from_json(&res).unwrap();
        assert_eq!(listings.len(), 2);

        // Query by token1
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ListingsByToken {
                token: "token1".to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
        let listings: Vec<Listing> = from_json(&res).unwrap();
        assert_eq!(listings.len(), 2);
    }
}

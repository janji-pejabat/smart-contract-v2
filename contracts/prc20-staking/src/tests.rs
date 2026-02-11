#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json, Addr, Uint128, Decimal, to_json_binary, CosmosMsg, WasmMsg};
    use crate::contract::{instantiate, execute, query};
    use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, RoomResponse, Cw20HookMsg};
    use crate::state::{StakeConfig, AutoCompoundConfig, AssetInfo};
    use cw20::Cw20ReceiveMsg;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin: "admin".to_string(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Create a room
        let create_msg = ExecuteMsg::CreateRoom {
            name: "Room 1".to_string(),
            stake_config: StakeConfig {
                stake_tokens: vec![Addr::unchecked("token1")],
                is_and_rule: false,
                min_stake_amount: Uint128::zero(),
            },
            nft_config: None,
            auto_compound_config: AutoCompoundConfig {
                enabled: true,
                min_stake_threshold: Uint128::zero(),
                nft_required: false,
            },
            early_withdraw_penalty: Decimal::zero(),
            cooldown_period: 0,
        };
        let info = mock_info("admin", &[]);
        execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Query the room
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Room { room_id: 1 }).unwrap();
        let room_res: RoomResponse = from_json(&res).unwrap();
        assert_eq!(room_res.room.name, "Room 1");
    }

    #[test]
    fn test_staking_and_rewards() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Instantiate
        let inst_msg = InstantiateMsg { admin: "admin".to_string() };
        instantiate(deps.as_mut(), env.clone(), mock_info("creator", &[]), inst_msg).unwrap();

        // Create Room
        let create_msg = ExecuteMsg::CreateRoom {
            name: "Room 1".to_string(),
            stake_config: StakeConfig {
                stake_tokens: vec![Addr::unchecked("stake_token")],
                is_and_rule: false,
                min_stake_amount: Uint128::zero(),
            },
            nft_config: None,
            auto_compound_config: AutoCompoundConfig {
                enabled: true,
                min_stake_threshold: Uint128::zero(),
                nft_required: false,
            },
            early_withdraw_penalty: Decimal::zero(),
            cooldown_period: 0,
        };
        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), create_msg).unwrap();

        // Add Reward Pool
        let add_reward_msg = ExecuteMsg::AddRewardPool {
            room_id: 1,
            reward_token: AssetInfo::Cw20(Addr::unchecked("reward_token")),
            emission_per_second: Uint128::from(100u128), // 100 per second
        };
        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), add_reward_msg).unwrap();

        // Fund Reward Pool
        let fund_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "admin".to_string(),
            amount: Uint128::from(100000u128),
            msg: to_json_binary(&Cw20HookMsg::FundPool { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("reward_token", &[]), fund_msg).unwrap();

        // Stake
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user1".to_string(),
            amount: Uint128::from(1000u128),
            msg: to_json_binary(&Cw20HookMsg::Stake { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("stake_token", &[]), stake_msg).unwrap();

        // Advance time by 100 seconds
        let mut env_after = env.clone();
        env_after.block.time = env.block.time.plus_seconds(100);

        // Query pending rewards
        let res = query(deps.as_ref(), env_after.clone(), QueryMsg::PendingRewards {
            room_id: 1,
            user: "user1".to_string(),
        }).unwrap();
        let rewards_res: crate::msg::PendingRewardsResponse = from_json(&res).unwrap();

        // 100 seconds * 100 reward/sec = 10000 rewards
        assert_eq!(rewards_res.rewards[0].1, Uint128::from(10000u128));

        // Claim rewards
        let claim_msg = ExecuteMsg::ClaimRewards { room_id: 1 };
        let res = execute(deps.as_mut(), env_after.clone(), mock_info("user1", &[]), claim_msg).unwrap();

        // Verify transfer message
        let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "reward_token".to_string(),
            msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: "user1".to_string(),
                amount: Uint128::from(10000u128),
            }).unwrap(),
            funds: vec![],
        });
        assert_eq!(res.messages[0].msg, expected_msg);
    }

    #[test]
    fn test_auto_compound() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Instantiate
        let inst_msg = InstantiateMsg { admin: "admin".to_string() };
        instantiate(deps.as_mut(), env.clone(), mock_info("creator", &[]), inst_msg).unwrap();

        // Create Room
        let create_msg = ExecuteMsg::CreateRoom {
            name: "Room 1".to_string(),
            stake_config: StakeConfig {
                stake_tokens: vec![Addr::unchecked("token")],
                is_and_rule: false,
                min_stake_amount: Uint128::zero(),
            },
            nft_config: None,
            auto_compound_config: AutoCompoundConfig {
                enabled: true,
                min_stake_threshold: Uint128::from(500u128), // min stake 500
                nft_required: false,
            },
            early_withdraw_penalty: Decimal::zero(),
            cooldown_period: 0,
        };
        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), create_msg).unwrap();

        // Add Reward Pool (same as stake token)
        let add_reward_msg = ExecuteMsg::AddRewardPool {
            room_id: 1,
            reward_token: AssetInfo::Cw20(Addr::unchecked("token")),
            emission_per_second: Uint128::from(10u128),
        };
        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), add_reward_msg).unwrap();

        // Fund Reward Pool
        let fund_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "admin".to_string(),
            amount: Uint128::from(100000u128),
            msg: to_json_binary(&Cw20HookMsg::FundPool { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("token", &[]), fund_msg).unwrap();

        // Stake 1000
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user1".to_string(),
            amount: Uint128::from(1000u128),
            msg: to_json_binary(&Cw20HookMsg::Stake { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("token", &[]), stake_msg).unwrap();

        // Enable auto-compound
        let toggle_msg = ExecuteMsg::ToggleAutoCompound { room_id: 1, enabled: true };
        execute(deps.as_mut(), env.clone(), mock_info("user1", &[]), toggle_msg).unwrap();

        // Advance time 100s -> 1000 rewards
        let mut env_after = env.clone();
        env_after.block.time = env.block.time.plus_seconds(100);

        // Compound
        let compound_msg = ExecuteMsg::Compound { room_id: 1 };
        execute(deps.as_mut(), env_after.clone(), mock_info("user1", &[]), compound_msg).unwrap();

        // Check user position
        let res = query(deps.as_ref(), env_after.clone(), QueryMsg::UserPosition {
            room_id: 1,
            user: "user1".to_string(),
        }).unwrap();
        let pos_res: crate::msg::UserPositionResponse = from_json(&res).unwrap();
        let pos = pos_res.position.unwrap();

        // Stake should be 1000 + 1000 = 2000
        assert_eq!(pos.staked_amounts[0].1, Uint128::from(2000u128));
    }

    #[test]
    fn test_withdraw_penalty() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Instantiate
        let inst_msg = InstantiateMsg { admin: "admin".to_string() };
        instantiate(deps.as_mut(), env.clone(), mock_info("creator", &[]), inst_msg).unwrap();

        // Create Room with 10% penalty
        let create_msg = ExecuteMsg::CreateRoom {
            name: "Room 1".to_string(),
            stake_config: StakeConfig {
                stake_tokens: vec![Addr::unchecked("token")],
                is_and_rule: false,
                min_stake_amount: Uint128::zero(),
            },
            nft_config: None,
            auto_compound_config: AutoCompoundConfig {
                enabled: false,
                min_stake_threshold: Uint128::zero(),
                nft_required: false,
            },
            early_withdraw_penalty: Decimal::percent(10),
            cooldown_period: 0,
        };
        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), create_msg).unwrap();

        // Stake 1000
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user1".to_string(),
            amount: Uint128::from(1000u128),
            msg: to_json_binary(&Cw20HookMsg::Stake { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("token", &[]), stake_msg).unwrap();

        // Unstake 1000
        let unstake_msg = ExecuteMsg::Unstake {
            room_id: 1,
            amount: Uint128::from(1000u128),
            token_address: "token".to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("user1", &[]), unstake_msg).unwrap();

        // Check penalty (10% of 1000 = 100)
        let penalty_attr = res.attributes.iter().find(|a| a.key == "penalty").unwrap();
        assert_eq!(penalty_attr.value, "100");

        // Check transfer amount (1000 - 100 = 900)
        let msg = &res.messages[0].msg;
        if let CosmosMsg::Wasm(WasmMsg::Execute { msg: bin, .. }) = msg {
            let cw20_msg: cw20::Cw20ExecuteMsg = from_json(bin).unwrap();
            if let cw20::Cw20ExecuteMsg::Transfer { amount, .. } = cw20_msg {
                assert_eq!(amount, Uint128::from(900u128));
            } else { panic!("Wrong cw20 msg"); }
        } else { panic!("Wrong msg type"); }
    }

    #[test]
    fn test_and_rule() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(deps.as_mut(), env.clone(), mock_info("creator", &[]), InstantiateMsg { admin: "admin".to_string() }).unwrap();

        // Create Room with AND rule (TokenA AND TokenB)
        let create_msg = ExecuteMsg::CreateRoom {
            name: "Partner Room".to_string(),
            stake_config: StakeConfig {
                stake_tokens: vec![Addr::unchecked("tokenA"), Addr::unchecked("tokenB")],
                is_and_rule: true,
                min_stake_amount: Uint128::from(100u128),
            },
            nft_config: None,
            auto_compound_config: AutoCompoundConfig { enabled: false, min_stake_threshold: Uint128::zero(), nft_required: false },
            early_withdraw_penalty: Decimal::zero(),
            cooldown_period: 0,
        };
        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), create_msg).unwrap();

        // Stake only TokenA -> Should succeed but total_staked_weight remains 0
        let stake_a_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_json_binary(&Cw20HookMsg::Stake { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("tokenA", &[]), stake_a_msg).unwrap();

        let res = query(deps.as_ref(), env.clone(), QueryMsg::Room { room_id: 1 }).unwrap();
        let room_res: RoomResponse = from_json(&res).unwrap();
        assert_eq!(room_res.room.total_staked_weight, Uint128::zero());

        // Stake TokenB as well -> Should succeed and total_staked_weight should be 200
        let stake_b_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_json_binary(&Cw20HookMsg::Stake { room_id: 1 }).unwrap(),
        });
        execute(deps.as_mut(), env.clone(), mock_info("tokenB", &[]), stake_b_msg).unwrap();

        let res = query(deps.as_ref(), env.clone(), QueryMsg::Room { room_id: 1 }).unwrap();
        let room_res: RoomResponse = from_json(&res).unwrap();
        assert_eq!(room_res.room.total_staked_weight, Uint128::from(200u128));
    }

    #[test]
    fn test_reward_solvency() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(deps.as_mut(), env.clone(), mock_info("creator", &[]), InstantiateMsg { admin: "admin".to_string() }).unwrap();

        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), ExecuteMsg::CreateRoom {
            name: "Room".to_string(),
            stake_config: StakeConfig { stake_tokens: vec![Addr::unchecked("token")], is_and_rule: false, min_stake_amount: Uint128::zero() },
            nft_config: None,
            auto_compound_config: AutoCompoundConfig { enabled: false, min_stake_threshold: Uint128::zero(), nft_required: false },
            early_withdraw_penalty: Decimal::zero(),
            cooldown_period: 0,
        }).unwrap();

        execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), ExecuteMsg::AddRewardPool {
            room_id: 1,
            reward_token: AssetInfo::Cw20(Addr::unchecked("reward")),
            emission_per_second: Uint128::from(100u128),
        }).unwrap();

        // Fund only 500 rewards
        execute(deps.as_mut(), env.clone(), mock_info("reward", &[]), ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "admin".to_string(),
            amount: Uint128::from(500u128),
            msg: to_json_binary(&Cw20HookMsg::FundPool { room_id: 1 }).unwrap(),
        })).unwrap();

        // Stake
        execute(deps.as_mut(), env.clone(), mock_info("token", &[]), ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user1".to_string(),
            amount: Uint128::from(1000u128),
            msg: to_json_binary(&Cw20HookMsg::Stake { room_id: 1 }).unwrap(),
        })).unwrap();

        // Advance 10 seconds (expected 1000 rewards, but only 500 available)
        let mut env_after = env.clone();
        env_after.block.time = env.block.time.plus_seconds(10);

        let res = query(deps.as_ref(), env_after, QueryMsg::PendingRewards { room_id: 1, user: "user1".to_string() }).unwrap();
        let rewards: crate::msg::PendingRewardsResponse = from_json(&res).unwrap();

        assert_eq!(rewards.rewards[0].1, Uint128::from(500u128));
    }
}

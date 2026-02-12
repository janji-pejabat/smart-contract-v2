#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate};
    use crate::error::ContractError;
    use crate::msg::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_json_binary, Decimal, Uint128};
    use cw20::Cw20ReceiveMsg;

    #[test]
    fn test_platform_fees() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            InstantiateMsg {
                admin: "admin".to_string(),
                emergency_unlock_delay: 100,
            },
        )
        .unwrap();

        // Update fee to 1% (100 bps)
        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::UpdateConfig {
                admin: None,
                reward_controller: None,
                emergency_unlock_delay: None,
                platform_fee_bps: Some(100),
                batch_limit: None,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::WhitelistLP {
                lp_token: "lp_token".to_string(),
                name: "LP Token".to_string(),
                symbol: "LPT".to_string(),
                min_lock_duration: 10,
                max_lock_duration: 1000,
                bonus_multiplier: Decimal::one(),
            },
        )
        .unwrap();

        // Lock 1000 LP
        let lock_hook = Cw20HookMsg::LockLP {
            unlock_time: env.block.time.seconds() + 100,
            metadata: None,
        };
        let receive_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "user".to_string(),
            amount: Uint128::new(1000),
            msg: to_json_binary(&lock_hook).unwrap(),
        });
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("lp_token", &[]),
            receive_msg,
        )
        .unwrap();

        // Amount locked should be 990
        assert_eq!(res.attributes[4].value, "990");

        // Unlock
        let mut env = env;
        env.block.time = env.block.time.plus_seconds(101);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            ExecuteMsg::UnlockLP { locker_id: 0 },
        )
        .unwrap();

        // Unlock Attr 3: amount = 981
        assert_eq!(res.attributes[3].value, "981");
    }

    #[test]
    fn test_batch_operations() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let admin_info = mock_info("admin", &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            InstantiateMsg {
                admin: "admin".to_string(),
                emergency_unlock_delay: 100,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::WhitelistLP {
                lp_token: "lp_token".to_string(),
                name: "LP Token".to_string(),
                symbol: "LPT".to_string(),
                min_lock_duration: 10,
                max_lock_duration: 1000,
                bonus_multiplier: Decimal::one(),
            },
        )
        .unwrap();

        // Lock 2 lockers
        for _ in 0..2 {
            let lock_hook = Cw20HookMsg::LockLP {
                unlock_time: env.block.time.seconds() + 100,
                metadata: None,
            };
            let receive_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
                sender: "user".to_string(),
                amount: Uint128::new(1000),
                msg: to_json_binary(&lock_hook).unwrap(),
            });
            execute(
                deps.as_mut(),
                env.clone(),
                mock_info("lp_token", &[]),
                receive_msg,
            )
            .unwrap();
        }

        // Batch Extend
        env.block.time = env.block.time.plus_seconds(10);
        let new_time = env.block.time.seconds() + 200;
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            ExecuteMsg::BatchExtendLock {
                locks: vec![(0, new_time), (1, new_time)],
            },
        )
        .unwrap();
        assert_eq!(res.attributes[0].value, "batch_extend_lock");

        // Batch Unlock (should fail - too early)
        env.block.time = env.block.time.plus_seconds(10);
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            ExecuteMsg::BatchUnlock {
                locker_ids: vec![0, 1],
            },
        )
        .unwrap_err();

        // Batch Unlock (should pass)
        env.block.time = env.block.time.plus_seconds(300);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            ExecuteMsg::BatchUnlock {
                locker_ids: vec![0, 1],
            },
        )
        .unwrap();
        assert_eq!(res.attributes[0].value, "batch_unlock");
    }

    #[test]
    fn test_fee_cap() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            InstantiateMsg {
                admin: "admin".to_string(),
                emergency_unlock_delay: 100,
            },
        )
        .unwrap();

        // Try to set fee to 6% (600 bps), which is above the 500 bps cap
        let err = execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::UpdateConfig {
                admin: None,
                reward_controller: None,
                emergency_unlock_delay: None,
                platform_fee_bps: Some(600),
                batch_limit: None,
            },
        )
        .unwrap_err();

        assert_eq!(err, ContractError::FeeTooHigh(500));

        // Setting to 5% (500 bps) should work
        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::UpdateConfig {
                admin: None,
                reward_controller: None,
                emergency_unlock_delay: None,
                platform_fee_bps: Some(500),
                batch_limit: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_migration_v1_to_v2() {
        use cosmwasm_std::{Addr, Decimal};
        use cw_storage_plus::{Item, Map};
        use crate::msg::{MigrateMsg, QueryMsg, ConfigResponse, WhitelistedLPResponse};
        use crate::contract::{query, migrate};

        let mut deps = mock_dependencies();
        let env = mock_env();

        // 1. Manually set V1 state
        #[derive(serde::Serialize, serde::Deserialize)]
        struct ConfigV1 {
            pub admin: Addr,
            pub reward_controller: Option<Addr>,
            pub emergency_unlock_delay: u64,
            pub platform_fee_bps: u16,
            pub paused: bool,
            pub next_locker_id: u64,
        }

        let v1_config = ConfigV1 {
            admin: Addr::unchecked("admin"),
            reward_controller: None,
            emergency_unlock_delay: 100,
            platform_fee_bps: 0,
            paused: false,
            next_locker_id: 10,
        };

        let config_item: Item<ConfigV1> = Item::new("config");
        config_item.save(deps.as_mut().storage, &v1_config).unwrap();

        #[derive(serde::Serialize, serde::Deserialize)]
        struct WhitelistedLPV1 {
            pub lp_token: Addr,
            pub min_lock_duration: u64,
            pub max_lock_duration: u64,
            pub enabled: bool,
            pub bonus_multiplier: Decimal,
        }

        let lp_v1 = WhitelistedLPV1 {
            lp_token: Addr::unchecked("lp_token"),
            min_lock_duration: 10,
            max_lock_duration: 1000,
            enabled: true,
            bonus_multiplier: Decimal::one(),
        };

        let lp_map: Map<&Addr, WhitelistedLPV1> = Map::new("whitelisted_lps");
        lp_map.save(deps.as_mut().storage, &Addr::unchecked("lp_token"), &lp_v1).unwrap();

        cw2::set_contract_version(deps.as_mut().storage, "crates.io:lp-locker", "1.0.0").unwrap();

        // 2. Run Migration
        migrate(deps.as_mut(), env.clone(), MigrateMsg::V1ToV2 { reward_controller: Some("new_reward".to_string()) }).unwrap();

        // 3. Verify V2 state via queries
        let config: ConfigResponse = cosmwasm_std::from_json(&query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.batch_limit, 20);
        assert_eq!(config.reward_controller.unwrap(), Addr::unchecked("new_reward"));

        let lp: WhitelistedLPResponse = cosmwasm_std::from_json(&query(deps.as_ref(), env.clone(), QueryMsg::WhitelistedLP { lp_token: "lp_token".to_string() }).unwrap()).unwrap();
        // Since we didn't mock the CW20 query, it should fall back to "Unknown LP"
        assert_eq!(lp.name, "Unknown LP");
    }
}

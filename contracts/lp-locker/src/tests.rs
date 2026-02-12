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
}

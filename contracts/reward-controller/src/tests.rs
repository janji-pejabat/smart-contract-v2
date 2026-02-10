#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{
        ExecuteMsg, InstantiateMsg, LockerHookMsg, PendingRewardsResponse, QueryMsg,
        UserStakeResponse,
    };
    use crate::state::AssetInfo;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Decimal, Uint128};

    #[test]
    fn test_reward_accrual_flow() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let admin_info = mock_info("admin", &[]);

        // Instantiate
        instantiate(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            InstantiateMsg {
                admin: "admin".to_string(),
                lp_locker_contract: "locker".to_string(),
                claim_interval: Some(0),
            },
        )
        .unwrap();

        // Create Pool: 10% APR for LP_TOKEN, reward is REWARD_TOKEN
        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::CreateRewardPool {
                lp_token: "lp_token".to_string(),
                reward_token: AssetInfo::Native("paxi".to_string()),
                apr: Decimal::percent(10),
            },
        )
        .unwrap();

        // Admin deposits rewards
        let deposit_info = mock_info(
            "admin",
            &[cosmwasm_std::Coin {
                denom: "paxi".to_string(),
                amount: Uint128::new(1000000),
            }],
        );
        execute(
            deps.as_mut(),
            env.clone(),
            deposit_info,
            ExecuteMsg::DepositRewards { pool_id: 0 },
        )
        .unwrap();

        // Mock Lock hook: User locks 1000 LP for 1 year (multiplier should be 2.5x)
        let lock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 1,
            owner: "user".to_string(),
            lp_token: "lp_token".to_string(),
            amount: Uint128::new(1000),
            locked_at: env.block.time.seconds(),
            unlock_time: env.block.time.seconds() + 365 * 86400,
        });
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("locker", &[]),
            lock_hook,
        )
        .unwrap();

        // Advance time by 1 year
        env.block.time = env.block.time.plus_seconds(365 * 86400);

        // Calculate expected rewards:
        // effective_amount = 1000 * 2.5 = 2500
        // reward = 2500 * 10% APR * 1 year = 250

        let res: PendingRewardsResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::PendingRewards {
                    user: "user".to_string(),
                    pool_id: 0,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert!(
            res.pending_amount.u128() >= 249 && res.pending_amount.u128() <= 251,
            "Expected ~250, got {}",
            res.pending_amount
        );

        // Claim
        let claim_msg = ExecuteMsg::ClaimRewards {
            locker_id: 1,
            pool_ids: vec![0],
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            claim_msg,
        )
        .unwrap();

        assert_eq!(res.attributes[0].value, "claim_rewards");
        assert_eq!(res.attributes[1].value, "250");

        // Advance time by another year
        env.block.time = env.block.time.plus_seconds(365 * 86400);

        // Unlock hook - should auto claim
        let unlock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnUnlock {
            locker_id: 1,
            owner: "user".to_string(),
        });
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("locker", &[]),
            unlock_hook,
        )
        .unwrap();

        assert_eq!(res.attributes[0].value, "locker_hook");
        assert_eq!(res.attributes[1].value, "on_unlock_auto_claim");

        // Verify user stake is removed
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::UserStake {
                user: "user".to_string(),
                locker_id: 1,
            },
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_multiplier_extension() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let admin_info = mock_info("admin", &[]);

        instantiate(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            InstantiateMsg {
                admin: "admin".to_string(),
                lp_locker_contract: "locker".to_string(),
                claim_interval: Some(0),
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::CreateRewardPool {
                lp_token: "lp_token".to_string(),
                reward_token: AssetInfo::Native("paxi".to_string()),
                apr: Decimal::percent(10),
            },
        )
        .unwrap();

        // Lock for 40 days (1.2x multiplier)
        let locked_at = env.block.time.seconds();
        let unlock_time = locked_at + 40 * 86400;
        let lock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 1,
            owner: "user".to_string(),
            lp_token: "lp_token".to_string(),
            amount: Uint128::new(1000),
            locked_at,
            unlock_time,
        });
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("locker", &[]),
            lock_hook,
        )
        .unwrap();

        let stake: UserStakeResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::UserStake {
                    user: "user".to_string(),
                    locker_id: 1,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(stake.bonus_multiplier, Decimal::from_ratio(12u128, 10u128));

        // Advance time by 35 days (only 5 days remaining)
        env.block.time = env.block.time.plus_seconds(35 * 86400);

        // Extend lock by another 60 days (total 100 days from start -> 1.5x multiplier)
        let new_unlock_time = unlock_time + 60 * 86400;
        let extend_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnExtend {
            locker_id: 1,
            new_unlock_time,
        });
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("locker", &[]),
            extend_hook,
        )
        .unwrap();

        let stake: UserStakeResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::UserStake {
                    user: "user".to_string(),
                    locker_id: 1,
                },
            )
            .unwrap(),
        )
        .unwrap();

        // Should be 1.5x (total 100 days), not 1.2x (remaining 65 days) or 1.0x (remaining 5 days before extension)
        assert_eq!(stake.bonus_multiplier, Decimal::from_ratio(15u128, 10u128));
    }
}

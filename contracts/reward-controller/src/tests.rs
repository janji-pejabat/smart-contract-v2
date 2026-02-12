#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{
        ExecuteMsg, InstantiateMsg, LockerHookMsg, PendingRewardsResponse, QueryMsg,
        ReferrerBalancesResponse, RewardPoolResponse, UserStakeResponse,
    };
    use crate::state::{AssetInfo, DynamicAPRConfig};
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
                dynamic_config: None,
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
                dynamic_config: None,
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

    #[test]
    fn test_referral_system() {
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
                dynamic_config: None,
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

        // Register Referral: User B referred by User A
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("userb", &[]),
            ExecuteMsg::RegisterReferral {
                referrer: "usera".to_string(),
            },
        )
        .unwrap();

        // Lock for User B
        let lock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 1,
            owner: "userb".to_string(),
            lp_token: "lp_token".to_string(),
            amount: Uint128::new(100000), // Larger amount for better precision
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

        // Advance 1 year
        env.block.time = env.block.time.plus_seconds(365 * 86400);

        // Expected reward for User B: 100,000 * 2.5 * 0.1 = 25,000
        // Commission for User A (5%): 25,000 * 0.05 = 1,250
        // Net for User B: 25,000 - 1,250 = 23,750

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("userb", &[]),
            ExecuteMsg::ClaimRewards {
                locker_id: 1,
                pool_ids: vec![0],
            },
        )
        .unwrap();

        // Check User A balance
        let res: ReferrerBalancesResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::ReferrerBalances {
                    referrer: "usera".to_string(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res.balances[0].amount.u128(), 1250);

        // User A claims referral rewards
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("usera", &[]),
            ExecuteMsg::ClaimReferralRewards {},
        )
        .unwrap();
        assert_eq!(res.attributes[0].value, "claim_referral_rewards");
    }

    #[test]
    fn test_dynamic_apr() {
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

        // Base APR 10%, boost +5% if TVL < 5000, reduce -5% if TVL > 15000
        let dynamic_config = DynamicAPRConfig {
            base_apr: Decimal::percent(10),
            tvl_threshold_low: Uint128::new(5000),
            tvl_threshold_high: Uint128::new(15000),
            adjustment_factor: Decimal::percent(5),
        };

        execute(
            deps.as_mut(),
            env.clone(),
            admin_info.clone(),
            ExecuteMsg::CreateRewardPool {
                lp_token: "lp_token".to_string(),
                reward_token: AssetInfo::Native("paxi".to_string()),
                apr: Decimal::percent(10),
                dynamic_config: Some(dynamic_config),
            },
        )
        .unwrap();

        // Deposit rewards
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

        // 1. TVL is 0 (below low threshold) -> APR should be 15%
        let lock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 1,
            owner: "user1".to_string(),
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

        let pool: RewardPoolResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::RewardPool { pool_id: 0 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(pool.apr, Decimal::percent(15));

        // 2. Add more TVL to get between thresholds (1000 + 9000 = 10000) -> APR should be 10%
        let lock_hook2 = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 2,
            owner: "user2".to_string(),
            lp_token: "lp_token".to_string(),
            amount: Uint128::new(9000),
            locked_at: env.block.time.seconds(),
            unlock_time: env.block.time.seconds() + 365 * 86400,
        });
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("locker", &[]),
            lock_hook2,
        )
        .unwrap();

        // Advance time to allow some rewards to accrue
        env.block.time = env.block.time.plus_seconds(864000);

        // Trigger update via a claim
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user1", &[]),
            ExecuteMsg::ClaimRewards {
                locker_id: 1,
                pool_ids: vec![0],
            },
        )
        .unwrap();

        let pool: RewardPoolResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::RewardPool { pool_id: 0 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(pool.apr, Decimal::percent(10));

        // 3. Add even more TVL to exceed high threshold (10000 + 10000 = 20000) -> APR should be 5%
        let lock_hook3 = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 3,
            owner: "user3".to_string(),
            lp_token: "lp_token".to_string(),
            amount: Uint128::new(10000),
            locked_at: env.block.time.seconds(),
            unlock_time: env.block.time.seconds() + 365 * 86400,
        });
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("locker", &[]),
            lock_hook3,
        )
        .unwrap();

        // Advance time
        env.block.time = env.block.time.plus_seconds(864000);

        // Trigger update
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user1", &[]),
            ExecuteMsg::ClaimRewards {
                locker_id: 1,
                pool_ids: vec![0],
            },
        )
        .unwrap();

        let pool: RewardPoolResponse = cosmwasm_std::from_json(
            &query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::RewardPool { pool_id: 0 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(pool.apr, Decimal::percent(5));
    }

    #[test]
    fn test_migration_v1_to_v2() {
        use cosmwasm_std::Addr;
        use cw_storage_plus::Item;
        use crate::msg::{MigrateMsg, QueryMsg, ConfigResponse};
        use crate::contract::{query, migrate};

        let mut deps = mock_dependencies();
        let env = mock_env();

        // 1. Set V1 state
        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct RewardConfigV1 {
            pub admin: Addr,
            pub lp_locker_contract: Addr,
            pub paused: bool,
            pub claim_interval: u64,
            pub next_pool_id: u64,
        }

        let v1_config = RewardConfigV1 {
            admin: Addr::unchecked("admin"),
            lp_locker_contract: Addr::unchecked("locker"),
            paused: false,
            claim_interval: 3600,
            next_pool_id: 1,
        };
        let config_item: Item<RewardConfigV1> = Item::new("config");
        config_item.save(deps.as_mut().storage, &v1_config).unwrap();

        cw2::set_contract_version(deps.as_mut().storage, "crates.io:reward-controller", "1.0.0").unwrap();

        // 2. Run Migration
        migrate(deps.as_mut(), env.clone(), MigrateMsg::V1ToV2 {}).unwrap();

        // 3. Verify V2 state
        let config: ConfigResponse = cosmwasm_std::from_json(&query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.referral_commission_bps, 500);
        assert_eq!(config.batch_limit, 20);
    }

    #[test]
    fn test_robust_hooks_and_register_stake() {
        use cosmwasm_std::{Addr, Uint128, to_json_binary};
        use crate::msg::{ExecuteMsg, LockerHookMsg, QueryMsg, UserStakeResponse};
        use crate::contract::{execute, instantiate, query};

        let mut deps = mock_dependencies();
        let env = mock_env();

        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info("admin", &[]),
            InstantiateMsg {
                admin: "admin".to_string(),
                lp_locker_contract: "locker".to_string(),
                claim_interval: None,
            },
        ).unwrap();

        // 1. Test OnUnlock with missing stake (should NOT fail)
        let unlock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnUnlock {
            locker_id: 99,
            owner: "user".to_string(),
        });
        let res = execute(deps.as_mut(), env.clone(), mock_info("locker", &[]), unlock_hook).unwrap();
        assert_eq!(res.attributes[1].value, "on_unlock_skipped");

        // 2. Test RegisterStake
        // Mock the querier for LP Locker
        #[cosmwasm_schema::cw_serde]
        pub struct MockLockerResponse {
            pub owner: Addr,
            pub lp_token: Addr,
            pub amount: Uint128,
            pub locked_at: u64,
            pub unlock_time: u64,
        }

        deps.querier.update_wasm(|_query| {
            cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(to_json_binary(&MockLockerResponse {
                owner: Addr::unchecked("user"),
                lp_token: Addr::unchecked("lp_token"),
                amount: Uint128::new(1000),
                locked_at: 1000,
                unlock_time: 2000,
            }).unwrap()))
        });

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("user", &[]),
            ExecuteMsg::RegisterStake { locker_id: 1 },
        ).unwrap();

        // Verify stake
        let res: UserStakeResponse = cosmwasm_std::from_json(&query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::UserStake { user: "user".to_string(), locker_id: 1 }
        ).unwrap()).unwrap();
        assert_eq!(res.lp_amount, Uint128::new(1000));
    }
}

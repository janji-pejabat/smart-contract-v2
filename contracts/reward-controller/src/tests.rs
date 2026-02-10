#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, LockerHookMsg, PendingRewardsResponse};
    use crate::state::AssetInfo;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Decimal, Uint128};

    #[test]
    fn test_reward_accrual_flow() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let admin_info = mock_info("admin", &[]);

        // Instantiate
        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), InstantiateMsg {
            admin: "admin".to_string(),
            lp_locker_contract: "locker".to_string(),
            claim_interval: Some(0),
        }).unwrap();

        // Create Pool: 10% APR for LP_TOKEN, reward is REWARD_TOKEN
        execute(deps.as_mut(), env.clone(), admin_info.clone(), ExecuteMsg::CreateRewardPool {
            lp_token: "lp_token".to_string(),
            reward_token: AssetInfo::Native("paxi".to_string()),
            apr: Decimal::percent(10),
        }).unwrap();

        // Admin deposits rewards
        let deposit_info = mock_info("admin", &[cosmwasm_std::Coin { denom: "paxi".to_string(), amount: Uint128::new(1000000) }]);
        execute(deps.as_mut(), env.clone(), deposit_info, ExecuteMsg::DepositRewards { pool_id: 0 }).unwrap();

        // Mock Lock hook: User locks 1000 LP for 1 year (multiplier should be 2.5x)
        let lock_hook = ExecuteMsg::LockerHook(LockerHookMsg::OnLock {
            locker_id: 1,
            owner: "user".to_string(),
            lp_token: "lp_token".to_string(),
            amount: Uint128::new(1000),
            unlock_time: env.block.time.seconds() + 365 * 86400,
        });
        execute(deps.as_mut(), env.clone(), mock_info("locker", &[]), lock_hook).unwrap();

        // Advance time by 1 year
        env.block.time = env.block.time.plus_seconds(365 * 86400);

        // Calculate expected rewards:
        // effective_amount = 1000 * 2.5 = 2500
        // reward = 2500 * 10% APR * 1 year = 250

        let res: PendingRewardsResponse = cosmwasm_std::from_json(
            &query(deps.as_ref(), env.clone(), QueryMsg::PendingRewards { user: "user".to_string(), pool_id: 0 }).unwrap()
        ).unwrap();

        assert!(res.pending_amount.u128() >= 249 && res.pending_amount.u128() <= 251, "Expected ~250, got {}", res.pending_amount);

        // Claim
        let claim_msg = ExecuteMsg::ClaimRewards {
            locker_id: 1,
            pool_ids: vec![0],
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info("user", &[]), claim_msg).unwrap();

        assert_eq!(res.attributes[0].value, "claim_rewards");
        assert_eq!(res.attributes[1].value, "250");
    }
}

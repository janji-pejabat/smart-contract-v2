#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate};
    use crate::msg::{ExecuteMsg, InstantiateMsg, Cw20HookMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_json_binary, Decimal, Uint128};
    use cw20::Cw20ReceiveMsg;

    #[test]
    fn test_platform_fees() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let admin_info = mock_info("admin", &[]);

        instantiate(deps.as_mut(), env.clone(), admin_info.clone(), InstantiateMsg {
            admin: "admin".to_string(),
            emergency_unlock_delay: 100,
        }).unwrap();

        // Update fee to 1% (100 bps)
        execute(deps.as_mut(), env.clone(), admin_info.clone(), ExecuteMsg::UpdateConfig {
            admin: None,
            reward_controller: None,
            emergency_unlock_delay: None,
            platform_fee_bps: Some(100),
        }).unwrap();

        execute(deps.as_mut(), env.clone(), admin_info.clone(), ExecuteMsg::WhitelistLP {
            lp_token: "lp_token".to_string(),
            min_lock_duration: 10,
            max_lock_duration: 1000,
            bonus_multiplier: Decimal::one(),
        }).unwrap();

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
        let res = execute(deps.as_mut(), env.clone(), mock_info("lp_token", &[]), receive_msg).unwrap();

        // Amount locked should be 990
        assert_eq!(res.attributes[4].value, "990");

        // Unlock
        let mut env = env;
        env.block.time = env.block.time.plus_seconds(101);
        let res = execute(deps.as_mut(), env.clone(), mock_info("user", &[]), ExecuteMsg::UnlockLP { locker_id: 0 }).unwrap();

        // Unlock Attr 3: amount = 981
        assert_eq!(res.attributes[3].value, "981");
    }
}

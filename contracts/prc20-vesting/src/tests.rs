use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_json, to_json_binary, Uint128, WasmMsg, CosmosMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::contract::{execute, instantiate, query};
use crate::msg::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, VestingResponse, VestingSchedule,
    VestingCreation, Milestone,
};

#[test]
fn test_instantiate() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        admin: "admin".to_string(),
    };
    let info = mock_info("creator", &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_create_linear_vesting() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        admin: "admin".to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();

    let hook_msg = Cw20HookMsg::CreateVesting {
        beneficiary: "beneficiary".to_string(),
        schedule: VestingSchedule::Linear {
            start_time: 1000,
            end_time: 2000,
            cliff_time: Some(1500),
            release_interval: 1,
        },
        category: "team".to_string(),
        revocable: true,
    };

    let receive_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(1000),
        msg: to_json_binary(&hook_msg).unwrap(),
    });

    let info = mock_info("token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, receive_msg).unwrap();
    assert_eq!(res.attributes[1].value, "1"); // ID

    // Query vesting
    let q_res = query(deps.as_ref(), mock_env(), QueryMsg::Vesting { id: 1 }).unwrap();
    let v_res: VestingResponse = from_json(&q_res).unwrap();
    assert_eq!(v_res.total_amount, Uint128::new(1000));
    assert_eq!(v_res.beneficiary, "beneficiary");
}

#[test]
fn test_vesting_math() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        admin: "admin".to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();

    let hook_msg = Cw20HookMsg::CreateVesting {
        beneficiary: "beneficiary".to_string(),
        schedule: VestingSchedule::Linear {
            start_time: 1000,
            end_time: 2000,
            cliff_time: Some(1500),
            release_interval: 1,
        },
        category: "team".to_string(),
        revocable: true,
    };

    let info = mock_info("token", &[]);
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(1000),
        msg: to_json_binary(&hook_msg).unwrap(),
    })).unwrap();

    // T = 1499 (before cliff)
    let mut env = mock_env();
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1499);
    let q_res = query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap();
    let amount: Uint128 = from_json(&q_res).unwrap();
    assert_eq!(amount, Uint128::zero());

    // T = 1500 (at cliff)
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1500);
    let q_res = query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap();
    let amount: Uint128 = from_json(&q_res).unwrap();
    // (1500-1000)/(2000-1000) * 1000 = 500
    assert_eq!(amount, Uint128::new(500));

    // T = 1750
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1750);
    let q_res = query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap();
    let amount: Uint128 = from_json(&q_res).unwrap();
    assert_eq!(amount, Uint128::new(750));

    // T = 2000
    env.block.time = cosmwasm_std::Timestamp::from_seconds(2000);
    let q_res = query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap();
    let amount: Uint128 = from_json(&q_res).unwrap();
    assert_eq!(amount, Uint128::new(1000));
}

#[test]
fn test_claim() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        admin: "admin".to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();

    let hook_msg = Cw20HookMsg::CreateVesting {
        beneficiary: "beneficiary".to_string(),
        schedule: VestingSchedule::Linear {
            start_time: 1000,
            end_time: 2000,
            cliff_time: None,
            release_interval: 1,
        },
        category: "team".to_string(),
        revocable: true,
    };

    execute(deps.as_mut(), mock_env(), mock_info("token", &[]), ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(1000),
        msg: to_json_binary(&hook_msg).unwrap(),
    })).unwrap();

    let mut env = mock_env();
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1500);

    let res = execute(deps.as_mut(), env.clone(), mock_info("beneficiary", &[]), ExecuteMsg::Claim {
        ids: vec![1],
    }).unwrap();

    assert_eq!(res.messages.len(), 1);
    if let CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, msg, .. }) = &res.messages[0].msg {
        assert_eq!(contract_addr, "token");
        let cw20_exec: Cw20ExecuteMsg = from_json(msg).unwrap();
        match cw20_exec {
            Cw20ExecuteMsg::Transfer { recipient, amount } => {
                assert_eq!(recipient, "beneficiary");
                assert_eq!(amount, Uint128::new(500));
            }
            _ => panic!("Wrong CW20 msg"),
        }
    }

    // Second claim at T=1600
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1600);
    let res = execute(deps.as_mut(), env.clone(), mock_info("beneficiary", &[]), ExecuteMsg::Claim {
        ids: vec![1],
    }).unwrap();
    assert_eq!(res.messages.len(), 1);
    if let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &res.messages[0].msg {
        let cw20_exec: Cw20ExecuteMsg = from_json(msg).unwrap();
        if let Cw20ExecuteMsg::Transfer { amount, .. } = cw20_exec {
            assert_eq!(amount, Uint128::new(100)); // 600 total vested - 500 already claimed
        }
    }
}

#[test]
fn test_revoke() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        admin: "admin".to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();

    let hook_msg = Cw20HookMsg::CreateVesting {
        beneficiary: "beneficiary".to_string(),
        schedule: VestingSchedule::Linear {
            start_time: 1000,
            end_time: 2000,
            cliff_time: None,
            release_interval: 1,
        },
        category: "team".to_string(),
        revocable: true,
    };

    execute(deps.as_mut(), mock_env(), mock_info("token", &[]), ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(1000),
        msg: to_json_binary(&hook_msg).unwrap(),
    })).unwrap();

    let mut env = mock_env();
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1500);

    // Revoke by admin
    let res = execute(deps.as_mut(), env.clone(), mock_info("admin", &[]), ExecuteMsg::Revoke {
        id: 1,
    }).unwrap();

    // Should return 500 to admin
    assert_eq!(res.messages.len(), 1);
    if let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &res.messages[0].msg {
        let cw20_exec: Cw20ExecuteMsg = from_json(msg).unwrap();
        if let Cw20ExecuteMsg::Transfer { recipient, amount } = cw20_exec {
            assert_eq!(recipient, "admin");
            assert_eq!(amount, Uint128::new(500));
        }
    }

    // Vesting total_amount should now be 500
    let q_res = query(deps.as_ref(), env.clone(), QueryMsg::Vesting { id: 1 }).unwrap();
    let v_res: VestingResponse = from_json(&q_res).unwrap();
    assert_eq!(v_res.total_amount, Uint128::new(500));
    assert_eq!(v_res.revoked, true);

    // Beneficiary can still claim what was vested
    let res = execute(deps.as_mut(), env.clone(), mock_info("beneficiary", &[]), ExecuteMsg::Claim {
        ids: vec![1],
    }).unwrap();
    assert_eq!(res.messages.len(), 1);
}

#[test]
fn test_custom_schedule() {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        admin: "admin".to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();

    let milestones = vec![
        Milestone { timestamp: 1000, amount: Uint128::new(200) },
        Milestone { timestamp: 2000, amount: Uint128::new(300) },
        Milestone { timestamp: 3000, amount: Uint128::new(500) },
    ];

    let hook_msg = Cw20HookMsg::CreateVesting {
        beneficiary: "beneficiary".to_string(),
        schedule: VestingSchedule::Custom { milestones },
        category: "investor".to_string(),
        revocable: false,
    };

    execute(deps.as_mut(), mock_env(), mock_info("token", &[]), ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(1000),
        msg: to_json_binary(&hook_msg).unwrap(),
    })).unwrap();

    let mut env = mock_env();

    // T=1500
    env.block.time = cosmwasm_std::Timestamp::from_seconds(1500);
    let amount: Uint128 = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap()).unwrap();
    assert_eq!(amount, Uint128::new(200));

    // T=2500
    env.block.time = cosmwasm_std::Timestamp::from_seconds(2500);
    let amount: Uint128 = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap()).unwrap();
    assert_eq!(amount, Uint128::new(500));

    // T=3500
    env.block.time = cosmwasm_std::Timestamp::from_seconds(3500);
    let amount: Uint128 = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::ClaimableAmount { id: 1 }).unwrap()).unwrap();
    assert_eq!(amount, Uint128::new(1000));
}

#[test]
fn test_batch_create() {
    let mut deps = mock_dependencies();
    instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), InstantiateMsg { admin: "admin".to_string() }).unwrap();

    let vestings = vec![
        VestingCreation {
            beneficiary: "beneficiary1".to_string(),
            amount: Uint128::new(100),
            schedule: VestingSchedule::Linear { start_time: 1000, end_time: 2000, cliff_time: None, release_interval: 1 },
            category: "cat".to_string(),
            revocable: true,
        },
        VestingCreation {
            beneficiary: "beneficiary2".to_string(),
            amount: Uint128::new(200),
            schedule: VestingSchedule::Linear { start_time: 1000, end_time: 2000, cliff_time: None, release_interval: 1 },
            category: "cat".to_string(),
            revocable: true,
        },
    ];

    let hook_msg = Cw20HookMsg::BatchCreateVesting { vestings };

    let info = mock_info("token", &[]);
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: "owner".to_string(),
        amount: Uint128::new(300),
        msg: to_json_binary(&hook_msg).unwrap(),
    })).unwrap();

    let stats: crate::msg::GlobalStatsResponse = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::GlobalStats { token_address: "token".to_string() }).unwrap()).unwrap();
    assert_eq!(stats.total_vested, Uint128::new(300));
    assert_eq!(stats.active_vesting_count, 2);
}

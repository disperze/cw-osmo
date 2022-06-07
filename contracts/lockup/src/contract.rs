#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Binary, Coin, ContractResult, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, Response, StdError, StdResult, SubMsg, Uint64,
};
use cw2::set_contract_version;
use cw_osmo_proto::osmosis::lockup;
use cw_osmo_proto::proto_ext::{proto_decode, MessageExt};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, LockResult, QueryMsg};
use crate::state::ADMIN;

use cw_utils::{nonpayable, one_coin};

const CONTRACT_NAME: &str = "crates.io:cw-osmo-lockup";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const LOCK_TOKEN_ID: u64 = 0x43ab;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        LOCK_TOKEN_ID => reply_lock(reply),
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

pub fn reply_lock(reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        ContractResult::Ok(tx) => {
            let data = tx.data.ok_or(ContractError::NoReplyData {})?;

            let response: lockup::MsgLockTokensResponse = proto_decode(data.as_slice())?;
            let result = LockResult {
                lock_id: response.id.into(),
            };

            Ok(Response::new().set_data(to_binary(&result)?))
        }
        ContractResult::Err(err) => Err(StdError::generic_err(err).into()),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin)?;
    ADMIN.set(deps.branch(), Some(admin))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = env.contract.address.into();
    match msg {
        ExecuteMsg::Lock { duration } => {
            let coin = one_coin(&info)?;
            execute_lock(deps, info, duration, coin, contract)
        }
        ExecuteMsg::Unlock { id } => execute_unlock(deps, info, contract, id),
        ExecuteMsg::Claim { denom } => execute_claim(deps, info, contract, denom),
        ExecuteMsg::UpdateAdmin { admin } => {
            let admin = deps.api.addr_validate(&admin)?;
            Ok(ADMIN.execute_update_admin(deps, info, Some(admin))?)
        }
    }
}

pub fn execute_lock(
    deps: DepsMut,
    info: MessageInfo,
    duration: Uint64,
    token_in: Coin,
    contract: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let tx = lockup::MsgLockTokens {
        owner: contract,
        duration: Some(cw_osmo_proto::Duration {
            seconds: duration.u64() as i64,
            nanos: 0,
        }),
        coins: vec![cw_osmo_proto::cosmos::base::v1beta1::Coin {
            denom: token_in.denom,
            amount: token_in.amount.to_string(),
        }],
    };
    let submsg = SubMsg::reply_on_success(tx.to_msg()?, LOCK_TOKEN_ID);

    Ok(Response::new()
        .add_submessage(submsg)
        .add_attribute("action", "lock")
        .add_attribute("duration", duration.to_string()))
}

pub fn execute_unlock(
    deps: DepsMut,
    info: MessageInfo,
    contract: String,
    lock_id: Uint64,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    if lock_id.is_zero() {
        return Err(ContractError::InvalidLockId {});
    }
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let tx = lockup::MsgBeginUnlocking {
        owner: contract,
        id: lock_id.u64(),
        coins: vec![],
    };

    Ok(Response::new()
        .add_message(tx.to_msg()?)
        .add_attribute("action", "unlock")
        .add_attribute("lock_id", lock_id.to_string()))
}

pub fn execute_claim(
    deps: DepsMut,
    info: MessageInfo,
    contract: String,
    denom: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    if denom.is_empty() {
        return Err(ContractError::InvalidEmptyDenom {});
    }
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let balance = deps.querier.query_balance(contract, denom)?;
    if balance.amount.is_zero() {
        return Err(ContractError::NoBalance {});
    }

    let bank_msg: CosmosMsg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![balance.clone()],
    }
    .into();

    Ok(Response::new()
        .set_data(to_binary(&balance)?)
        .add_message(bank_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", balance.amount))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Admin {} => to_binary(&ADMIN.query_admin(deps)?),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::{
        mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{
        attr, coins, from_binary, Binary, Empty, OwnedDeps, SubMsgExecutionResponse,
    };
    use cw_controllers::{AdminError, AdminResponse};
    use cw_utils::PaymentError::NonPayable;

    pub fn mock_lock_events() -> Vec<Event> {
        return vec![
            Event::new("begin_unlock").add_attributes(vec![
                attr("period_lock_id", "16"),
                attr("owner", "osmo1q4aw0vtcyyredprm4ncmr4jdj70kpgyr3"),
                attr("duration", "336h0m0s"),
                attr("unlock_time", "0001-01-01 00:00:00 +0000 UTC"),
            ]),
            Event::new("message").add_attributes(vec![attr("action", "begin_unlocking")]),
        ];
    }

    fn setup_init() -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
        let mut deps = mock_dependencies_with_balance(&coins(1250u128, "uosmo"));

        let sender = mock_info("owner", &[]);
        let msg = InstantiateMsg {
            admin: "owner".to_string(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), sender, msg).unwrap();
        assert_eq!(0, res.messages.len());

        deps
    }

    #[test]
    fn execute_lock() {
        let mut deps = setup_init();
        let denom = "gamm/pool/1";

        let msg = ExecuteMsg::Lock {
            duration: 86400u64.into(),
        };

        // lock token: Invalid owner
        let sender = mock_info("any", &coins(1000u128, denom));
        let err = execute(deps.as_mut(), mock_env(), sender, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Admin(AdminError::NotAdmin {}));

        // lock token: Valid owner
        let sender = mock_info("owner", &coins(1000u128, denom));
        let res = execute(deps.as_mut(), mock_env(), sender, msg).unwrap();
        assert_eq!(1, res.messages.len());

        // Simulate reply result
        let data = Binary::from_base64("CAE=").unwrap(); // id: 1
        let reply_msg = Reply {
            id: LOCK_TOKEN_ID,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: mock_lock_events(),
                data: Some(data),
            }),
        };
        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let lock_res: LockResult = from_binary(&res.data.unwrap()).unwrap();
        assert_eq!(Uint64::new(1u64), lock_res.lock_id);
    }

    #[test]
    fn execute_claim_rewards() {
        let mut deps = setup_init();
        let denom = "uosmo";

        let msg = ExecuteMsg::Claim {
            denom: "uatom".to_string(),
        };

        // Claim rewards: Invalid owner
        let sender = mock_info("any", &[]);
        let err = execute(deps.as_mut(), mock_env(), sender, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Admin(AdminError::NotAdmin {}));

        // Claim rewards: valid owner, no valid denom
        let sender = mock_info("owner", &[]);
        let err = execute(deps.as_mut(), mock_env(), sender, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::NoBalance {});

        // Claim rewards: error funds
        let sender = mock_info("any", &coins(1u128, "uatom"));
        let err = execute(deps.as_mut(), mock_env(), sender, msg).unwrap_err();
        assert_eq!(err, ContractError::Payment(NonPayable {}));

        // Claim rewards: valid owner, valid amount
        let msg = ExecuteMsg::Claim {
            denom: denom.to_string(),
        };

        let sender = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), mock_env(), sender, msg).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner".to_string(),
                amount: coins(1250u128, denom),
            }))
        );
    }

    #[test]
    fn execute_unlock() {
        let mut deps = setup_init();

        let msg = ExecuteMsg::Unlock { id: 1u64.into() };

        // unlock token: Invalid owner
        let sender = mock_info("any", &[]);
        let err = execute(deps.as_mut(), mock_env(), sender, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Admin(AdminError::NotAdmin {}));

        // unlock token: error funds
        let sender = mock_info("any", &coins(1u128, "uatom"));
        let err = execute(deps.as_mut(), mock_env(), sender, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Payment(NonPayable {}));

        // unlock token: Valid owner
        let sender = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), mock_env(), sender, msg).unwrap();
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn query_admin() {
        let deps = setup_init();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Admin {}).unwrap();
        let admin: AdminResponse = from_binary(&res).unwrap();

        assert_eq!("owner", admin.admin.unwrap().as_str());
    }
}

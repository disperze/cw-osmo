#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, BankMsg, Coin, ContractResult, CosmosMsg, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, SubMsg, Uint64,
};
use cw2::set_contract_version;
use cw_osmo_proto::proto_ext::{proto_decode, MessageExt};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, LockResult};
use crate::state::ADMIN;

const CONTRACT_NAME: &str = "crates.io:cw-osmo-lockup";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const LOCK_TOKEN_ID: u64 = 0x43ab;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        LOCK_TOKEN_ID => match reply.result {
            ContractResult::Ok(tx) => {
                let data = tx.data.ok_or(ContractError::NoReplyData {})?;

                let response: cw_osmo_proto::osmosis::lockup::MsgLockTokensResponse =
                    proto_decode(data.as_slice())?;
                let result = LockResult {
                    lock_id: response.id.into(),
                };

                Ok(Response::new().set_data(to_binary(&result)?))
            }
            ContractResult::Err(err) => Err(StdError::generic_err(err).into()),
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
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
        ExecuteMsg::Exit { id } => execute_exit(deps, info, contract, id),
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

    let tx = cw_osmo_proto::osmosis::lockup::MsgLockTokens {
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
        .add_attribute("method", "lock")
        .add_attribute("duration", duration.to_string()))
}

pub fn execute_exit(
    deps: DepsMut,
    info: MessageInfo,
    contract: String,
    lock_id: Uint64,
) -> Result<Response, ContractError> {
    if lock_id.is_zero() {
        return Err(ContractError::InvalidLockId {});
    }
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let tx = cw_osmo_proto::osmosis::lockup::MsgBeginUnlocking {
        owner: contract,
        id: lock_id.u64(),
        coins: vec![],
    };

    Ok(Response::new()
        .add_message(tx.to_msg()?)
        .add_attribute("method", "unlock")
        .add_attribute("lock_id", lock_id.to_string()))
}

pub fn execute_claim(
    deps: DepsMut,
    info: MessageInfo,
    contract: String,
    denom: String,
) -> Result<Response, ContractError> {
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
        .add_attribute("method", "claim")
        .add_attribute("amount", balance.amount))
}

fn one_coin(info: &MessageInfo) -> Result<Coin, ContractError> {
    if info.funds.len() != 1 {
        return Err(ContractError::NoOneToken {});
    }

    let coin = &info.funds[0];
    Ok(coin.clone())
}

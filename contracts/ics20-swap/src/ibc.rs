use cosmwasm_std::{
    attr, coins, entry_point, from_binary, to_binary, BankMsg, Binary, Coin, ContractResult,
    CosmosMsg, DepsMut, Env, IbcBasicResponse, IbcChannel, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcEndpoint, IbcOrder, IbcPacket, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Reply, Response, StdError,
    SubMsg, WasmMsg,
};

use crate::amount::Amount;
use crate::error::{ContractError, Never};
use crate::ibc_msg::{
    parse_gamm_result, AmountResultAck, ClaimPacket, ExitPoolPacket, Ics20Ack, Ics20Packet,
    JoinPoolPacket, LockPacket, LockuptAck, OsmoPacket, SwapPacket, UnlockPacket, Voucher,
};
use crate::msg::LockupInitMsg;
use crate::parse::{
    parse_pool_id, EXIT_POOL_ATTR, EXIT_POOL_EVENT, JOIN_POOL_ATTR, JOIN_POOL_EVENT, SWAP_ATTR,
    SWAP_EVENT,
};
use crate::state::{
    increase_channel_balance, reduce_channel_balance, restore_balance_reply, ChannelInfo,
    ReplyArgs, CHANNEL_INFO, CONFIG, LOCKUP, REPLY_ARGS,
};
use cw_osmo_proto::proto_ext::MessageExt;
use cw_utils::parse_reply_instantiate_data;

pub const ICS20_VERSION: &str = "ics20-1";
pub const ICS20_ORDERING: IbcOrder = IbcOrder::Unordered;

// create a serialized success message
fn ack_success_with_body(data: Binary) -> Binary {
    let res = Ics20Ack::Result(data);
    to_binary(&res).unwrap()
}

// create a serialized success message
fn ack_success() -> Binary {
    let res = Ics20Ack::Result(b"1".into());
    to_binary(&res).unwrap()
}

// create a serialized error message
fn ack_fail(err: String) -> Binary {
    let res = Ics20Ack::Error(err);
    to_binary(&res).unwrap()
}

const RECEIVE_ID: u64 = 1337;
const SWAP_ID: u64 = 0xcb37;
const JOIN_POOL_ID: u64 = 0xad54;
const EXIT_POOL_ID: u64 = 0xfa61;
const ACK_FAILURE_ID: u64 = 0xfa17;
const LOCKUP_ID: u64 = 0xdf16;
const LOCK_TOKEN_ID: u64 = 0xbc42;
const CLAIM_TOKEN_ID: u64 = 0x1654;
const UNLOCK_TOKEN_ID: u64 = 0x6f11;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        RECEIVE_ID => reply_receive(deps, reply),
        SWAP_ID => reply_gamm_result(deps, reply, SWAP_EVENT, SWAP_ATTR),
        JOIN_POOL_ID => reply_gamm_result(deps, reply, JOIN_POOL_EVENT, JOIN_POOL_ATTR),
        EXIT_POOL_ID => reply_gamm_result(deps, reply, EXIT_POOL_EVENT, EXIT_POOL_ATTR),
        LOCKUP_ID => reply_lockup_account(deps, reply),
        LOCK_TOKEN_ID => reply_ack_from_data(deps, reply),
        CLAIM_TOKEN_ID => reply_claim_result(deps, reply),
        UNLOCK_TOKEN_ID => reply_ack_from_data(deps, reply),
        ACK_FAILURE_ID => match reply.result {
            ContractResult::Ok(_) => Ok(Response::new()),
            ContractResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

pub fn reply_gamm_result(
    deps: DepsMut,
    reply: Reply,
    event: &str,
    attribute: &str,
) -> Result<Response, ContractError> {
    match reply.result {
        ContractResult::Ok(tx) => {
            let gamm_res = parse_gamm_result(tx.events, event, attribute);
            match gamm_res {
                Ok(ack) => {
                    let reply_args = REPLY_ARGS.load(deps.storage)?;
                    // increase gamm amount out
                    increase_channel_balance(
                        deps.storage,
                        &reply_args.channel,
                        &ack.denom,
                        ack.amount,
                    )?;
                    let data = to_binary(&ack).unwrap();
                    Ok(Response::new().set_data(ack_success_with_body(data)))
                }
                Err(err) => {
                    restore_balance_reply(deps.storage)?;
                    Ok(Response::new().set_data(ack_fail(err.to_string())))
                }
            }
        }
        ContractResult::Err(err) => {
            restore_balance_reply(deps.storage)?;
            Ok(Response::new().set_data(ack_fail(err)))
        }
    }
}

pub fn reply_lockup_account(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.result.clone() {
        ContractResult::Ok(_) => {
            let res = parse_reply_instantiate_data(reply);

            match res {
                Ok(data) => {
                    let reply_args = REPLY_ARGS.load(deps.storage)?;

                    LOCKUP.save(
                        deps.storage,
                        (&reply_args.channel, &reply_args.sender),
                        &data.contract_address,
                    )?;
                    let ack = LockuptAck {
                        contract: data.contract_address,
                    };
                    let data = to_binary(&ack).unwrap();

                    Ok(Response::new().set_data(ack_success_with_body(data)))
                }
                Err(err) => Ok(Response::new().set_data(ack_fail(err.to_string()))),
            }
        }
        ContractResult::Err(err) => Ok(Response::new().set_data(ack_fail(err))),
    }
}

pub fn reply_claim_result(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        ContractResult::Ok(tx) => {
            let data = tx
                .data
                .ok_or_else(|| StdError::generic_err("Missing reply data"))?;
            let token: Coin = from_binary(&data)?;
            let reply_args = REPLY_ARGS.load(deps.storage)?;
            increase_channel_balance(
                deps.storage,
                &reply_args.channel,
                &token.denom,
                token.amount,
            )?;

            let ack = AmountResultAck {
                denom: token.denom,
                amount: token.amount,
            };
            let data = to_binary(&ack).unwrap();
            Ok(Response::new().set_data(ack_success_with_body(data)))
        }
        ContractResult::Err(err) => {
            restore_balance_reply(deps.storage)?;
            Ok(Response::new().set_data(ack_fail(err)))
        }
    }
}

pub fn reply_ack_from_data(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        ContractResult::Ok(tx) => {
            let data = tx
                .data
                .ok_or_else(|| StdError::generic_err("Missing reply data"))?;

            Ok(Response::new().set_data(ack_success_with_body(data)))
        }
        ContractResult::Err(err) => {
            restore_balance_reply(deps.storage)?;
            Ok(Response::new().set_data(ack_fail(err)))
        }
    }
}

pub fn reply_receive(deps: DepsMut, reply: Reply) -> Result<Response, ContractError> {
    match reply.result {
        ContractResult::Ok(_) => Ok(Response::new()),
        ContractResult::Err(err) => {
            restore_balance_reply(deps.storage)?;
            Ok(Response::new().set_data(ack_fail(err)))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<(), ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// record the channel in CHANNEL_INFO
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    // we need to check the counter party version in try and ack (sometimes here)
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFO.save(deps.storage, &info.id, &info)?;

    Ok(IbcBasicResponse::default())
}

fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version.as_str() != ICS20_VERSION {
        return Err(ContractError::InvalidIbcVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != ICS20_VERSION {
            return Err(ContractError::InvalidIbcVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != ICS20_ORDERING {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    channel: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {
    match channel {
        IbcChannelCloseMsg::CloseConfirm { .. } => Ok(IbcBasicResponse::new()),
        IbcChannelCloseMsg::CloseInit { .. } => Err(ContractError::CannotClose {}),
        _ => panic!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    let packet = msg.packet;

    do_ibc_packet_receive(deps, env, &packet).or_else(|err| {
        Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail(err.to_string()))
            .add_attributes(vec![
                attr("action", "receive"),
                attr("success", "false"),
                attr("error", err.to_string()),
            ]))
    })
}

// Returns local denom if the denom is an encoded voucher from the expected endpoint
// Otherwise, error
fn parse_voucher(
    voucher_denom: String,
    remote_endpoint: &IbcEndpoint,
) -> Result<Voucher, ContractError> {
    let split_denom: Vec<&str> = voucher_denom.splitn(3, '/').collect();
    if split_denom.len() != 3 {
        return Err(ContractError::NoForeignTokens {});
    }
    // a few more sanity checks
    if split_denom[0] != remote_endpoint.port_id {
        return Err(ContractError::FromOtherPort {
            port: split_denom[0].into(),
        });
    }
    if split_denom[1] != remote_endpoint.channel_id {
        return Err(ContractError::FromOtherChannel {
            channel: split_denom[1].into(),
        });
    }

    Ok(Voucher {
        denom: split_denom[2].to_string(),
    })
}

// this does the work of ibc_packet_receive, we wrap it to turn errors into acknowledgements
fn do_ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    packet: &IbcPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;
    let channel = packet.dest.channel_id.clone();

    // If the token originated on the remote chain, it looks like "ucosm".
    // If it originated on our chain, it looks like "port/channel/ucosm".
    let voucher = parse_voucher(msg.denom, &packet.src)?;
    let denom = voucher.denom.as_str();

    reduce_channel_balance(deps.storage, &channel, denom, msg.amount)?;

    // we need to save the data to update the balances in reply
    let reply_args = ReplyArgs {
        channel: channel.clone(),
        denom: denom.to_string(),
        amount: msg.amount,
        sender: msg.sender.clone(),
    };
    REPLY_ARGS.save(deps.storage, &reply_args)?;
    let to_send = Amount::from_parts(denom.to_string(), msg.amount);

    if let Some(action) = msg.action {
        let contract = env.contract.address.into();
        match action {
            OsmoPacket::Swap(swap) => swap_receive(swap, msg.sender, to_send, contract),
            OsmoPacket::JoinPool(join_pool) => {
                receive_join_pool(join_pool, msg.sender, to_send, contract)
            }
            OsmoPacket::ExitPool(exit_pool) => {
                receive_exit_pool(exit_pool, msg.sender, to_send, contract)
            }
            OsmoPacket::LockupAccount {} => {
                receive_create_lockup(deps, &channel, msg.sender, to_send, contract)
            }
            OsmoPacket::Lock(lock) => {
                receive_lock_tokens(deps, &channel, lock, msg.sender, to_send)
            }
            OsmoPacket::Claim(claim) => {
                receive_claim_tokens(deps, &channel, claim, msg.sender, to_send)
            }
            OsmoPacket::Unlock(unlock) => {
                receive_unlock_tokens(deps, &channel, unlock, msg.sender, to_send)
            }
        }
    } else {
        let send = send_amount(to_send, msg.receiver.clone());
        let submsg = SubMsg::reply_on_error(send, RECEIVE_ID);

        let res = IbcReceiveResponse::new()
            .set_ack(ack_success())
            .add_submessage(submsg)
            .add_attribute("action", "receive")
            .add_attribute("sender", msg.sender)
            .add_attribute("receiver", msg.receiver)
            .add_attribute("denom", denom)
            .add_attribute("amount", msg.amount)
            .add_attribute("success", "true");

        Ok(res)
    }
}

fn swap_receive(
    swap: SwapPacket,
    sender: String,
    token_in: Amount,
    contract: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let tx = cw_osmo_proto::osmosis::gamm::v1beta1::MsgSwapExactAmountIn {
        sender: contract,
        token_in: Some(cw_osmo_proto::cosmos::base::v1beta1::Coin {
            denom: token_in.denom(),
            amount: token_in.amount().to_string(),
        }),
        routes: swap
            .routes
            .iter()
            .map(
                |r| cw_osmo_proto::osmosis::gamm::v1beta1::SwapAmountInRoute {
                    token_out_denom: r.token_out_denom.to_owned(),
                    pool_id: r.pool_id.u64(),
                },
            )
            .collect(),
        token_out_min_amount: swap.token_out_min_amount.to_string(),
    };

    let submsg = SubMsg::reply_always(tx.to_msg()?, SWAP_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_swap")
        .add_attribute("sender", sender)
        .add_attribute("denom", token_in.denom())
        .add_attribute("amount", token_in.amount())
        .add_attribute("success", "true");

    Ok(res)
}

fn receive_join_pool(
    join_pool: JoinPoolPacket,
    sender: String,
    token_in: Amount,
    contract: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let tx = cw_osmo_proto::osmosis::gamm::v1beta1::MsgJoinSwapExternAmountIn {
        sender: contract,
        token_in: Some(cw_osmo_proto::cosmos::base::v1beta1::Coin {
            denom: token_in.denom(),
            amount: token_in.amount().to_string(),
        }),
        pool_id: join_pool.pool_id.u64(),
        share_out_min_amount: join_pool.share_out_min_amount.to_string(),
    };

    let submsg = SubMsg::reply_always(tx.to_msg()?, JOIN_POOL_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_join_pool")
        .add_attribute("sender", sender)
        .add_attribute("denom", token_in.denom())
        .add_attribute("amount", token_in.amount())
        .add_attribute("success", "true");

    Ok(res)
}

fn receive_exit_pool(
    exit_pool: ExitPoolPacket,
    sender: String,
    token_in: Amount,
    contract: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let pool_id = parse_pool_id(token_in.denom().as_str())?;
    let tx = cw_osmo_proto::osmosis::gamm::v1beta1::MsgExitSwapShareAmountIn {
        sender: contract,
        pool_id,
        token_out_denom: exit_pool.token_out_denom,
        share_in_amount: token_in.amount().to_string(),
        token_out_min_amount: exit_pool.token_out_min_amount.to_string(),
    };

    let submsg = SubMsg::reply_always(tx.to_msg()?, EXIT_POOL_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_exit_pool")
        .add_attribute("sender", sender)
        .add_attribute("denom", token_in.denom())
        .add_attribute("amount", token_in.amount())
        .add_attribute("success", "true");

    Ok(res)
}

fn receive_create_lockup(
    deps: DepsMut,
    channel: &str,
    sender: String,
    token_in: Amount,
    contract: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let lockup_key = (channel, sender.as_str());
    if LOCKUP.has(deps.storage, lockup_key) {
        return Err(ContractError::OnlyLockupByChannel {});
    }

    if !token_in.amount().is_zero() {
        return Err(ContractError::InvalidAmountValue {});
    }

    let config = CONFIG.load(deps.storage)?;

    let admin = LockupInitMsg { admin: contract };
    let init_msg: CosmosMsg = WasmMsg::Instantiate {
        admin: None,
        msg: to_binary(&admin)?,
        code_id: config.lockup_id,
        label: format!("Lockup {}", channel),
        funds: vec![],
    }
    .into();

    let submsg = SubMsg::reply_always(init_msg, LOCKUP_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_lockup_account")
        .add_attribute("success", "true");

    Ok(res)
}

fn receive_lock_tokens(
    deps: DepsMut,
    channel: &str,
    lock: LockPacket,
    sender: String,
    token_in: Amount,
) -> Result<IbcReceiveResponse, ContractError> {
    let lock_key = (channel, sender.as_str());
    let lockup_contract = LOCKUP
        .load(deps.storage, lock_key)
        .map_err(|_| ContractError::LockupNotFound {})?;

    let exec_msg = create_lockup_msg(lockup_contract, to_binary(&lock)?, &token_in);
    let submsg = SubMsg::reply_always(exec_msg, LOCK_TOKEN_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_lock_tokens")
        .add_attribute("sender", sender)
        .add_attribute("denom", token_in.denom())
        .add_attribute("amount", token_in.amount())
        .add_attribute("success", "true");

    Ok(res)
}

fn receive_claim_tokens(
    deps: DepsMut,
    channel: &str,
    claim: ClaimPacket,
    sender: String,
    token_in: Amount,
) -> Result<IbcReceiveResponse, ContractError> {
    let lock_key = (channel, sender.as_str());
    let lockup_contract = LOCKUP
        .load(deps.storage, lock_key)
        .map_err(|_| ContractError::LockupNotFound {})?;

    let exec_msg = create_lockup_msg(lockup_contract, to_binary(&claim)?, &token_in);
    let submsg = SubMsg::reply_always(exec_msg, CLAIM_TOKEN_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_claim_tokens")
        .add_attribute("sender", sender)
        .add_attribute("success", "true");

    Ok(res)
}

fn receive_unlock_tokens(
    deps: DepsMut,
    channel: &str,
    unlock: UnlockPacket,
    sender: String,
    token_in: Amount,
) -> Result<IbcReceiveResponse, ContractError> {
    let lock_key = (channel, sender.as_str());
    let lockup_contract = LOCKUP
        .load(deps.storage, lock_key)
        .map_err(|_| ContractError::LockupNotFound {})?;

    let exec_msg = create_lockup_msg(lockup_contract, to_binary(&unlock)?, &token_in);
    let submsg = SubMsg::reply_always(exec_msg, UNLOCK_TOKEN_ID);

    let res = IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(submsg)
        .add_attribute("action", "receive_unlock")
        .add_attribute("sender", sender)
        .add_attribute("success", "true");

    Ok(res)
}

fn create_lockup_msg(contract_addr: String, msg: Binary, fund: &Amount) -> CosmosMsg {
    WasmMsg::Execute {
        contract_addr,
        msg,
        funds: coins(fund.amount().u128(), fund.denom()),
    }
    .into()
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// check if success or failure and update balance, or return funds
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let ics20msg: Ics20Ack = from_binary(&msg.acknowledgement.data)?;
    match ics20msg {
        Ics20Ack::Result(_) => on_packet_success(msg.original_packet),
        Ics20Ack::Error(err) => on_packet_failure(deps, msg.original_packet, err),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// return fund to original sender (same as failure in ibc_packet_ack)
pub fn ibc_packet_timeout(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let packet = msg.packet;
    on_packet_failure(deps, packet, "timeout".to_string())
}

// update the balance stored on this (channel, denom) index
fn on_packet_success(packet: IbcPacket) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;

    // similar event messages like ibctransfer module
    let attributes = vec![
        attr("action", "acknowledge"),
        attr("sender", &msg.sender),
        attr("receiver", &msg.receiver),
        attr("denom", &msg.denom),
        attr("amount", msg.amount),
        attr("success", "true"),
    ];

    Ok(IbcBasicResponse::new().add_attributes(attributes))
}

// return the tokens to sender
fn on_packet_failure(
    deps: DepsMut,
    packet: IbcPacket,
    err: String,
) -> Result<IbcBasicResponse, ContractError> {
    let msg: Ics20Packet = from_binary(&packet.data)?;

    reduce_channel_balance(deps.storage, &packet.src.channel_id, &msg.denom, msg.amount)?;

    let to_send = Amount::from_parts(msg.denom.clone(), msg.amount);
    let send = send_amount(to_send, msg.sender.clone());
    let submsg = SubMsg::reply_on_error(send, ACK_FAILURE_ID);

    // similar event messages like ibctransfer module
    let res = IbcBasicResponse::new()
        .add_submessage(submsg)
        .add_attribute("action", "acknowledge")
        .add_attribute("sender", msg.sender)
        .add_attribute("receiver", msg.receiver)
        .add_attribute("denom", msg.denom)
        .add_attribute("amount", msg.amount.to_string())
        .add_attribute("success", "false")
        .add_attribute("error", err);

    Ok(res)
}

fn send_amount(amount: Amount, recipient: String) -> CosmosMsg {
    match amount {
        Amount::Native(coin) => BankMsg::Send {
            to_address: recipient,
            amount: vec![coin],
        }
        .into(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::*;

    use crate::contract::{execute, query_channel};
    use crate::ibc_msg::{AmountResultAck, SwapAmountInRoute};
    use crate::msg::{ExecuteMsg, TransferMsg};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{
        coins, to_vec, Event, IbcEndpoint, ReplyOn, StdError, StdResult, SubMsgExecutionResponse,
        Timestamp, Uint128, Uint64,
    };
    use serde::Serialize;

    #[test]
    fn check_ack_json() {
        let success = Ics20Ack::Result(b"1".into());
        let fail = Ics20Ack::Error("bad coin".into());

        let success_json = String::from_utf8(to_vec(&success).unwrap()).unwrap();
        assert_eq!(r#"{"result":"MQ=="}"#, success_json.as_str());

        let fail_json = String::from_utf8(to_vec(&fail).unwrap()).unwrap();
        assert_eq!(r#"{"error":"bad coin"}"#, fail_json.as_str());
    }

    #[test]
    fn check_packet_json() {
        let packet = Ics20Packet::new(
            Uint128::new(12345),
            "ucosm",
            "cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n",
            "wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc",
        );
        // Example message generated from the SDK
        let expected = r#"{"amount":"12345","denom":"ucosm","receiver":"wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc","sender":"cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n"}"#;

        let encoded = String::from_utf8(to_vec(&packet).unwrap()).unwrap();
        assert_eq!(expected, encoded.as_str());
    }

    #[test]
    fn check_gamm_packet_json() {
        let packet = Ics20Packet {
            sender: "cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n".to_string(),
            receiver: "wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc".to_string(),
            amount: Uint128::new(12345),
            denom: "ucosm".to_string(),
            action: Some(OsmoPacket::JoinPool(JoinPoolPacket {
                pool_id: Uint64::new(1),
                share_out_min_amount: Uint128::new(1),
            })),
        };

        // Example message generated from the SDK
        let expected = r#"{"amount":"12345","denom":"ucosm","receiver":"wasm1fucynrfkrt684pm8jrt8la5h2csvs5cnldcgqc","sender":"cosmos1zedxv25ah8fksmg2lzrndrpkvsjqgk4zt5ff7n","action":{"join_pool":{"pool_id":"1","share_out_min_amount":"1"}}}"#;

        let encoded = String::from_utf8(to_vec(&packet).unwrap()).unwrap();
        assert_eq!(expected, encoded.as_str());
    }

    fn native_payment(amount: u128, denom: &str, recipient: &str) -> SubMsg {
        SubMsg::reply_on_error(
            BankMsg::Send {
                to_address: recipient.into(),
                amount: coins(amount, denom),
            },
            RECEIVE_ID,
        )
    }

    fn check_gamm_submsg(msg: SubMsg, reply_id: u64, action: &str) -> StdResult<()> {
        if msg.id != reply_id {
            return Err(StdError::generic_err("Invalid reply id"));
        }

        if msg.reply_on != ReplyOn::Always {
            return Err(StdError::generic_err("Invalid reply on"));
        }

        match msg.msg {
            CosmosMsg::Stargate { type_url, .. } => {
                if !type_url.to_lowercase().contains(action) {
                    return Err(StdError::generic_err(format!(
                        "Invalid stargate proto url: {}",
                        type_url
                    )));
                }
            }
            _ => return Err(StdError::generic_err("Invalid cosmMsg")),
        };

        Ok(())
    }

    fn get_gamm_ack(data: &Binary) -> StdResult<AmountResultAck> {
        let ack: Ics20Ack = from_binary(data).unwrap();
        match ack {
            Ics20Ack::Result(data) => {
                let result: AmountResultAck = from_binary(&data).unwrap();
                Ok(result)
            }
            Ics20Ack::Error(err) => Err(StdError::generic_err(err)),
        }
    }

    fn mock_reply_msg(id: u64, events: Vec<Event>) -> Reply {
        Reply {
            id,
            result: ContractResult::Ok(SubMsgExecutionResponse { events, data: None }),
        }
    }

    fn mock_ics20_data(
        amount: u128,
        denom: &str,
        receiver: &str,
        action: Option<OsmoPacket>,
    ) -> Ics20Packet {
        Ics20Packet {
            // this is returning a foreign (our) token, thus denom is <port>/<channel>/<denom>
            denom: format!("{}/{}/{}", REMOTE_PORT, "channel-1234", denom),
            amount: amount.into(),
            sender: "remote-sender".to_string(),
            receiver: receiver.to_string(),
            action,
        }
    }

    fn mock_receive_packet(
        my_channel: &str,
        amount: u128,
        denom: &str,
        receiver: &str,
    ) -> IbcPacketReceiveMsg {
        let data = mock_ics20_data(amount, denom, receiver, None);

        mock_ibc_rcv_packet(my_channel, &data)
    }

    fn mock_ibc_rcv_packet(my_channel: &str, data: &impl Serialize) -> IbcPacketReceiveMsg {
        IbcPacketReceiveMsg::new(IbcPacket::new(
            to_binary(&data).unwrap(),
            IbcEndpoint {
                port_id: REMOTE_PORT.to_string(),
                channel_id: "channel-1234".to_string(),
            },
            IbcEndpoint {
                port_id: CONTRACT_PORT.to_string(),
                channel_id: my_channel.to_string(),
            },
            3,
            Timestamp::from_seconds(1665321069).into(),
        ))
    }

    #[test]
    fn send_receive_native() {
        let send_channel = "channel-9";
        let mut deps = setup(&["channel-1", "channel-7", send_channel]);

        let denom = "uatom";

        // prepare some mock packets
        let recv_packet = mock_receive_packet(send_channel, 876543210, denom, "local-rcpt");
        let recv_high_packet = mock_receive_packet(send_channel, 1876543210, denom, "local-rcpt");

        // cannot receive this denom yet
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), recv_packet.clone()).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        let no_funds = Ics20Ack::Error(ContractError::InsufficientFunds {}.to_string());
        assert_eq!(ack, no_funds);

        // we transfer some tokens
        let msg = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "my-remote-address".to_string(),
            timeout: None,
        });
        let info = mock_info("local-sender", &coins(987654321, denom));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(987654321, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);

        // cannot receive more than we sent
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), recv_high_packet).unwrap();
        assert!(res.messages.is_empty());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert_eq!(ack, no_funds);

        // we can receive less than we sent
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), recv_packet).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(
            native_payment(876543210, denom, "local-rcpt"),
            res.messages[0]
        );
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert!(matches!(ack, Ics20Ack::Result(_)));

        // only need to call reply block on error case

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(111111111, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);
    }

    #[test]
    fn receive_swap_action() {
        let send_channel = "channel-9";
        let mut deps = setup(&["channel-1", "channel-7", send_channel]);
        let denom = "uatom";
        let swap_denom = "uosmo";

        let swap = OsmoPacket::Swap(SwapPacket {
            routes: vec![SwapAmountInRoute {
                pool_id: 1u8.into(),
                token_out_denom: swap_denom.to_string(),
            }],
            token_out_min_amount: 1u8.into(),
        });

        let swap_packet_data = mock_ics20_data(876543210, denom, "", Some(swap));

        // prepare some mock packets
        let swap_packet = mock_ibc_rcv_packet(send_channel, &swap_packet_data);

        // we transfer some tokens
        let msg = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "my-remote-address".to_string(),
            timeout: None,
        });
        let info = mock_info("local-sender", &coins(987654321, denom));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(987654321, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);

        // Swap action
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), swap_packet).unwrap();
        assert_eq!(1, res.messages.len());
        check_gamm_submsg(res.messages[0].clone(), SWAP_ID, "swap").unwrap();

        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert!(matches!(ack, Ics20Ack::Result(_)));

        // Simulate swap reply
        let reply_msg = mock_reply_msg(SWAP_ID, swap_events_mock());

        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        assert_eq!(0, res.messages.len());
        let gamm_ack = get_gamm_ack(&res.data.unwrap()).unwrap();
        let gamm_ack_exp = AmountResultAck {
            amount: Uint128::new(36601070u128),
            denom: swap_denom.to_string(),
        };
        assert_eq!(gamm_ack, gamm_ack_exp);

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(
            state.balances,
            vec![
                Amount::native(111111111, denom),
                Amount::native(36601070, swap_denom)
            ]
        );
        assert_eq!(
            state.total_sent,
            vec![
                Amount::native(987654321, denom),
                Amount::native(36601070, swap_denom)
            ]
        );
    }

    #[test]
    fn receive_liquidty_actions() {
        let send_channel = "channel-9";
        let mut deps = setup(&["channel-1", "channel-7", send_channel]);
        let denom = "uosmo";
        let pool_denom = "gamm/pool/1";

        let join_pool = OsmoPacket::JoinPool(JoinPoolPacket {
            pool_id: 1u8.into(),
            share_out_min_amount: 1u8.into(),
        });

        let exit_pool = OsmoPacket::ExitPool(ExitPoolPacket {
            token_out_denom: denom.into(),
            token_out_min_amount: 1u8.into(),
        });

        let join_packet_data = mock_ics20_data(876543210, denom, "", Some(join_pool));
        let exit_packet_data =
            mock_ics20_data(74196992097318119147, pool_denom, "", Some(exit_pool));

        // prepare some mock packets
        let join_packet = mock_ibc_rcv_packet(send_channel, &join_packet_data);
        let exit_packet = mock_ibc_rcv_packet(send_channel, &exit_packet_data);

        // we transfer some tokens
        let msg = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "my-remote-address".to_string(),
            timeout: None,
        });
        let info = mock_info("local-sender", &coins(987654321, denom));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(987654321, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);

        // Join pool action
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), join_packet).unwrap();
        assert_eq!(1, res.messages.len());
        check_gamm_submsg(res.messages[0].clone(), JOIN_POOL_ID, "join").unwrap();

        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert!(matches!(ack, Ics20Ack::Result(_)));

        // Simulate join_pool reply
        let reply_msg = mock_reply_msg(JOIN_POOL_ID, join_pool_events_mock());
        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        assert_eq!(0, res.messages.len());
        let gamm_ack = get_gamm_ack(&res.data.unwrap()).unwrap();
        let gamm_ack_exp = AmountResultAck {
            amount: Uint128::new(74196993097318119147u128),
            denom: pool_denom.to_string(),
        };
        assert_eq!(gamm_ack, gamm_ack_exp);

        // Exit pool action
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), exit_packet).unwrap();
        assert_eq!(1, res.messages.len());
        check_gamm_submsg(res.messages[0].clone(), EXIT_POOL_ID, "exit").unwrap();

        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert!(matches!(ack, Ics20Ack::Result(_)));

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(
            state.balances,
            vec![
                Amount::native(1000000000000, pool_denom),
                Amount::native(111111111, denom)
            ]
        );
        assert_eq!(
            state.total_sent,
            vec![
                Amount::native(74196993097318119147, pool_denom),
                Amount::native(987654321, denom)
            ]
        );
    }

    #[test]
    fn receive_lockup_actions() {
        let send_channel = "channel-9";
        let mut deps = setup(&["channel-1", "channel-7", send_channel]);
        let denom = "uosmo";

        let lockup = OsmoPacket::LockupAccount {};
        let unlock = OsmoPacket::Unlock(UnlockPacket { id: 1u64.into() });

        let lockup_packet_data = mock_ics20_data(0, denom, "", Some(lockup));
        let unlock_packet_data = mock_ics20_data(0, denom, "", Some(unlock));

        // prepare some mock packets
        let lockup_packet = mock_ibc_rcv_packet(send_channel, &lockup_packet_data);
        let unlock_packet = mock_ibc_rcv_packet(send_channel, &unlock_packet_data);

        // we transfer some tokens
        let msg = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "my-remote-address".to_string(),
            timeout: None,
        });
        let info = mock_info("local-sender", &coins(987654321, denom));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // query channel state|_|
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(987654321, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);

        // Unlock invalid, no lockup account
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), unlock_packet.clone()).unwrap();
        assert_eq!(0, res.messages.len());

        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        let no_lockup_account = Ics20Ack::Error(ContractError::LockupNotFound {}.to_string());
        assert_eq!(ack, no_lockup_account);

        // Lockup account action
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), lockup_packet).unwrap();
        assert_eq!(1, res.messages.len());

        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert!(matches!(ack, Ics20Ack::Result(_)));

        // Save lockup contract
        let lockup_contract = "lockup-addr".to_string();
        LOCKUP
            .save(
                deps.as_mut().storage,
                (send_channel, lockup_packet_data.sender.as_str()),
                &lockup_contract,
            )
            .unwrap();

        // Unlock tokens action on invalid channel
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), unlock_packet).unwrap();
        assert_eq!(1, res.messages.len());
        let ack: Ics20Ack = from_binary(&res.acknowledgement).unwrap();
        assert!(matches!(ack, Ics20Ack::Result(_)));
        assert!(matches!(res.messages[0], SubMsg {
            id,
            ref reply_on,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                ref contract_addr,
                ..
            }),
            ..
        } if id == UNLOCK_TOKEN_ID && reply_on.clone() == ReplyOn::Always && contract_addr.eq(&lockup_contract)));

        // query channel state
        let state = query_channel(deps.as_ref(), send_channel.to_string()).unwrap();
        assert_eq!(state.balances, vec![Amount::native(987654321, denom)]);
        assert_eq!(state.total_sent, vec![Amount::native(987654321, denom)]);
    }

    #[test]
    fn reply_on_errors() {
        let send_channel = "channel-9";
        let mut deps = setup(&["channel-1", "channel-7", send_channel]);
        let denom = "uosmo";
        let error_msg = "Invalid operation".to_string();

        let join_pool = OsmoPacket::JoinPool(JoinPoolPacket {
            pool_id: 1u8.into(),
            share_out_min_amount: 1u8.into(),
        });
        let reply_msg = Reply {
            id: SWAP_ID,
            result: ContractResult::Err(error_msg.clone()),
        };

        // Transfer initial tokens
        let msg = ExecuteMsg::Transfer(TransferMsg {
            channel: send_channel.to_string(),
            remote_address: "my-remote-address".to_string(),
            timeout: None,
        });
        let info = mock_info("local-sender", &coins(1000, denom));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Join pool to simulate reply
        let join_packet = mock_ibc_rcv_packet(
            send_channel,
            &mock_ics20_data(1000, denom, "", Some(join_pool)),
        );
        let res = ibc_packet_receive(deps.as_mut(), mock_env(), join_packet).unwrap();
        assert_eq!(1, res.messages.len());

        // Reply with error result
        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        let ack: Ics20Ack = from_binary(&res.data.unwrap()).unwrap();
        assert!(matches!(ack, Ics20Ack::Error(err) if err == error_msg));
    }
}

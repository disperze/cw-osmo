use crate::ContractError;
use cosmwasm_std::{Attribute, Coin, Event};

pub const OSMOSIS_EVENT_SWAP: &str = "token_swapped";
pub const OSMOSIS_ATTRIBUTE_TOKEN_OUT: &str = "tokens_out";

pub fn find_event_type(events: Vec<Event>, key: &str) -> Option<Event> {
    for ev in events {
        if ev.ty.eq(&key) {
            return Some(ev);
        }
    }

    None
}

pub fn find_attributes(attributes: Vec<Attribute>, key: &str) -> Vec<String> {
    let mut values = vec![];
    for attr in attributes {
        if attr.key.eq(&key) {
            values.push(attr.value)
        }
    }

    values
}

pub fn parse_coin(value: &str) -> Result<Coin, ContractError> {
    let mut num_str = vec![];
    for c in value.chars() {
        if !c.is_numeric() {
            break;
        }

        num_str.push(c)
    }

    let amount_str: String = num_str.into_iter().collect();
    let amount = amount_str
        .parse::<u128>()
        .map_err(|_| ContractError::InvalidAmountValue {})?;
    let denom = value.replace(amount_str.as_str(), "");

    Ok(Coin {
        amount: amount.into(),
        denom,
    })
}

#[cfg(test)]
mod test {
    use crate::ibc_msg::parse_swap_out;
    use crate::parse::{
        find_attributes, find_event_type, parse_coin, OSMOSIS_ATTRIBUTE_TOKEN_OUT,
        OSMOSIS_EVENT_SWAP,
    };
    use crate::test_helpers::swap_events_mock;
    use cosmwasm_std::Uint128;

    #[test]
    fn parse_token_str() {
        let ibc_denom = "ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC";
        let cases = vec![
            ("1000ujuno", 1000u64, "ujuno", true),
            (
                "1000338527564ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC",
                1000338527564u64,
                ibc_denom,
                true,
            ),
            ("6543gamm/pool/1", 6543u64, "gamm/pool/1", true),
            ("aafffbbcc", 0u64, "", false),
            ("x6557", 0u64, "", false),
        ];

        for case in cases {
            let res = parse_coin(case.0);
            assert_eq!(case.3, res.is_ok());
            if !case.3 {
                continue;
            }

            let coin = res.unwrap();

            assert_eq!(Uint128::from(case.1), coin.amount);
            assert_eq!(case.2, coin.denom);
        }
    }

    #[test]
    fn find_events_attributes() {
        let events = swap_events_mock();

        let event = find_event_type(events, OSMOSIS_EVENT_SWAP);
        assert_eq!(true, event.is_some());

        let attrs = find_attributes(event.unwrap().attributes, OSMOSIS_ATTRIBUTE_TOKEN_OUT);
        assert_eq!(2, attrs.len());
    }

    #[test]
    fn parse_swap_output() {
        let events = swap_events_mock();
        let result = parse_swap_out(events);

        assert_eq!(true, result.is_ok());
        let token = result.unwrap();

        assert_eq!(Uint128::new(338527564), token.amount);
        assert_eq!(
            "ibc/BE1BB42D4BE3C30D50B68D7C41DB4DFCE9678E8EF8C539F6E6A9345048894FCC",
            token.denom
        );
    }
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Coin, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Amount {
    Native(Coin),
}

impl Amount {
    pub fn from_parts(denom: String, amount: Uint128) -> Self {
        Amount::Native(Coin { denom, amount })
    }

    pub fn native(amount: u128, denom: &str) -> Self {
        Amount::Native(Coin {
            denom: denom.to_string(),
            amount: Uint128::new(amount),
        })
    }
}

impl Amount {
    pub fn denom(&self) -> String {
        match self {
            Amount::Native(c) => c.denom.clone(),
        }
    }

    pub fn amount(&self) -> Uint128 {
        match self {
            Amount::Native(c) => c.amount,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Amount::Native(c) => c.amount.is_zero(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::amount::Amount;
    use cosmwasm_std::Uint128;

    #[test]
    fn parse_amount() {
        // native denom
        let res = Amount::from_parts("ucosm".to_string(), 1u8.into());

        assert_eq!("ucosm", res.denom());
        assert_eq!(Uint128::new(1), res.amount());
    }
}

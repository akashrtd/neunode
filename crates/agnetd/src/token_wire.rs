use serde::Serialize;

/// Canonical token balance representation used by every machine-facing transport.
///
/// Amounts are decimal strings because protocol balances are `u128`, which cannot be represented
/// losslessly by JavaScript numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct TokenBalanceWire {
    pub token: String,
    pub balance: String,
    pub staked: String,
}

impl TokenBalanceWire {
    pub fn new(token: impl Into<String>, balance: u128, staked: u128) -> Self {
        Self { token: token.into(), balance: balance.to_string(), staked: staked.to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct TokenBalancesWire {
    pub balances: Vec<TokenBalanceWire>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_u128_amounts_losslessly_as_decimal_strings() {
        let balance = TokenBalanceWire::new("nCompute", u128::MAX, u128::MAX - 1);
        let json = serde_json::to_value(balance).unwrap();

        assert_eq!(json["balance"], u128::MAX.to_string());
        assert_eq!(json["staked"], (u128::MAX - 1).to_string());
    }
}

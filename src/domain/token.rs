use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A tradeable token registered in the system.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Token {
    pub id: Uuid,
    /// Token ticker symbol (e.g., "ETH", "USDC").
    pub symbol: String,
    /// Human-readable name (e.g., "Ethereum").
    pub name: String,
    /// Number of decimal places for the token.
    pub decimals: i16,
    /// On-chain address of the token contract.
    pub address: String,
}

/// A pair of tokens forming a market.
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub base: Token,
    pub quote: Token,
}

impl TokenPair {
    pub fn new(base: Token, quote: Token) -> Self {
        Self { base, quote }
    }
}

/// Request payload for creating a new token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTokenRequest {
    pub symbol: String,
    pub name: String,
    pub decimals: i16,
    pub address: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_pair() {
        let base = Token {
            id: Uuid::new_v4(),
            symbol: "ETH".to_string(),
            name: "Ethereum".to_string(),
            decimals: 18,
            address: "0x0000".to_string(),
        };
        let quote = Token {
            id: Uuid::new_v4(),
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            address: "0x0001".to_string(),
        };
        let pair = TokenPair::new(base.clone(), quote.clone());
        assert_eq!(pair.base.symbol, "ETH");
        assert_eq!(pair.quote.symbol, "USDC");
    }
}

use anyhow::Result;
use neunode_core::types::TokenType;

pub fn parse_token_type(s: &str) -> Result<TokenType> {
    match s.to_lowercase().as_str() {
        "compute" | "ncompute" => Ok(TokenType::Compute),
        "train" | "ntrain" => Ok(TokenType::Train),
        "bandwidth" | "nbandwidth" => Ok(TokenType::Bandwidth),
        "storage" | "nstorage" => Ok(TokenType::Storage),
        _ => anyhow::bail!(
            "invalid token type '{}'. Must be one of: compute, train, bandwidth, storage",
            s
        ),
    }
}

pub fn token_type_display(t: &TokenType) -> &'static str {
    match t {
        TokenType::Compute => "nCompute",
        TokenType::Train => "nTrain",
        TokenType::Bandwidth => "nBandwidth",
        TokenType::Storage => "nStorage",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_token_type_all_variants() {
        assert!(matches!(parse_token_type("compute"), Ok(TokenType::Compute)));
        assert!(matches!(parse_token_type("ncompute"), Ok(TokenType::Compute)));
        assert!(matches!(parse_token_type("train"), Ok(TokenType::Train)));
        assert!(matches!(parse_token_type("ntrain"), Ok(TokenType::Train)));
        assert!(matches!(parse_token_type("bandwidth"), Ok(TokenType::Bandwidth)));
        assert!(matches!(parse_token_type("nbandwidth"), Ok(TokenType::Bandwidth)));
        assert!(matches!(parse_token_type("storage"), Ok(TokenType::Storage)));
        assert!(matches!(parse_token_type("nstorage"), Ok(TokenType::Storage)));
        assert!(parse_token_type("invalid").is_err());
    }

    #[test]
    fn parse_token_type_case_insensitive() {
        assert!(matches!(parse_token_type("Compute"), Ok(TokenType::Compute)));
        assert!(matches!(parse_token_type("TRAIN"), Ok(TokenType::Train)));
    }
}

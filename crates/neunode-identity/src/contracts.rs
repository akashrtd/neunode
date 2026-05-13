use alloy::primitives::{keccak256, Address, B256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;

use crate::keyring::Keyring;

sol! {
    #[sol(rpc)]
    interface INeunodeIdentity {
        function createDid(bytes32 ed25519PubKeyHash) external returns (bytes32 didHash);
        function getController(bytes32 didHash) external view returns (address);
        function isActive(bytes32 didHash) external view returns (bool);
        function getDidForAddress(address addr) external view returns (bytes32);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OnChainError {
    #[error("missing RPC URL configuration")]
    MissingRpcUrl,
    #[error("missing identity contract address")]
    MissingContractAddress,
    #[error("invalid contract address: {0}")]
    InvalidContractAddress(String),
    #[error("RPC connection failed: {0}")]
    RpcConnectionFailed(String),
    #[error("transaction failed: {0}")]
    TransactionFailed(String),
}

pub type OnChainResult<T> = std::result::Result<T, OnChainError>;

pub struct OnChainConfig {
    pub eth_rpc_url: String,
    pub identity_contract_address: String,
}

pub struct OnChainDidResult {
    pub tx_hash: String,
    pub did_hash: [u8; 32],
    pub block_number: u64,
}

pub async fn register_did_onchain(
    config: &OnChainConfig,
    keyring: &Keyring,
) -> OnChainResult<OnChainDidResult> {
    let contract_address: Address = config
        .identity_contract_address
        .parse()
        .map_err(|e| OnChainError::InvalidContractAddress(format!("{e}")))?;

    let ed_pubkey_bytes = keyring.ed25519_public_key().to_bytes();
    let ed_pubkey_hash: [u8; 32] = keccak256(ed_pubkey_bytes).into();

    let (_, secp_bytes) = keyring.to_bytes();
    let signer = PrivateKeySigner::from_slice(&secp_bytes)
        .map_err(|e| OnChainError::TransactionFailed(format!("invalid signing key: {e}")))?;
    let signer_address = signer.address();

    let url: url::Url = config
        .eth_rpc_url
        .parse()
        .map_err(|e| OnChainError::RpcConnectionFailed(format!("invalid RPC URL: {e}")))?;

    let provider = ProviderBuilder::new().wallet(signer).connect_http(url);

    let contract = INeunodeIdentity::new(contract_address, &provider);

    let pending_tx = contract
        .createDid(B256::from(ed_pubkey_hash))
        .send()
        .await
        .map_err(|e| OnChainError::TransactionFailed(format!("send failed: {e}")))?;

    let receipt = pending_tx
        .get_receipt()
        .await
        .map_err(|e| OnChainError::TransactionFailed(format!("receipt failed: {e}")))?;

    let tx_hash_bytes = receipt.transaction_hash.as_slice();
    let tx_hash =
        format!("0x{}", tx_hash_bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());

    let did_hash: [u8; 32] = match contract.getDidForAddress(signer_address).call().await {
        Ok(h) => h.into(),
        Err(_) => keccak256(ed_pubkey_bytes).into(),
    };

    Ok(OnChainDidResult { tx_hash, did_hash, block_number: receipt.block_number.unwrap_or(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keccak256_ed25519_pubkey_nonzero() {
        let kr = Keyring::generate();
        let pubkey_bytes = kr.ed25519_public_key().to_bytes();
        let hash: [u8; 32] = keccak256(pubkey_bytes).into();
        assert_ne!(hash, [0u8; 32]);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn keccak256_deterministic() {
        let kr = Keyring::generate();
        let pk = kr.ed25519_public_key().to_bytes();
        let h1: [u8; 32] = keccak256(pk).into();
        let h2: [u8; 32] = keccak256(pk).into();
        assert_eq!(h1, h2);
    }

    #[test]
    fn invalid_contract_address_fails_parse() {
        let result: std::result::Result<Address, _> = "not-an-address".parse();
        assert!(result.is_err());
    }

    #[test]
    fn valid_contract_address_parses() {
        let result: std::result::Result<Address, _> =
            "0x1234567890123456789012345678901234567890".parse();
        assert!(result.is_ok());
    }

    #[test]
    fn signer_from_keyring_secp_bytes() {
        let kr = Keyring::generate();
        let (_, secp_bytes) = kr.to_bytes();
        let signer = PrivateKeySigner::from_slice(&secp_bytes);
        assert!(signer.is_ok());
        let addr = format!("{:x}", signer.unwrap().address());
        let eth_addr = kr.ethereum_address();
        let expected = eth_addr.trim_start_matches("0x");
        assert_eq!(addr, expected);
    }
}

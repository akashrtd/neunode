use std::borrow::Cow;

use alloy::primitives::{Address, U256};
use alloy::sol_types::Eip712Domain;
use k256::ecdsa::{
    signature::hazmat::PrehashSigner, signature::hazmat::PrehashVerifier, SigningKey, VerifyingKey,
};
use sha3::{Digest, Keccak256};

use crate::CryptoError;

const EIP712_DOMAIN_TYPE: &[u8] =
    b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";

const TIMESTAMP_TOLERANCE_SECS: u64 = 300;

fn keccak256(data: &[u8]) -> [u8; 32] {
    let hash = Keccak256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hash);
    out
}

fn encode_type(primary_type: &str, types: &serde_json::Value) -> String {
    let type_obj = match types.get(primary_type) {
        Some(t) => t,
        None => return primary_type.to_string(),
    };

    let fields = type_obj
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|f| {
                    let name = f.get("name")?.as_str()?;
                    let ty = f.get("type")?.as_str()?;
                    Some(format!("{ty} {name}"))
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();

    let mut result = format!("{primary_type}({fields})");

    let mut referenced: Vec<String> = Vec::new();
    collect_referenced_types(primary_type, types, &mut referenced);
    referenced.sort();
    for rtype in referenced {
        let rtype_obj = match types.get(&rtype) {
            Some(t) => t,
            None => continue,
        };
        let rfields = rtype_obj
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| {
                        let name = f.get("name")?.as_str()?;
                        let ty = f.get("type")?.as_str()?;
                        Some(format!("{ty} {name}"))
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        result.push_str(&format!("{rtype}({rfields})"));
    }

    result
}

fn collect_referenced_types(
    primary_type: &str,
    types: &serde_json::Value,
    collected: &mut Vec<String>,
) {
    let type_obj = match types.get(primary_type) {
        Some(t) => t,
        None => return,
    };
    let Some(arr) = type_obj.as_array() else {
        return;
    };
    for field in arr {
        if let Some(field_type) = field.get("type").and_then(|t| t.as_str()) {
            if !is_native_type(field_type)
                && field_type != primary_type
                && !collected.iter().any(|c| c == field_type)
            {
                collected.push(field_type.to_string());
                collect_referenced_types(field_type, types, collected);
            }
        }
    }
}

fn is_native_type(t: &str) -> bool {
    matches!(
        t,
        "uint256"
            | "uint"
            | "int256"
            | "int"
            | "address"
            | "bool"
            | "string"
            | "bytes"
            | "bytes32"
            | "bytes1"
            | "bytes2"
            | "bytes4"
            | "bytes8"
            | "bytes16"
            | "bytes20"
            | "bytes64"
    ) || t.starts_with("uint")
        || t.starts_with("int")
}

fn encode_data(
    primary_type: &str,
    types: &serde_json::Value,
    message: &serde_json::Value,
) -> Vec<u8> {
    let type_obj = match types.get(primary_type) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let type_hash = keccak256(encode_type(primary_type, types).as_bytes());
    let mut encoded = type_hash.to_vec();

    let Some(arr) = type_obj.as_array() else {
        return encoded;
    };

    for field in arr {
        let field_name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let field_type = field.get("type").and_then(|t| t.as_str()).unwrap_or("bytes32");
        let value = message.get(field_name);

        encoded.extend(encode_value(field_type, value, types));
    }

    encoded
}

fn encode_value(
    field_type: &str,
    value: Option<&serde_json::Value>,
    types: &serde_json::Value,
) -> Vec<u8> {
    let null_val = serde_json::Value::Null;
    let value = value.unwrap_or(&null_val);

    match field_type {
        "uint256" | "uint" => {
            let n = value.as_u64().unwrap_or(0);
            // u64 is only 8 bytes, so we need to left-pad to 32
            let mut out = [0u8; 32];
            out[24..32].copy_from_slice(&n.to_be_bytes());
            out.to_vec()
        }
        "int256" | "int" => {
            let n = value.as_i64().unwrap_or(0);
            let mut out = [0u8; 32];
            out[24..32].copy_from_slice(&n.to_be_bytes());
            out.to_vec()
        }
        "address" => {
            let s = value.as_str().unwrap_or("0x0000000000000000000000000000000000000000");
            let clean = s.trim_start_matches("0x");
            let bytes = hex::decode(clean).unwrap_or_else(|_| vec![0u8; 20]);
            let mut out = [0u8; 32];
            if bytes.len() == 20 {
                out[12..32].copy_from_slice(&bytes);
            }
            out.to_vec()
        }
        "bool" => {
            let b = value.as_bool().unwrap_or(false);
            let mut out = [0u8; 32];
            if b {
                out[31] = 1;
            }
            out.to_vec()
        }
        "string" => {
            let s = value.as_str().unwrap_or("");
            keccak256(s.as_bytes()).to_vec()
        }
        "bytes" => {
            let s = value.as_str().unwrap_or("");
            let clean = s.trim_start_matches("0x");
            let bytes = hex::decode(clean).unwrap_or_default();
            keccak256(&bytes).to_vec()
        }
        _ if field_type.starts_with("bytes") => {
            let s = value.as_str().unwrap_or("");
            let clean = s.trim_start_matches("0x");
            let bytes = hex::decode(clean).unwrap_or_default();
            let mut out = vec![0u8; 32];
            let len = bytes.len().min(32);
            out[..len].copy_from_slice(&bytes[..len]);
            out
        }
        _ => {
            if let Some(nested) = value.as_object() {
                let type_hash = keccak256(encode_type(field_type, types).as_bytes());
                let mut encoded = type_hash.to_vec();
                if let Some(type_fields) = types.get(field_type).and_then(|t| t.as_array()) {
                    for nested_field in type_fields {
                        let nested_name =
                            nested_field.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let nested_type =
                            nested_field.get("type").and_then(|t| t.as_str()).unwrap_or("bytes32");
                        let nested_val = nested.get(nested_name);
                        encoded.extend(encode_value(nested_type, nested_val, types));
                    }
                }
                keccak256(&encoded).to_vec()
            } else {
                [0u8; 32].to_vec()
            }
        }
    }
}

fn compute_domain_separator(domain: &Eip712Domain) -> [u8; 32] {
    let type_hash = keccak256(EIP712_DOMAIN_TYPE);

    let mut encoded = type_hash.to_vec();

    if let Some(name) = &domain.name {
        encoded.extend_from_slice(&keccak256(name.as_bytes()));
    }
    if let Some(version) = &domain.version {
        encoded.extend_from_slice(&keccak256(version.as_bytes()));
    }
    if let Some(chain_id) = domain.chain_id {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&chain_id.to_be_bytes::<32>());
        encoded.extend_from_slice(&buf);
    }
    if let Some(contract) = &domain.verifying_contract {
        let mut buf = [0u8; 32];
        buf[12..32].copy_from_slice(contract.as_slice());
        encoded.extend_from_slice(&buf);
    }
    if let Some(salt) = &domain.salt {
        encoded.extend_from_slice(salt.as_slice());
    }

    keccak256(&encoded)
}

fn eip712_signing_hash(domain_separator: &[u8; 32], struct_hash: &[u8; 32]) -> [u8; 32] {
    let mut data = vec![0x19, 0x01];
    data.extend_from_slice(domain_separator);
    data.extend_from_slice(struct_hash);
    keccak256(&data)
}

pub fn neunode_domain(chain_id: u64, verifying_contract: [u8; 20]) -> Eip712Domain {
    Eip712Domain {
        name: Some(Cow::Borrowed("Neunode")),
        version: Some(Cow::Borrowed("1")),
        chain_id: Some(U256::from(chain_id)),
        verifying_contract: Some(Address::from(verifying_contract)),
        salt: None,
    }
}

pub fn sign_typed_data(
    signing_key: &SigningKey,
    domain: &Eip712Domain,
    types: &serde_json::Value,
    message: &serde_json::Value,
) -> Result<Vec<u8>, CryptoError> {
    let primary_type = types
        .as_object()
        .and_then(|m| m.keys().find(|k| *k != "EIP712Domain").cloned())
        .ok_or_else(|| CryptoError::SerializationError("no primary type found".into()))?;

    let domain_sep = compute_domain_separator(domain);
    let struct_data = encode_data(&primary_type, types, message);
    let struct_hash = keccak256(&struct_data);
    let signing_hash = eip712_signing_hash(&domain_sep, &struct_hash);

    let sig: k256::ecdsa::Signature =
        signing_key.sign_prehash(&signing_hash).map_err(|_| CryptoError::InvalidSignature)?;

    let bytes = sig.to_bytes();
    let mut out = Vec::with_capacity(65);
    out.extend_from_slice(&bytes);
    out.push(0);
    Ok(out)
}

pub fn verify_typed_data(
    verifying_key: &VerifyingKey,
    domain: &Eip712Domain,
    types: &serde_json::Value,
    message: &serde_json::Value,
    signature: &[u8],
) -> Result<bool, CryptoError> {
    if signature.len() < 64 {
        return Ok(false);
    }

    let primary_type = types
        .as_object()
        .and_then(|m| m.keys().find(|k| *k != "EIP712Domain").cloned())
        .ok_or_else(|| CryptoError::SerializationError("no primary type found".into()))?;

    let domain_sep = compute_domain_separator(domain);
    let struct_data = encode_data(&primary_type, types, message);
    let struct_hash = keccak256(&struct_data);
    let signing_hash = eip712_signing_hash(&domain_sep, &struct_hash);

    let sig = k256::ecdsa::Signature::from_slice(&signature[..64])
        .map_err(|_| CryptoError::InvalidSignature)?;

    Ok(verifying_key.verify_prehash(&signing_hash, &sig).is_ok())
}

pub fn generate_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    use rand::RngCore;
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    nonce
}

pub fn is_timestamp_valid(challenge_ts: u64, now_ts: u64) -> bool {
    challenge_ts <= now_ts + TIMESTAMP_TOLERANCE_SECS
        && challenge_ts >= now_ts.saturating_sub(TIMESTAMP_TOLERANCE_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_domain() -> Eip712Domain {
        let contract_bytes: [u8; 20] = {
            let mut b = [0u8; 20];
            let hex_bytes = hex::decode("dca7ef03e98e0dc2b855be647c39abe984fcf21b").unwrap();
            b.copy_from_slice(&hex_bytes);
            b
        };
        neunode_domain(1, contract_bytes)
    }

    fn test_types() -> serde_json::Value {
        serde_json::json!({
            "Bounty": [
                {"name": "id", "type": "string"},
                {"name": "reward", "type": "uint256"},
                {"name": "deadline", "type": "uint256"}
            ]
        })
    }

    fn test_message() -> serde_json::Value {
        serde_json::json!({
            "id": "bnty_8f3a2c",
            "reward": 1000,
            "deadline": 1700000000
        })
    }

    #[test]
    fn domain_construction() {
        let domain = test_domain();
        assert_eq!(domain.name.as_deref(), Some("Neunode"));
        assert_eq!(domain.version.as_deref(), Some("1"));
        assert_eq!(domain.chain_id, Some(U256::from(1)));
        assert!(domain.verifying_contract.is_some());
    }

    #[test]
    fn sign_verify_roundtrip() {
        let (sk, vk) = crate::secp256k1::generate_keypair();
        let domain = test_domain();
        let types = test_types();
        let message = test_message();

        let sig = sign_typed_data(&sk, &domain, &types, &message).unwrap();
        assert_eq!(sig.len(), 65);

        let valid = verify_typed_data(&vk, &domain, &types, &message, &sig).unwrap();
        assert!(valid);
    }

    #[test]
    fn sign_verify_wrong_message_fails() {
        let (sk, vk) = crate::secp256k1::generate_keypair();
        let domain = test_domain();
        let types = test_types();

        let sig = sign_typed_data(&sk, &domain, &types, &test_message()).unwrap();

        let wrong_message = serde_json::json!({
            "id": "bnty_different",
            "reward": 999,
            "deadline": 1700000001
        });
        let valid = verify_typed_data(&vk, &domain, &types, &wrong_message, &sig).unwrap();
        assert!(!valid);
    }

    #[test]
    fn sign_verify_wrong_key_fails() {
        let (sk, _) = crate::secp256k1::generate_keypair();
        let (_, vk2) = crate::secp256k1::generate_keypair();
        let domain = test_domain();
        let types = test_types();
        let message = test_message();

        let sig = sign_typed_data(&sk, &domain, &types, &message).unwrap();
        let valid = verify_typed_data(&vk2, &domain, &types, &message, &sig).unwrap();
        assert!(!valid);
    }

    #[test]
    fn generate_nonce_is_32_bytes() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32);
    }

    #[test]
    fn generate_nonce_is_random() {
        let n1 = generate_nonce();
        let n2 = generate_nonce();
        assert_ne!(n1, n2);
    }

    #[test]
    fn timestamp_valid_within_window() {
        let now = 1700000000u64;
        assert!(is_timestamp_valid(now, now));
        assert!(is_timestamp_valid(now + 299, now));
        assert!(is_timestamp_valid(now.saturating_sub(299), now));
        assert!(is_timestamp_valid(now + 300, now));
        assert!(is_timestamp_valid(now.saturating_sub(300), now));
    }

    #[test]
    fn timestamp_invalid_outside_window() {
        let now = 1700000000u64;
        assert!(!is_timestamp_valid(now + 301, now));
        assert!(!is_timestamp_valid(now.saturating_sub(301), now));
        assert!(!is_timestamp_valid(now + 1000, now));
        assert!(!is_timestamp_valid(now.saturating_sub(1000), now));
    }

    #[test]
    fn compute_domain_separator_deterministic() {
        let domain = test_domain();
        let sep1 = compute_domain_separator(&domain);
        let sep2 = compute_domain_separator(&domain);
        assert_eq!(sep1, sep2);
    }

    #[test]
    fn different_domains_produce_different_separators() {
        let domain1 = test_domain();
        let domain2 = neunode_domain(137, [0u8; 20]);
        let sep1 = compute_domain_separator(&domain1);
        let sep2 = compute_domain_separator(&domain2);
        assert_ne!(sep1, sep2);
    }

    #[test]
    fn encode_type_simple() {
        let types = serde_json::json!({
            "Mail": [
                {"name": "from", "type": "address"},
                {"name": "to", "type": "address"},
                {"name": "contents", "type": "string"}
            ]
        });
        let encoded = encode_type("Mail", &types);
        assert_eq!(encoded, "Mail(address from,address to,string contents)");
    }

    #[test]
    fn encode_type_with_referenced_types() {
        let types = serde_json::json!({
            "Mail": [
                {"name": "from", "type": "Person"},
                {"name": "to", "type": "Person"},
                {"name": "contents", "type": "string"}
            ],
            "Person": [
                {"name": "name", "type": "string"},
                {"name": "wallet", "type": "address"}
            ]
        });
        let encoded = encode_type("Mail", &types);
        assert_eq!(
            encoded,
            "Mail(Person from,Person to,string contents)Person(string name,address wallet)"
        );
    }
}

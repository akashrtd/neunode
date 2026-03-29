use neunode_core::{Did, NeunodeError, PeerId, Result};

pub const DID_METHOD: &str = "neunode";

const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedDid {
    Key { pubkey_multibase: String },
    Neunode { address: String },
}

fn base58btc_encode(data: &[u8]) -> String {
    let leading_zeros = data.iter().take_while(|&&b| b == 0).count();

    let mut digits: Vec<u8> = Vec::new();
    for &byte in data {
        let mut carry = byte as u64;
        for d in digits.iter_mut() {
            carry += (*d as u64) * 256;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    let mut result = String::with_capacity(leading_zeros + digits.len());
    for _ in 0..leading_zeros {
        result.push('1');
    }
    for &d in digits.iter().rev() {
        result.push(BASE58_ALPHABET[d as usize] as char);
    }
    result
}

fn base58btc_decode(encoded: &str) -> Result<Vec<u8>> {
    let leading_zeros = encoded.chars().take_while(|&c| c == '1').count();
    let stripped: String = encoded.chars().skip_while(|&c| c == '1').collect();

    let mut bytes: Vec<u8> = Vec::new();

    for c in stripped.chars() {
        let digit = BASE58_ALPHABET.iter().position(|&b| b == c as u8).ok_or_else(|| {
            NeunodeError::EncodingError(format!("invalid base58btc character: {c}"))
        })? as u64;

        let mut carry = digit;
        for b in bytes.iter_mut() {
            carry += (*b as u64) * 58;
            *b = (carry & 0xFF) as u8;
            carry >>= 8;
        }
        while carry > 0 {
            bytes.push((carry & 0xFF) as u8);
            carry >>= 8;
        }
    }

    let mut out = vec![0u8; leading_zeros];
    for &b in bytes.iter().rev() {
        out.push(b);
    }
    Ok(out)
}

fn varint_encode(value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = value;
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
    out
}

pub fn generate_did_key(ed25519_pubkey: &ed25519_dalek::VerifyingKey) -> Did {
    let pubkey_bytes = ed25519_pubkey.to_bytes();
    let mut combined = Vec::with_capacity(34);
    combined.extend_from_slice(&[0xed, 0x01]);
    combined.extend_from_slice(&pubkey_bytes);

    let encoded = base58btc_encode(&combined);
    Did(format!("did:key:z{encoded}"))
}

pub fn generate_did_neunode(secp256k1_address: &str) -> Did {
    Did(format!("did:{DID_METHOD}:{secp256k1_address}"))
}

pub fn parse_did(did_string: &str) -> Result<ParsedDid> {
    let parts: Vec<&str> = did_string.splitn(3, ':').collect();
    if parts.len() != 3 || parts[0] != "did" {
        return Err(NeunodeError::InvalidDid(format!("invalid DID format: {did_string}")));
    }

    match parts[1] {
        "key" => {
            let id = parts[2];
            if !id.starts_with('z') {
                return Err(NeunodeError::InvalidDid(
                    "did:key identifier must start with 'z' (base58btc multibase)".into(),
                ));
            }
            if id.len() < 2 {
                return Err(NeunodeError::InvalidDid("did:key identifier is too short".into()));
            }
            Ok(ParsedDid::Key { pubkey_multibase: id.to_string() })
        }
        "neunode" => {
            let addr = parts[2];
            if !addr.starts_with("0x") || addr.len() != 42 {
                return Err(NeunodeError::InvalidDid(
                    "did:neunode identifier must be 0x-prefixed 42-char Ethereum address".into(),
                ));
            }
            if addr[2..].chars().any(|c| !c.is_ascii_hexdigit()) {
                return Err(NeunodeError::InvalidDid(
                    "did:neunode address contains non-hex characters".into(),
                ));
            }
            Ok(ParsedDid::Neunode { address: addr.to_string() })
        }
        other => Err(NeunodeError::InvalidDid(format!("unsupported DID method: {other}"))),
    }
}

pub fn validate_did(did: &Did) -> Result<()> {
    parse_did(did.as_str())?;
    Ok(())
}

pub fn did_to_peer_id(did: &Did) -> Result<PeerId> {
    let parsed = parse_did(did.as_str())?;

    match parsed {
        ParsedDid::Key { pubkey_multibase } => {
            let encoded = &pubkey_multibase[1..];
            let decoded = base58btc_decode(encoded)?;

            if decoded.len() != 34 || decoded[0] != 0xed || decoded[1] != 0x01 {
                return Err(NeunodeError::InvalidDid(
                    "invalid Ed25519 multicodec in did:key".into(),
                ));
            }
            let pubkey_bytes = &decoded[2..34];

            let mut protobuf = Vec::with_capacity(36);
            protobuf.extend_from_slice(&[0x08, 0x01]);
            protobuf.extend_from_slice(&[0x12, 0x20]);
            protobuf.extend_from_slice(pubkey_bytes);

            let len_varint = varint_encode(protobuf.len() as u64);
            let mut mh = Vec::with_capacity(1 + len_varint.len() + protobuf.len());
            mh.push(0x00);
            mh.extend_from_slice(&len_varint);
            mh.extend_from_slice(&protobuf);

            Ok(PeerId(base58btc_encode(&mh)))
        }
        ParsedDid::Neunode { .. } => Err(NeunodeError::InvalidDid(
            "cannot derive PeerId from did:neunode without DID document resolution".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neunode_crypto::ed25519::generate_keypair;
    use neunode_crypto::secp256k1::{generate_keypair as secp_gen, verifying_key_to_address};

    fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn generate_did_key_format() {
        let (_, vk) = generate_keypair();
        let did = generate_did_key(&vk);
        assert!(did.as_str().starts_with("did:key:z6Mk"));
        assert!(did.is_key());
        assert!(!did.is_neunode());
    }

    #[test]
    fn generate_did_key_deterministic() {
        let seed = [42u8; 32];
        let (_, vk) = neunode_crypto::ed25519::keypair_from_seed(&seed);
        let did1 = generate_did_key(&vk);
        let (_, vk2) = neunode_crypto::ed25519::keypair_from_seed(&seed);
        let did2 = generate_did_key(&vk2);
        assert_eq!(did1, did2);

        let seed2 = [99u8; 32];
        let (_, vk3) = neunode_crypto::ed25519::keypair_from_seed(&seed2);
        let did3 = generate_did_key(&vk3);
        assert_ne!(did1, did3);
    }

    #[test]
    fn generate_did_neunode_format() {
        let did = generate_did_neunode("0x7e5f4552091a69125d5dfcb7b8c2659029395bdf");
        assert!(did.is_neunode());
        assert!(!did.is_key());
        assert_eq!(did.as_str(), "did:neunode:0x7e5f4552091a69125d5dfcb7b8c2659029395bdf");
    }

    #[test]
    fn generate_did_neunode_from_keypair() {
        let (_, vk) = secp_gen();
        let addr = verifying_key_to_address(&vk);
        let addr_hex = format!("0x{}", bytes_to_hex(&addr));
        let did = generate_did_neunode(&addr_hex);
        assert!(did.is_neunode());
        assert!(did.as_str().starts_with("did:neunode:0x"));
        assert_eq!(did.as_str().len(), "did:neunode:0x".len() + 40);
    }

    #[test]
    fn parse_did_key_roundtrip() {
        let (_, vk) = generate_keypair();
        let did = generate_did_key(&vk);
        let parsed = parse_did(did.as_str()).expect("parse should succeed");
        match parsed {
            ParsedDid::Key { pubkey_multibase } => {
                assert!(pubkey_multibase.starts_with('z'));
                assert!(pubkey_multibase.len() > 40);
            }
            _ => panic!("expected ParsedDid::Key"),
        }
    }

    #[test]
    fn parse_did_neunode_roundtrip() {
        let did_str = "did:neunode:0x7e5f4552091a69125d5dfcb7b8c2659029395bdf";
        let parsed = parse_did(did_str).expect("parse should succeed");
        assert_eq!(
            parsed,
            ParsedDid::Neunode {
                address: "0x7e5f4552091a69125d5dfcb7b8c2659029395bdf".to_string()
            }
        );
    }

    #[test]
    fn validate_did_key_valid() {
        let (_, vk) = generate_keypair();
        let did = generate_did_key(&vk);
        assert!(validate_did(&did).is_ok());
    }

    #[test]
    fn validate_did_neunode_valid() {
        let did = Did("did:neunode:0x7e5f4552091a69125d5dfcb7b8c2659029395bdf".into());
        assert!(validate_did(&did).is_ok());
    }

    #[test]
    fn validate_did_invalid_format() {
        let cases = ["not-a-did", "didx:key:z6Mktest", "did::test", "did:key", ""];
        for case in cases {
            let did = Did(case.to_string());
            assert!(validate_did(&did).is_err(), "should fail: {case}");
        }
    }

    #[test]
    fn validate_did_neunode_invalid_address() {
        let cases = [
            "did:neunode:ABC",
            "did:neunode:0xABC",
            "did:neunode:0xGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
        ];
        for case in cases {
            let did = Did(case.to_string());
            assert!(validate_did(&did).is_err(), "should fail: {case}");
        }
    }

    #[test]
    fn validate_did_unsupported_method() {
        let did = Did("did:ethr:0x1234567890abcdef1234567890abcdef12345678".into());
        assert!(validate_did(&did).is_err());
    }

    #[test]
    fn did_to_peer_id_from_key() {
        let (_, vk) = generate_keypair();
        let did = generate_did_key(&vk);
        let peer_id = did_to_peer_id(&did).expect("peer id derivation");
        assert!(peer_id.as_str().starts_with("12D3Koo"));
        assert!(peer_id.as_str().len() > 40);
    }

    #[test]
    fn did_to_peer_id_deterministic() {
        let seed = [7u8; 32];
        let (_, vk) = neunode_crypto::ed25519::keypair_from_seed(&seed);
        let did = generate_did_key(&vk);
        let p1 = did_to_peer_id(&did).expect("p1");
        let p2 = did_to_peer_id(&did).expect("p2");
        assert_eq!(p1, p2);
    }

    #[test]
    fn did_to_peer_id_neunode_fails() {
        let did = Did("did:neunode:0x7e5f4552091a69125d5dfcb7b8c2659029395bdf".into());
        assert!(did_to_peer_id(&did).is_err());
    }

    #[test]
    fn base58btc_encode_decode_roundtrip() {
        let cases: Vec<Vec<u8>> = vec![
            b"hello".to_vec(),
            vec![],
            vec![0u8; 32],
            vec![0x01, 0x55, 0x12, 0x20],
            b"Hello World!".to_vec(),
            b"\x00\x00\x00\x05".to_vec(),
        ];
        for case in cases {
            let encoded = base58btc_encode(&case);
            let decoded = base58btc_decode(&encoded).expect("decode");
            assert_eq!(decoded, case, "roundtrip failed for {:?}", case);
        }
    }

    #[test]
    fn base58btc_single_byte() {
        let data = vec![97u8]; // 'a'
        let encoded = base58btc_encode(&data);
        let decoded = base58btc_decode(&encoded).expect("decode");
        assert_eq!(decoded, data);
    }

    #[test]
    fn base58btc_leading_zeros() {
        let data = vec![0u8, 0, 0, 5];
        let encoded = base58btc_encode(&data);
        assert!(encoded.starts_with("111"));
        let decoded = base58btc_decode(&encoded).expect("decode");
        assert_eq!(decoded, data);
    }

    #[test]
    fn peer_id_only_valid_base58_chars() {
        let (_, vk) = generate_keypair();
        let did = generate_did_key(&vk);
        let peer_id = did_to_peer_id(&did).expect("peer id");
        let pid_str = peer_id.as_str();
        assert!(pid_str.starts_with("12D3Koo"));
        for c in pid_str.chars() {
            assert!(
                "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c),
                "invalid char in PeerId: {c}"
            );
        }
    }

    #[test]
    fn different_keys_different_peer_ids() {
        let (_, vk1) = generate_keypair();
        let (_, vk2) = generate_keypair();
        let did1 = generate_did_key(&vk1);
        let did2 = generate_did_key(&vk2);
        let p1 = did_to_peer_id(&did1).expect("p1");
        let p2 = did_to_peer_id(&did2).expect("p2");
        assert_ne!(p1, p2);
    }

    #[test]
    fn varint_encoding() {
        assert_eq!(varint_encode(0), vec![0x00]);
        assert_eq!(varint_encode(1), vec![0x01]);
        assert_eq!(varint_encode(127), vec![0x7F]);
        assert_eq!(varint_encode(128), vec![0x80, 0x01]);
        assert_eq!(varint_encode(237), vec![0xED, 0x01]);
        assert_eq!(varint_encode(297), vec![0xA9, 0x02]);
    }
}

use sha2::{Digest, Sha256};

use crate::error::{LineageError, Result};

pub fn compute_content_hash(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    format!("sha256:{}", hex_encode(&hash))
}

pub fn verify_content_hash(data: &[u8], expected_hash: &str) -> bool {
    let actual = compute_content_hash(data);
    actual == expected_hash
}

/// Validates safetensors header and computes SHA-256 of the entire blob.
pub fn compute_safetensors_hash(data: &[u8]) -> Result<String> {
    if data.len() < 8 {
        return Err(LineageError::ConfigInvalid(
            "safetensors data too short for header".to_string(),
        ));
    }
    let header_len = u64::from_le_bytes(data[..8].try_into().expect("slice is 8 bytes"));
    let header_end = 8 + header_len as usize;
    if data.len() < header_end {
        return Err(LineageError::ConfigInvalid(
            "safetensors data truncated before header end".to_string(),
        ));
    }
    serde_json::from_slice::<serde_json::Value>(&data[8..header_end]).map_err(|e| {
        LineageError::ConfigInvalid(format!("invalid safetensors JSON header: {e}"))
    })?;
    Ok(compute_content_hash(data))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_content_hash_deterministic() {
        let data = b"hello neunode lineage";
        let h1 = compute_content_hash(data);
        let h2 = compute_content_hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_content_hash_different_inputs() {
        let h1 = compute_content_hash(b"input A");
        let h2 = compute_content_hash(b"input B");
        assert_ne!(h1, h2);
    }

    #[test]
    fn verify_content_hash_correct() {
        let data = b"test data for hashing";
        let hash = compute_content_hash(data);
        assert!(verify_content_hash(data, &hash));
    }

    #[test]
    fn verify_content_hash_wrong_data() {
        let hash = compute_content_hash(b"original data");
        assert!(!verify_content_hash(b"tampered data", &hash));
    }

    #[test]
    fn sha256_prefix_format() {
        let hash = compute_content_hash(b"test");
        assert!(hash.starts_with("sha256:"));
        let hex_part = &hash[7..];
        assert_eq!(hex_part.len(), 64);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn safetensors_hash_valid_header() {
        let header_json = r#"{"tensor1":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]}}"#;
        let header_bytes = header_json.as_bytes();
        let header_len = header_bytes.len() as u64;
        let mut data = Vec::new();
        data.extend_from_slice(&header_len.to_le_bytes());
        data.extend_from_slice(header_bytes);
        data.extend_from_slice(&[0u8; 16]);

        let hash = compute_safetensors_hash(&data).unwrap();
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn safetensors_hash_invalid_header_too_short() {
        let data = vec![0u8; 4];
        let result = compute_safetensors_hash(&data);
        assert!(result.is_err());
    }

    #[test]
    fn safetensors_hash_invalid_json() {
        let bad_json = b"not valid json!!!";
        let header_len = bad_json.len() as u64;
        let mut data = Vec::new();
        data.extend_from_slice(&header_len.to_le_bytes());
        data.extend_from_slice(bad_json);

        let result = compute_safetensors_hash(&data);
        assert!(result.is_err());
    }

    #[test]
    fn safetensors_hash_realistic_data() {
        let header = serde_json::json!({
            "__metadata__": {"format": "pt"},
            "model.embed_tokens.weight": {
                "dtype": "F16",
                "shape": [32000, 4096],
                "data_offsets": [0, 262144000]
            }
        });
        let header_str = serde_json::to_string(&header).unwrap();
        let header_bytes = header_str.as_bytes();
        let header_len = header_bytes.len() as u64;
        let mut data = Vec::new();
        data.extend_from_slice(&header_len.to_le_bytes());
        data.extend_from_slice(header_bytes);

        let hash = compute_safetensors_hash(&data).unwrap();
        assert!(hash.starts_with("sha256:"));
        assert_eq!(&hash[7..].len(), &64);
    }

    #[test]
    fn empty_data_hash() {
        let hash = compute_content_hash(b"");
        assert!(hash.starts_with("sha256:"));
        assert!(verify_content_hash(b"", &hash));
    }

    #[test]
    fn empty_data_safetensors_too_short() {
        let result = compute_safetensors_hash(b"");
        assert!(result.is_err());
    }

    #[test]
    fn safetensors_header_truncated() {
        let header_json = r#"{"test":1}"#;
        let header_bytes = header_json.as_bytes();
        let mut fake_len = [0u8; 8];
        let inflated_len = (header_bytes.len() as u64) * 10;
        fake_len.copy_from_slice(&inflated_len.to_le_bytes());
        let mut data = Vec::new();
        data.extend_from_slice(&fake_len);
        data.extend_from_slice(header_bytes);

        let result = compute_safetensors_hash(&data);
        assert!(result.is_err());
    }

    #[test]
    fn known_sha256_empty() {
        let hash = compute_content_hash(b"");
        assert_eq!(
            &hash[7..],
            "e3b0c44298fc1c149afbf4c8996fb924\
             27ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn known_sha256_abc() {
        let hash = compute_content_hash(b"abc");
        assert_eq!(
            &hash[7..],
            "ba7816bf8f01cfea414140de5dae2223\
             b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_rejects_wrong_prefix() {
        assert!(!verify_content_hash(b"test", "sha512:abc"));
    }

    #[test]
    fn safetensors_zero_length_header() {
        let data = vec![0u8; 8];
        // header_len=0 → empty JSON → invalid, so we expect error
        let result = compute_safetensors_hash(&data);
        assert!(result.is_err());
    }
}

use crate::error::{P2pError, Result};

const MAGIC: &[u8; 7] = b"NNZSTD\x01";
const HEADER_LEN: usize = MAGIC.len() + size_of::<u32>();
const COMPRESSION_THRESHOLD: usize = 256;
const ZSTD_LEVEL: i32 = 3;

/// Encode a payload for the wire, compressing only when the complete frame is smaller.
///
/// Payloads below the threshold and incompressible payloads remain byte-for-byte compatible
/// with legacy peers. Compressed frames carry a versioned magic prefix and decoded length.
pub fn encode(data: &[u8], max_decoded_size: usize) -> Result<Vec<u8>> {
    ensure_size(data.len(), max_decoded_size)?;
    if data.len() < COMPRESSION_THRESHOLD {
        return Ok(data.to_vec());
    }

    let compressed = zstd::bulk::compress(data, ZSTD_LEVEL)
        .map_err(|error| P2pError::WireFormat(format!("zstd compression failed: {error}")))?;
    if compressed.len().saturating_add(HEADER_LEN) >= data.len() {
        return Ok(data.to_vec());
    }

    let decoded_len = u32::try_from(data.len())
        .map_err(|_| P2pError::WireFormat("payload length exceeds wire format".to_string()))?;
    let mut frame = Vec::with_capacity(HEADER_LEN + compressed.len());
    frame.extend_from_slice(MAGIC);
    frame.extend_from_slice(&decoded_len.to_be_bytes());
    frame.extend_from_slice(&compressed);
    Ok(frame)
}

/// Decode a wire payload with a hard ceiling on decompressed output.
///
/// Unframed data is returned unchanged for rolling-upgrade compatibility. A payload beginning
/// with the Neunode compression magic must be a valid frame; malformed frames fail closed.
pub fn decode(data: &[u8], max_decoded_size: usize) -> Result<Vec<u8>> {
    if !data.starts_with(MAGIC) {
        ensure_size(data.len(), max_decoded_size)?;
        return Ok(data.to_vec());
    }
    if data.len() <= HEADER_LEN {
        return Err(P2pError::WireFormat("truncated compressed frame".to_string()));
    }

    let length_bytes: [u8; 4] = data[MAGIC.len()..HEADER_LEN]
        .try_into()
        .map_err(|_| P2pError::WireFormat("invalid compressed length".to_string()))?;
    let decoded_len = u32::from_be_bytes(length_bytes) as usize;
    ensure_size(decoded_len, max_decoded_size)?;

    let decoded = zstd::bulk::decompress(&data[HEADER_LEN..], decoded_len)
        .map_err(|error| P2pError::WireFormat(format!("zstd decompression failed: {error}")))?;
    if decoded.len() != decoded_len {
        return Err(P2pError::WireFormat(format!(
            "decoded length {} does not match declared length {decoded_len}",
            decoded.len()
        )));
    }
    Ok(decoded)
}

fn ensure_size(actual: usize, maximum: usize) -> Result<()> {
    if actual > maximum {
        return Err(P2pError::WireFormat(format!(
            "decoded payload size {actual} exceeds maximum {maximum}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: usize = 1024 * 1024;

    #[test]
    fn small_payload_stays_legacy_compatible() {
        let input = b"small payload";
        assert_eq!(encode(input, MAX).unwrap(), input);
    }

    #[test]
    fn repetitive_payload_compresses_and_roundtrips() {
        let input = vec![b'a'; 32 * 1024];
        let encoded = encode(&input, MAX).unwrap();
        assert!(encoded.starts_with(MAGIC));
        assert!(encoded.len() < input.len() / 10);
        assert_eq!(decode(&encoded, MAX).unwrap(), input);
    }

    #[test]
    fn incompressible_payload_remains_unframed() {
        let mut state = 0x9e37_79b9_u32;
        let input: Vec<u8> = (0..4096)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let encoded = encode(&input, MAX).unwrap();
        assert!(!encoded.starts_with(MAGIC));
        assert_eq!(decode(&encoded, MAX).unwrap(), input);
    }

    #[test]
    fn legacy_unframed_payload_decodes() {
        assert_eq!(decode(b"legacy", MAX).unwrap(), b"legacy");
    }

    #[test]
    fn oversized_raw_payload_is_rejected() {
        let error = decode(&[0; 33], 32).unwrap_err();
        assert!(error.to_string().contains("exceeds maximum"));
    }

    #[test]
    fn declared_compression_bomb_is_rejected_before_decompression() {
        let mut frame = MAGIC.to_vec();
        frame.extend_from_slice(&1024_u32.to_be_bytes());
        frame.push(0);
        let error = decode(&frame, 128).unwrap_err();
        assert!(error.to_string().contains("exceeds maximum"));
    }

    #[test]
    fn truncated_frame_is_rejected() {
        let error = decode(MAGIC, MAX).unwrap_err();
        assert!(error.to_string().contains("truncated"));
    }

    #[test]
    fn corrupted_frame_is_rejected() {
        let input = vec![b'x'; 4096];
        let mut encoded = encode(&input, MAX).unwrap();
        let last = encoded.len() - 1;
        encoded[last] ^= 0xff;
        assert!(decode(&encoded, MAX).is_err());
    }
}

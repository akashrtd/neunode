use serde::de::DeserializeOwned;
use serde::Serialize;
pub type ReadError = bincode::Error;
pub type WriteError = bincode::Error;

/// Serialize using the repository's legacy bincode 1.3 wire format.
///
/// Keeping this dependency behind one boundary allows a future versioned migration without
/// coupling storage callers to a particular codec.
///
/// Byte compatibility is an invariant: these bytes are used both as RocksDB keys and values.
pub fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, WriteError> {
    bincode::serialize(value)
}

/// Deserialize bytes written by either bincode 1.3 or [`serialize`].
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ReadError> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_legacy_bincode_1_3_golden_bytes() {
        let value = (42_u32, "neunode".to_string(), vec![1_u8, 2, 3]);
        let expected = [
            42, 0, 0, 0, // fixed-width little-endian u32
            7, 0, 0, 0, 0, 0, 0, 0, b'n', b'e', b'u', b'n', b'o', b'd', b'e', // String
            3, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, // Vec<u8>
        ];

        assert_eq!(serialize(&value).unwrap(), expected);
        assert_eq!(deserialize::<(u32, String, Vec<u8>)>(&expected).unwrap(), value);
    }

    #[test]
    fn preserves_legacy_enum_discriminants() {
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        enum LegacyState {
            First,
            Second(u64),
        }

        let expected = [1, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(serialize(&LegacyState::Second(9)).unwrap(), expected);
        assert_eq!(deserialize::<LegacyState>(&expected).unwrap(), LegacyState::Second(9));
    }
}

use thiserror::Error;

/// Errors returned by TurboQuant compression operations.
#[derive(Debug, Error)]
pub enum TurboQuantError {
    #[error("invalid bit width: {bits} bits — {reason}")]
    InvalidBitWidth { bits: u32, reason: String },

    #[error("dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("codebook not initialized")]
    CodebookNotInitialized,

    #[error("rotation failed: {0}")]
    RotationFailed(String),

    #[error("quantization failed: {0}")]
    QuantizationFailed(String),

    #[error("buffer too small: need {needed} bytes, have {available}")]
    BufferTooSmall { needed: usize, available: usize },

    #[error("rotation matrix not initialized")]
    RotationNotInitialized,

    #[error("codebook has no levels")]
    CodebookEmpty,
}

/// Result type alias for TurboQuant operations.
pub type Result<T> = std::result::Result<T, TurboQuantError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_invalid_bit_width() {
        let err = TurboQuantError::InvalidBitWidth {
            bits: 0,
            reason: "must be between 1 and 8".to_string(),
        };
        assert_eq!(format!("{err}"), "invalid bit width: 0 bits — must be between 1 and 8");
    }

    #[test]
    fn error_display_dimension_mismatch() {
        let err = TurboQuantError::DimensionMismatch { expected: 768, actual: 512 };
        assert_eq!(format!("{err}"), "dimension mismatch: expected 768, got 512");
    }

    #[test]
    fn error_display_codebook_not_initialized() {
        let err = TurboQuantError::CodebookNotInitialized;
        assert_eq!(format!("{err}"), "codebook not initialized");
    }

    #[test]
    fn error_display_rotation_failed() {
        let err = TurboQuantError::RotationFailed("matrix is singular".to_string());
        assert_eq!(format!("{err}"), "rotation failed: matrix is singular");
    }

    #[test]
    fn error_display_quantization_failed() {
        let err = TurboQuantError::QuantizationFailed("value out of range".to_string());
        assert_eq!(format!("{err}"), "quantization failed: value out of range");
    }

    #[test]
    fn error_display_buffer_too_small() {
        let err = TurboQuantError::BufferTooSmall { needed: 4096, available: 1024 };
        assert_eq!(format!("{err}"), "buffer too small: need 4096 bytes, have 1024");
    }

    #[test]
    fn error_display_rotation_not_initialized() {
        let err = TurboQuantError::RotationNotInitialized;
        assert_eq!(format!("{err}"), "rotation matrix not initialized");
    }

    #[test]
    fn error_display_codebook_empty() {
        let err = TurboQuantError::CodebookEmpty;
        assert_eq!(format!("{err}"), "codebook has no levels");
    }

    #[test]
    fn result_ok() {
        let res: u32 = 42;
        assert_eq!(res, 42);
    }

    #[test]
    fn result_err() {
        let res: Result<u32> = Err(TurboQuantError::CodebookNotInitialized);
        assert!(res.is_err());
    }
}

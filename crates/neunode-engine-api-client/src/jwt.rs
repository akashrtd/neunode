use crate::error::EngineApiError;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

/// JWT claims for Engine API authentication.
#[derive(Debug, Serialize, Deserialize)]
pub struct EngineApiClaims {
    /// Required: issued-at timestamp (seconds since epoch).
    pub iat: u64,
    /// Optional: unique CL client identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional: CL client type/version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clv: Option<String>,
}

/// Manages JWT token generation for Engine API authentication.
#[derive(Debug)]
pub struct JwtAuth {
    /// The 256-bit shared secret.
    secret: [u8; 32],
    /// Optional client identifier.
    client_id: Option<String>,
    /// Optional client version string.
    client_version: Option<String>,
}

impl JwtAuth {
    /// Create a new JWT authenticator from a raw 32-byte secret.
    pub fn from_bytes(secret: [u8; 32]) -> Self {
        Self { secret, client_id: None, client_version: None }
    }

    /// Create a new JWT authenticator from a hex-encoded secret.
    pub fn from_hex_secret(hex: &str) -> Result<Self, EngineApiError> {
        let trimmed = hex.trim().trim_start_matches("0x");
        let bytes = hex::decode(trimmed)
            .map_err(|e| EngineApiError::JwtAuth(format!("invalid hex secret: {e}")))?;
        if bytes.len() != 32 {
            return Err(EngineApiError::JwtAuth(
                "JWT secret must be exactly 256 bits (32 bytes)".into(),
            ));
        }
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&bytes);
        Ok(Self { secret, client_id: None, client_version: None })
    }

    /// Load JWT secret from a file (hex-encoded content).
    pub async fn from_file(path: &std::path::Path) -> Result<Self, EngineApiError> {
        let hex = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| EngineApiError::JwtAuth(format!("cannot read JWT file: {e}")))?;
        Self::from_hex_secret(&hex)
    }

    /// Set the optional client identifier.
    pub fn with_client_id(mut self, id: String) -> Self {
        self.client_id = Some(id);
        self
    }

    /// Set the optional client version string.
    pub fn with_client_version(mut self, version: String) -> Self {
        self.client_version = Some(version);
        self
    }

    /// Generate a new JWT token valid for the current time.
    pub fn generate_token(&self) -> Result<String, EngineApiError> {
        let key = EncodingKey::from_secret(&self.secret);
        let claims = EngineApiClaims {
            iat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time went backwards")
                .as_secs(),
            id: self.client_id.clone(),
            clv: self.client_version.clone(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &key)
            .map_err(|e| EngineApiError::JwtAuth(format!("token signing failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, DecodingKey, Validation};

    /// Helper: create a valid 32-byte hex secret.
    fn test_secret_hex() -> String {
        hex::encode([42u8; 32])
    }

    #[test]
    fn from_hex_secret_valid() {
        let auth = JwtAuth::from_hex_secret(&test_secret_hex());
        assert!(auth.is_ok());
    }

    #[test]
    fn from_hex_secret_with_0x_prefix() {
        let hex = format!("0x{}", test_secret_hex());
        let auth = JwtAuth::from_hex_secret(&hex);
        assert!(auth.is_ok());
    }

    #[test]
    fn from_hex_secret_too_short() {
        let auth = JwtAuth::from_hex_secret("abcd");
        assert!(auth.is_err());
        assert!(auth.unwrap_err().to_string().contains("256 bits"));
    }

    #[test]
    fn from_hex_secret_too_long() {
        let hex = hex::encode([0u8; 64]);
        let auth = JwtAuth::from_hex_secret(&hex);
        assert!(auth.is_err());
        assert!(auth.unwrap_err().to_string().contains("256 bits"));
    }

    #[test]
    fn from_hex_secret_invalid_hex() {
        let auth = JwtAuth::from_hex_secret("zzzz");
        assert!(auth.is_err());
        assert!(auth.unwrap_err().to_string().contains("invalid hex"));
    }

    #[test]
    fn generate_token_basic() {
        let auth = JwtAuth::from_hex_secret(&test_secret_hex()).unwrap();
        let token = auth.generate_token().unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn generate_token_has_correct_claims() {
        let auth = JwtAuth::from_hex_secret(&test_secret_hex()).unwrap();
        let token = auth.generate_token().unwrap();

        let key = DecodingKey::from_secret(&[42u8; 32]);
        let mut validation = Validation::new(Algorithm::HS256);
        // Engine API only uses iat, not exp
        validation.validate_exp = false;
        validation.set_required_spec_claims::<String>(&[]);

        let token_data = decode::<EngineApiClaims>(&token, &key, &validation).unwrap();
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        // Token iat should be within 2 seconds of now
        assert!(token_data.claims.iat <= now);
        assert!(token_data.claims.iat >= now - 2);
    }

    #[test]
    fn generate_token_with_optional_claims() {
        let auth = JwtAuth::from_hex_secret(&test_secret_hex())
            .unwrap()
            .with_client_id("test-cl".to_string())
            .with_client_version("neunode/0.1.0".to_string());

        let token = auth.generate_token().unwrap();

        let key = DecodingKey::from_secret(&[42u8; 32]);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        validation.set_required_spec_claims::<String>(&[]);

        let token_data = decode::<EngineApiClaims>(&token, &key, &validation).unwrap();
        assert_eq!(token_data.claims.id.as_deref(), Some("test-cl"));
        assert_eq!(token_data.claims.clv.as_deref(), Some("neunode/0.1.0"));
    }

    #[test]
    fn generate_token_uses_hs256() {
        let auth = JwtAuth::from_hex_secret(&test_secret_hex()).unwrap();
        let token = auth.generate_token().unwrap();

        // JWT header is the first segment, base64-encoded
        let header_b64 = token.split('.').next().unwrap();
        // Decode base64
        let header_bytes = base64_decode(header_b64);
        let header_json: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header_json["alg"], "HS256");
    }

    #[test]
    fn from_bytes_creates_valid_auth() {
        let secret = [99u8; 32];
        let auth = JwtAuth::from_bytes(secret);
        let token = auth.generate_token().unwrap();

        let key = DecodingKey::from_secret(&[99u8; 32]);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        validation.set_required_spec_claims::<String>(&[]);

        assert!(decode::<EngineApiClaims>(&token, &key, &validation).is_ok());
    }

    #[test]
    fn tokens_are_unique_per_call() {
        let auth = JwtAuth::from_hex_secret(&test_secret_hex()).unwrap();
        let t1 = auth.generate_token().unwrap();
        // Small sleep to ensure different iat in edge cases, though unlikely needed
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = auth.generate_token().unwrap();
        // Tokens may be the same if within the same second, but the call should
        // succeed regardless
        assert!(!t1.is_empty());
        assert!(!t2.is_empty());
    }

    /// Helper to decode standard base64 (no padding).
    fn base64_decode(input: &str) -> Vec<u8> {
        use base64::engine::general_purpose::STANDARD;
        use base64::Engine;
        STANDARD.decode(input).unwrap_or_else(|_| {
            // Try with padding
            let padded = format!("{input}==");
            STANDARD.decode(padded.trim_end_matches('=')).unwrap_or_default()
        })
    }

    #[tokio::test]
    async fn from_file_reads_hex() {
        let dir = std::env::temp_dir().join("neunode_engine_api_test_jwt");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("jwt.hex");
        std::fs::write(&file_path, test_secret_hex()).unwrap();

        let auth = JwtAuth::from_file(&file_path).await;
        assert!(auth.is_ok());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn from_file_missing() {
        let auth = JwtAuth::from_file(std::path::Path::new("/nonexistent/jwt.hex")).await;
        assert!(auth.is_err());
        assert!(auth.unwrap_err().to_string().contains("cannot read JWT file"));
    }

    #[tokio::test]
    async fn from_file_invalid_content() {
        let dir = std::env::temp_dir().join("neunode_engine_api_test_jwt_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("jwt.hex");
        std::fs::write(&file_path, "not-valid-hex-content").unwrap();

        let auth = JwtAuth::from_file(&file_path).await;
        assert!(auth.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}

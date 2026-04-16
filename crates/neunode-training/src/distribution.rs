use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use neunode_core::types::CID;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use ts_rs::TS;

use crate::error::{Result, TrainingError};

/// Reference to a single chunk in a sharded checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ChunkRef {
    #[ts(type = "number")]
    pub index: u32,
    #[ts(type = "number")]
    pub offset: u64,
    #[ts(type = "number")]
    pub size: u64,
    pub blake3_hash: String,
}

/// Manifest describing a sharded checkpoint for distribution.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChunkManifest {
    pub version: u8,
    pub cid: CID,
    #[ts(type = "number")]
    pub total_size: u64,
    #[ts(type = "number")]
    pub chunk_size: u64,
    pub chunks: Vec<ChunkRef>,
    pub blake3_root: String,
    #[ts(type = "number")]
    pub created_at: u64,
}

/// Configuration for checkpoint distribution.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistributionConfig {
    #[ts(type = "number")]
    pub chunk_size: u64,
    #[ts(type = "number")]
    pub max_concurrent_downloads: usize,
    #[ts(type = "number")]
    pub retry_max: u32,
    #[ts(type = "number")]
    pub retry_delay_ms: u64,
    pub bind_addr: String,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            chunk_size: 50_000_000, // 50MB
            max_concurrent_downloads: 10,
            retry_max: 3,
            retry_delay_ms: 1000,
            bind_addr: "0.0.0.0:9090".to_string(),
        }
    }
}

pub struct ChunkStore {
    base_dir: PathBuf,
}

impl ChunkStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn chunk_dir(&self, cid: &CID) -> PathBuf {
        self.base_dir.join("chunks").join(cid.0.replace(':', "_"))
    }

    pub fn chunk_path(&self, cid: &CID, index: u32) -> PathBuf {
        self.chunk_dir(cid).join(format!("chunk_{index:08}.bin"))
    }

    pub fn store_chunk(&self, cid: &CID, index: u32, data: &[u8]) -> Result<()> {
        let dir = self.chunk_dir(cid);
        std::fs::create_dir_all(&dir).map_err(|e| TrainingError::TransferFailed(e.to_string()))?;
        let path = self.chunk_path(cid, index);
        std::fs::write(&path, data).map_err(|e| TrainingError::TransferFailed(e.to_string()))
    }

    pub fn load_chunk(&self, cid: &CID, index: u32) -> Result<Vec<u8>> {
        let path = self.chunk_path(cid, index);
        std::fs::read(&path).map_err(|_| TrainingError::ChunkMissing { index, cid: cid.0.clone() })
    }

    pub fn chunk_exists(&self, cid: &CID, index: u32) -> bool {
        self.chunk_path(cid, index).exists()
    }

    pub fn manifest_path(&self, cid: &CID) -> PathBuf {
        self.chunk_dir(cid).join("manifest.json")
    }

    pub fn store_manifest(&self, manifest: &ChunkManifest) -> Result<()> {
        let dir = self.chunk_dir(&manifest.cid);
        std::fs::create_dir_all(&dir).map_err(|e| TrainingError::TransferFailed(e.to_string()))?;
        let path = self.manifest_path(&manifest.cid);
        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| TrainingError::ManifestInvalid(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| TrainingError::TransferFailed(e.to_string()))
    }

    pub fn load_manifest(&self, cid: &CID) -> Result<ChunkManifest> {
        let path = self.manifest_path(cid);
        let json = std::fs::read_to_string(&path)
            .map_err(|e| TrainingError::ManifestInvalid(e.to_string()))?;
        serde_json::from_str(&json).map_err(|e| TrainingError::ManifestInvalid(e.to_string()))
    }

    /// Clone with a new base_dir (for tests that need to own the tempdir).
    #[cfg(test)]
    pub fn clone_with_dir(&self) -> Self {
        Self { base_dir: self.base_dir.clone() }
    }
}

pub fn shard_checkpoint(
    data: &[u8],
    cid: &CID,
    config: &DistributionConfig,
) -> Result<(ChunkManifest, Vec<Vec<u8>>)> {
    let total_size = data.len() as u64;
    let chunk_size = config.chunk_size;
    let blake3_root = hex::encode(blake3::hash(data).as_bytes());

    let mut chunks_data = Vec::new();
    let mut chunk_refs = Vec::new();
    let mut offset = 0u64;
    let mut index = 0u32;

    while offset < total_size {
        let end = (offset + chunk_size).min(total_size);
        let chunk = data[offset as usize..end as usize].to_vec();
        let hash = blake3::hash(&chunk);
        let hash_hex = hex::encode(hash.as_bytes());

        chunk_refs.push(ChunkRef {
            index,
            offset,
            size: chunk.len() as u64,
            blake3_hash: hash_hex,
        });
        chunks_data.push(chunk);

        offset = end;
        index += 1;
    }

    let manifest = ChunkManifest {
        version: 1,
        cid: cid.clone(),
        total_size,
        chunk_size,
        chunks: chunk_refs,
        blake3_root,
        created_at: chrono::Utc::now().timestamp_millis() as u64,
    };

    Ok((manifest, chunks_data))
}

pub fn verify_chunk(chunk: &[u8], expected_hash: &str) -> Result<()> {
    let actual = hex::encode(blake3::hash(chunk).as_bytes());
    if actual == expected_hash {
        Ok(())
    } else {
        Err(TrainingError::HashMismatch { expected: expected_hash.to_string(), actual })
    }
}

pub fn verify_whole(data: &[u8], expected_root: &str) -> Result<()> {
    let actual = hex::encode(blake3::hash(data).as_bytes());
    if actual == expected_root {
        Ok(())
    } else {
        Err(TrainingError::HashMismatch { expected: expected_root.to_string(), actual })
    }
}

// ── CheckpointServer ─────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ServerState {
    chunk_store: Arc<ChunkStore>,
}

pub struct CheckpointServer {
    chunk_store: Arc<ChunkStore>,
    bind_addr: String,
}

impl CheckpointServer {
    pub fn new(chunk_store: ChunkStore, bind_addr: String) -> Self {
        Self { chunk_store: Arc::new(chunk_store), bind_addr }
    }

    pub fn router(&self) -> Router {
        let state = ServerState { chunk_store: Arc::clone(&self.chunk_store) };
        Router::new()
            .route("/v1/checkpoints/{cid}/manifest", get(get_manifest))
            .route("/v1/checkpoints/{cid}/chunks/{index}", get(get_chunk))
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    pub async fn serve(self) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(&self.bind_addr)
            .await
            .map_err(|e| TrainingError::ServerUnavailable(e.to_string()))?;
        tracing::info!("CheckpointServer listening on {}", self.bind_addr);
        axum::serve(listener, self.router())
            .await
            .map_err(|e| TrainingError::ServerUnavailable(e.to_string()))
    }

    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }
}

async fn get_manifest(
    State(state): State<ServerState>,
    AxumPath(cid_str): AxumPath<String>,
) -> Response {
    let cid = CID(cid_str);
    match state.chunk_store.load_manifest(&cid) {
        Ok(manifest) => (StatusCode::OK, Json(manifest)).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn get_chunk(
    State(state): State<ServerState>,
    AxumPath((cid_str, index)): AxumPath<(String, u32)>,
) -> Response {
    let cid = CID(cid_str);
    match state.chunk_store.load_chunk(&cid, index) {
        Ok(data) => (StatusCode::OK, data).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── CheckpointClient ─────────────────────────────────────────────────────

pub struct CheckpointClient {
    http: reqwest::Client,
    config: DistributionConfig,
}

impl CheckpointClient {
    pub fn new(config: DistributionConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { http, config }
    }

    pub async fn fetch_manifest(&self, base_url: &str, cid: &CID) -> Result<ChunkManifest> {
        let url = format!("{base_url}/v1/checkpoints/{}/manifest", cid.0);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| TrainingError::ServerUnavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TrainingError::ManifestInvalid(format!("HTTP {}", resp.status())));
        }
        resp.json().await.map_err(|e| TrainingError::ManifestInvalid(e.to_string()))
    }

    pub async fn fetch_chunk(&self, base_url: &str, cid: &CID, index: u32) -> Result<Vec<u8>> {
        let url = format!("{base_url}/v1/checkpoints/{}/chunks/{index}", cid.0);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| TrainingError::ServerUnavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(TrainingError::ChunkMissing { index, cid: cid.0.clone() });
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| TrainingError::TransferFailed(e.to_string()))
    }

    pub async fn download_checkpoint(
        &self,
        base_url: &str,
        cid: &CID,
        chunk_store: &ChunkStore,
    ) -> Result<()> {
        let manifest = self.fetch_manifest(base_url, cid).await?;

        for chunk_ref in &manifest.chunks {
            if chunk_store.chunk_exists(cid, chunk_ref.index) {
                continue;
            }

            let mut last_err = None;
            for attempt in 0..=self.config.retry_max {
                match self.fetch_chunk(base_url, cid, chunk_ref.index).await {
                    Ok(data) => {
                        if let Err(e) = verify_chunk(&data, &chunk_ref.blake3_hash) {
                            last_err = Some(e);
                            continue;
                        }
                        chunk_store.store_chunk(cid, chunk_ref.index, &data)?;
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e);
                        if attempt < self.config.retry_max {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                self.config.retry_delay_ms,
                            ))
                            .await;
                        }
                    }
                }
            }
            if let Some(e) = last_err {
                return Err(e);
            }
        }

        Ok(())
    }
}

// ── RelayNode ────────────────────────────────────────────────────────────

pub struct RelayNode {
    upstream_url: String,
    chunk_store: ChunkStore,
    client: CheckpointClient,
    config: DistributionConfig,
}

impl RelayNode {
    pub fn new(upstream_url: String, config: DistributionConfig, base_dir: PathBuf) -> Self {
        let chunk_store = ChunkStore::new(base_dir);
        let client = CheckpointClient::new(config.clone());
        Self { upstream_url, chunk_store, client, config }
    }

    pub async fn mirror(&self, cid: &CID) -> Result<()> {
        let manifest = self.client.fetch_manifest(&self.upstream_url, cid).await?;
        self.chunk_store.store_manifest(&manifest)?;

        self.client.download_checkpoint(&self.upstream_url, cid, &self.chunk_store).await?;

        Ok(())
    }

    pub async fn serve(self) -> Result<()> {
        let server = CheckpointServer::new(self.chunk_store, self.config.bind_addr.clone());
        server.serve().await
    }

    pub fn bind_addr(&self) -> &str {
        &self.config.bind_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::compute_cid;
    use tower::ServiceExt;

    #[allow(dead_code)]
    fn test_config() -> DistributionConfig {
        DistributionConfig { chunk_size: 100, ..Default::default() }
    }

    #[test]
    fn chunk_ref_serde_roundtrip() {
        let cr = ChunkRef { index: 3, offset: 150, size: 50, blake3_hash: "abc123".to_string() };
        let json = serde_json::to_string(&cr).unwrap();
        let back: ChunkRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.index, 3);
        assert_eq!(back.offset, 150);
        assert_eq!(back.size, 50);
        assert_eq!(back.blake3_hash, "abc123");
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let cid = CID::from_blake3_hex("deadbeef");
        let manifest = ChunkManifest {
            version: 1,
            cid: cid.clone(),
            total_size: 500,
            chunk_size: 100,
            chunks: vec![ChunkRef {
                index: 0,
                offset: 0,
                size: 100,
                blake3_hash: "hash0".to_string(),
            }],
            blake3_root: "roothash".to_string(),
            created_at: 1700000000000,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: ChunkManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, 1);
        assert_eq!(back.cid, cid);
        assert_eq!(back.total_size, 500);
        assert_eq!(back.chunks.len(), 1);
    }

    #[test]
    fn distribution_config_default() {
        let config = DistributionConfig::default();
        assert_eq!(config.chunk_size, 50_000_000);
        assert_eq!(config.max_concurrent_downloads, 10);
        assert_eq!(config.retry_max, 3);
        assert_eq!(config.retry_delay_ms, 1000);
        assert_eq!(config.bind_addr, "0.0.0.0:9090");
    }

    #[test]
    fn chunk_store_store_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkStore::new(dir.path().to_path_buf());
        let cid = CID::from_blake3_hex("testcid");
        let data = b"chunk data here";

        store.store_chunk(&cid, 0, data).unwrap();
        assert!(store.chunk_exists(&cid, 0));
        let loaded = store.load_chunk(&cid, 0).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn chunk_store_missing_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkStore::new(dir.path().to_path_buf());
        let cid = CID::from_blake3_hex("missing");
        let result = store.load_chunk(&cid, 99);
        assert!(result.is_err());
    }

    #[test]
    fn chunk_store_manifest_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ChunkStore::new(dir.path().to_path_buf());
        let cid = CID::from_blake3_hex("manifesttest");
        let manifest = ChunkManifest {
            version: 1,
            cid: cid.clone(),
            total_size: 300,
            chunk_size: 100,
            chunks: vec![
                ChunkRef { index: 0, offset: 0, size: 100, blake3_hash: "h0".to_string() },
                ChunkRef { index: 1, offset: 100, size: 100, blake3_hash: "h1".to_string() },
                ChunkRef { index: 2, offset: 200, size: 100, blake3_hash: "h2".to_string() },
            ],
            blake3_root: "root".to_string(),
            created_at: 1700000000000,
        };

        store.store_manifest(&manifest).unwrap();
        let loaded = store.load_manifest(&cid).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.chunks.len(), 3);
        assert_eq!(loaded.cid, cid);
    }

    #[test]
    fn shard_checkpoint_exact_boundary() {
        let config = DistributionConfig { chunk_size: 10, ..Default::default() };
        let data = b"0123456789";
        let cid = compute_cid(data);
        let (manifest, chunks) = shard_checkpoint(data, &cid, &config).unwrap();

        assert_eq!(manifest.total_size, 10);
        assert_eq!(manifest.chunks.len(), 1);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 10);
    }

    #[test]
    fn shard_checkpoint_multiple_chunks() {
        let config = DistributionConfig { chunk_size: 10, ..Default::default() };
        let data = b"0123456789012345678901234567";
        let cid = compute_cid(data);
        let (manifest, chunks) = shard_checkpoint(data, &cid, &config).unwrap();

        assert_eq!(manifest.total_size, 28);
        assert_eq!(manifest.chunks.len(), 3);
        assert!(manifest.chunks[0].size == 10);
        assert!(manifest.chunks[1].size == 10);
        assert!(manifest.chunks[2].size == 8);
        assert_eq!(chunks[0].len(), 10);
        assert_eq!(chunks[1].len(), 10);
        assert_eq!(chunks[2].len(), 8);
    }

    #[test]
    fn shard_checkpoint_single_byte() {
        let config = DistributionConfig { chunk_size: 100, ..Default::default() };
        let data = b"x";
        let cid = compute_cid(data);
        let (manifest, chunks) = shard_checkpoint(data, &cid, &config).unwrap();
        assert_eq!(manifest.chunks.len(), 1);
        assert_eq!(chunks[0], b"x");
    }

    #[test]
    fn verify_chunk_valid() {
        let data = b"hello world";
        let hash = hex::encode(blake3::hash(data).as_bytes());
        assert!(verify_chunk(data, &hash).is_ok());
    }

    #[test]
    fn verify_chunk_tampered() {
        let data = b"hello world";
        let hash = hex::encode(blake3::hash(data).as_bytes());
        let tampered = b"hello warld";
        let result = verify_chunk(tampered, &hash);
        assert!(result.is_err());
    }

    #[test]
    fn verify_whole_valid() {
        let data = b"entire checkpoint data";
        let hash = hex::encode(blake3::hash(data).as_bytes());
        assert!(verify_whole(data, &hash).is_ok());
    }

    #[test]
    fn verify_whole_tampered() {
        let data = b"entire checkpoint data";
        let hash = hex::encode(blake3::hash(data).as_bytes());
        let tampered = b"entire checkpoint deta";
        let result = verify_whole(tampered, &hash);
        assert!(result.is_err());
    }

    #[test]
    fn ts_export_types() {
        use ts_rs::Config;
        let cfg = Config::new();
        assert!(!ChunkRef::name(&cfg).is_empty());
        assert!(!ChunkManifest::name(&cfg).is_empty());
        assert!(!DistributionConfig::name(&cfg).is_empty());
    }

    // ── Server tests (tower one-shot, no real TCP) ──────────────────

    async fn setup_server() -> (ChunkStore, Router, CID, ChunkManifest) {
        let dir = tempfile::tempdir().unwrap().keep();
        let store = ChunkStore::new(dir);
        let config = DistributionConfig { chunk_size: 10, ..Default::default() };
        let data = b"01234567890123456789012345678901234";
        let cid = compute_cid(data);
        let (manifest, chunks) = shard_checkpoint(data, &cid, &config).unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            store.store_chunk(&cid, i as u32, chunk).unwrap();
        }
        store.store_manifest(&manifest).unwrap();

        let server = CheckpointServer::new(store.clone_with_dir(), "0.0.0.0:0".to_string());
        (store, server.router(), cid, manifest)
    }

    #[tokio::test]
    async fn server_get_manifest_returns_200() {
        let (_, router, cid, _) = setup_server().await;
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/v1/checkpoints/{}/manifest", cid.0))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn server_get_manifest_unknown_cid_returns_404() {
        let (_, router, _, _) = setup_server().await;
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/checkpoints/nonexistent/manifest")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn server_get_chunk_returns_200() {
        let (_, router, cid, _) = setup_server().await;
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/v1/checkpoints/{}/chunks/0", cid.0))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn server_get_chunk_unknown_returns_404() {
        let (_, router, cid, _) = setup_server().await;
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/v1/checkpoints/{}/chunks/9999", cid.0))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}

//! Durable reservation and atomic local-ledger settlement for actual provider calls.
use std::collections::BTreeMap;
use std::sync::Arc;

use futures::StreamExt;
use neunode_inference::openai::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageRole,
};
use neunode_inference::provider::InferenceProvider;
use neunode_storage::{
    cf, codec,
    db::NeunodeDb,
    error::StorageError,
    token_store::{TokenBalance, TokenStore, TOKEN_COMPUTE},
};
use serde::{Deserialize, Serialize};

use crate::api::error::ApiError;
use crate::api::inference_api::{InferenceRequest, InferenceResponse};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Receipt {
    pub requester: String,
    pub provider: String,
    pub gross_cost: String,
    pub protocol_fee: String,
    pub net_payout: String,
    pub response_hash: String,
    pub ledger: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct Operation {
    id: String,
    requester: String,
    provider: String,
    fingerprint: String,
    reserved: u128,
    status: String,
    result: Option<InferenceResponse>,
}

pub(crate) fn price(
    input: u32,
    output: u32,
    model: &neunode_inference::provider::ModelInfo,
) -> Result<u128, ApiError> {
    neunode_inference::settlement::SettlementEngine::calculate_cost(
        input,
        output,
        model.input_price_per_million,
        model.output_price_per_million,
    )
    .map(|amount| amount.0)
    .map_err(|error| ApiError::BadRequest(error.to_string()))
}

fn operation_key(id: &str) -> Vec<u8> {
    format!("inference:{id}").into_bytes()
}

/// Update all involved accounts and the operation record in one ledger batch.
fn commit(
    db: &NeunodeDb,
    operation: &Operation,
    deltas: &[(String, u128, u128)],
) -> Result<(), StorageError> {
    let mut accounts = BTreeMap::<String, TokenBalance>::new();
    let store = TokenStore::new(db);
    for (did, debit, credit) in deltas {
        if !accounts.contains_key(did) {
            accounts.insert(did.clone(), store.get_balance(did, TOKEN_COMPUTE)?);
        }
        let balance = accounts.get_mut(did).expect("account inserted");
        balance.balance =
            balance.balance.checked_sub(*debit).ok_or(StorageError::InsufficientBalance {
                required: *debit,
                available: balance.balance,
            })?;
        balance.balance = balance
            .balance
            .checked_add(*credit)
            .ok_or_else(|| StorageError::Serialization("inference balance overflow".into()))?;
    }
    let mut writes = Vec::new();
    for (did, balance) in accounts {
        writes.push((
            cf::CF_TOKENS,
            cf::token_key(&cf::did_hash_16(&did), TOKEN_COMPUTE).to_vec(),
            codec::serialize(&balance)
                .map_err(|error| StorageError::Serialization(error.to_string()))?,
        ));
    }
    writes.push((
        cf::CF_MODELS,
        operation_key(&operation.id),
        serde_json::to_vec(operation)
            .map_err(|error| StorageError::Serialization(error.to_string()))?,
    ));
    let borrowed: Vec<_> =
        writes.iter().map(|(cf, key, value)| (*cf, key.as_slice(), value.as_slice())).collect();
    db.batch_put_raw(&borrowed)
}

pub async fn execute(
    db: Arc<NeunodeDb>,
    requester: String,
    provider: InferenceProvider,
    body: InferenceRequest,
    mut response: InferenceResponse,
) -> Result<InferenceResponse, ApiError> {
    static SLOTS: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> = std::sync::OnceLock::new();
    let _slot = SLOTS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(16)))
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Unavailable("inference capacity exhausted; retry later".into()))?;
    let fingerprint = hex::encode(neunode_crypto::hash::sha256(
        &serde_json::to_vec(&body).map_err(|error| ApiError::Internal(error.to_string()))?,
    ));
    if body.idempotency_key.as_ref().is_some_and(|key| key.is_empty() || key.len() > 128) {
        return Err(ApiError::BadRequest("idempotency_key must have 1 to 128 bytes".into()));
    }
    let nonce = body.idempotency_key.clone().unwrap_or_else(crate::keystore::random_id);
    let id = hex::encode(neunode_crypto::hash::sha256(
        &serde_json::to_vec(&(requester.clone(), nonce))
            .map_err(|error| ApiError::Internal(error.to_string()))?,
    ));
    let model = provider
        .find_model(&body.model)
        .ok_or_else(|| ApiError::NotFound("provider model unavailable".into()))?;
    let request = ChatCompletionRequest {
        model: body.model.clone(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: body.prompt.clone(),
            name: None,
        }],
        temperature: Some(body.temperature),
        max_tokens: Some(body.max_tokens),
        top_p: None,
        stream: Some(false),
        stop: None,
        frequency_penalty: None,
        presence_penalty: None,
    };
    let input_bound =
        neunode_inference::settlement::SettlementEngine::estimate_input_tokens(&request)
            .checked_mul(2)
            .ok_or_else(|| ApiError::BadRequest("input tokens exceed bounds".into()))?;
    let reserved = price(input_bound, body.max_tokens, model)?;
    let mut operation = Operation {
        id: id.clone(),
        requester: requester.clone(),
        provider: provider.did.0.clone(),
        fingerprint: fingerprint.clone(),
        reserved,
        status: "pending".into(),
        result: None,
    };
    let cached = db
        .with_ledger_write(|| {
            if let Some(bytes) = db.get_raw(cf::CF_MODELS, &operation_key(&id))? {
                let previous: Operation = serde_json::from_slice(&bytes)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                return Ok(Some(previous));
            }
            neunode_storage::breaker_store::ensure_closed(&db, "token_volume")?;
            commit(&db, &operation, &[(requester.clone(), reserved, 0)])?;
            Ok::<Option<Operation>, StorageError>(None)
        })
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    if let Some(previous) = cached {
        if previous.fingerprint != fingerprint {
            return Err(ApiError::BadRequest(
                "idempotency key already used for different input".into(),
            ));
        }
        return previous.result.ok_or_else(|| ApiError::Unavailable(format!("request {} is {}; retry with the same key or use a new key after a terminal failure", id, previous.status)));
    }
    let result: Result<ChatCompletionResponse, ApiError> = async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let url = format!(
            "{}/v1/chat/completions",
            provider.endpoint.trim_end_matches('/').trim_end_matches("/v1")
        );
        let upstream =
            client.post(url).header("Idempotency-Key", &id).json(&request).send().await.map_err(
                |error| ApiError::Unavailable(format!("provider request failed: {error}")),
            )?;
        if !upstream.status().is_success() {
            return Err(ApiError::Unavailable(format!(
                "provider returned HTTP {}",
                upstream.status()
            )));
        }
        let mut stream = upstream.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| ApiError::Unavailable(error.to_string()))?;
            if bytes.len().saturating_add(chunk.len()) > 1024 * 1024 {
                return Err(ApiError::Unavailable("provider response exceeds 1 MiB".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        let completion: ChatCompletionResponse =
            serde_json::from_slice(&bytes).map_err(|error| {
                ApiError::Unavailable(format!("invalid provider response: {error}"))
            })?;
        if completion.model != body.model
            || completion.object != "chat.completion"
            || completion.choices.is_empty()
            || completion.usage.prompt_tokens.checked_add(completion.usage.completion_tokens)
                != Some(completion.usage.total_tokens)
            || completion.usage.completion_tokens > body.max_tokens
        {
            return Err(ApiError::Unavailable(
                "provider response has inconsistent model, choices or usage".into(),
            ));
        }
        neunode_inference::settlement::SettlementEngine::validate_token_counts(
            &request,
            completion.usage.prompt_tokens,
            completion.usage.completion_tokens,
        )
        .map_err(|error| ApiError::Unavailable(error.to_string()))?;
        Ok(completion)
    }
    .await;
    let completion = match result {
        Ok(completion) => completion,
        Err(error) => {
            operation.status = "failed_refunded".into();
            db.with_ledger_write(|| commit(&db, &operation, &[(requester, 0, reserved)]))?;
            return Err(error);
        }
    };
    let gross = price(completion.usage.prompt_tokens, completion.usage.completion_tokens, model)?;
    let fee = gross / 50 + u128::from(gross % 50 != 0);
    let net =
        gross.checked_sub(fee).ok_or_else(|| ApiError::Internal("invalid protocol fee".into()))?;
    let refund = reserved
        .checked_sub(gross)
        .ok_or_else(|| ApiError::Internal("usage exceeds reservation".into()))?;
    response.request_id = id;
    response.status = "completed".into();
    response.settlement = Some(Receipt {
        requester: requester.clone(),
        provider: provider.did.0.clone(),
        gross_cost: gross.to_string(),
        protocol_fee: fee.to_string(),
        net_payout: net.to_string(),
        response_hash: hex::encode(neunode_crypto::hash::sha256(
            &serde_json::to_vec(&completion)
                .map_err(|error| ApiError::Internal(error.to_string()))?,
        )),
        ledger: "local".into(),
    });
    response.completion = Some(completion);
    operation.status = "completed".into();
    operation.result = Some(response.clone());
    let settled = db.with_ledger_write(|| {
        commit(
            &db,
            &operation,
            &[
                (requester.clone(), 0, refund),
                (provider.did.0, 0, net),
                ("did:neunode:treasury".into(), 0, fee),
            ],
        )
    });
    if let Err(error) = settled {
        operation.status = "failed_refunded".into();
        operation.result = None;
        db.with_ledger_write(|| commit(&db, &operation, &[(requester, 0, reserved)]))?;
        return Err(error.into());
    }
    Ok(response)
}

/// A crash leaves reservations durable. Refund ambiguous calls once; never replay them blindly.
pub fn recover(db: &NeunodeDb) -> Result<(), ApiError> {
    for (_, bytes) in db.prefix_scan(cf::CF_MODELS, b"inference:")? {
        let operation: Operation = serde_json::from_slice(&bytes)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        if operation.status != "pending" {
            continue;
        }
        db.with_ledger_write(|| {
            let mut current: Operation = serde_json::from_slice(
                &db.get_raw(cf::CF_MODELS, &operation_key(&operation.id))?
                    .ok_or_else(|| StorageError::Serialization("missing reservation".into()))?,
            )
            .map_err(|error| StorageError::Serialization(error.to_string()))?;
            if current.status != "pending" {
                return Ok(());
            }
            current.status = "interrupted_refunded".into();
            commit(db, &current, &[(current.requester.clone(), 0, current.reserved)])
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_reservations_refund_once_and_preserve_terminal_records() {
        let state = crate::testutil::test_state();
        let operation = Operation {
            id: "crash".into(),
            requester: "did:requester".into(),
            provider: "did:provider".into(),
            fingerprint: "input".into(),
            reserved: 40,
            status: "pending".into(),
            result: None,
        };
        TokenStore::new(&state.db)
            .set_balance(
                &operation.requester,
                TOKEN_COMPUTE,
                &TokenBalance { balance: 100, ..Default::default() },
            )
            .unwrap();
        state
            .db
            .with_ledger_write(|| {
                commit(&state.db, &operation, &[(operation.requester.clone(), 40, 0)])
            })
            .unwrap();
        assert_eq!(
            TokenStore::new(&state.db)
                .get_balance(&operation.requester, TOKEN_COMPUTE)
                .unwrap()
                .balance,
            60
        );
        recover(&state.db).unwrap();
        recover(&state.db).unwrap();
        assert_eq!(
            TokenStore::new(&state.db)
                .get_balance(&operation.requester, TOKEN_COMPUTE)
                .unwrap()
                .balance,
            100
        );
        let record: Operation = serde_json::from_slice(
            &state.db.get_raw(cf::CF_MODELS, &operation_key("crash")).unwrap().unwrap(),
        )
        .unwrap();
        assert_eq!(record.status, "interrupted_refunded");
    }
}

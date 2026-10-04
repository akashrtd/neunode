use anyhow::Result;
use neunode_feed::event::FeedEvent;
use neunode_storage::feed_store::StoredEvent;

#[cfg(test)]
pub fn serialize_feed_event(event: &FeedEvent) -> Result<Vec<u8>> {
    serde_json::to_vec(event).map_err(|e| anyhow::anyhow!("feed event serialization failed: {e}"))
}

#[cfg(test)]
pub fn deserialize_feed_event(data: &[u8]) -> Result<FeedEvent> {
    serde_json::from_slice(data)
        .map_err(|e| anyhow::anyhow!("feed event deserialization failed: {e}"))
}

/// Convert a FeedEvent to StoredEvent for persistence.
/// The caller MUST validate the event (including signature) before calling this.
pub(crate) fn feed_event_to_stored(event: &FeedEvent) -> StoredEvent {
    StoredEvent {
        kind: event.kind.as_u16(),
        timestamp: event.timestamp,
        agent_did: event.author.0.clone(),
        sequence: event.sequence,
        prev_hash: event.prev_hash.0.as_bytes().to_vec(),
        payload: [
            b"NNFEED002\n".as_slice(),
            &serde_json::to_vec(event).expect("FeedEvent serializes"),
        ]
        .concat(),
        signature: event.signature.as_ref().map(|s| s.0.as_bytes().to_vec()).unwrap_or_default(),
    }
}

/// Preserve the exact signed body. Legacy rows can be read, but remain unverified.
pub fn stored_to_event(stored: &StoredEvent) -> Result<FeedEvent> {
    if let Some(json) = stored.payload.strip_prefix(b"NNFEED002\n") {
        return Ok(serde_json::from_slice(json)?);
    }
    let mut event = FeedEvent::new(
        neunode_core::kind::Kind::from_u16(stored.kind)?,
        neunode_core::types::Did(stored.agent_did.clone()),
        stored.sequence,
        neunode_core::types::Hash256(String::from_utf8(stored.prev_hash.clone())?),
        String::from_utf8(stored.payload.clone())?,
    )?;
    event.timestamp = stored.timestamp;
    event.signature = if stored.signature.is_empty() {
        None
    } else {
        Some(neunode_core::types::Signature(String::from_utf8(stored.signature.clone())?))
    };
    event.id = event.compute_id()?;
    Ok(event)
}

pub fn create_event(
    db: &neunode_storage::db::NeunodeDb,
    keyring: &neunode_identity::keyring::Keyring,
    kind: u32,
    content: String,
    tags: &[String],
) -> Result<FeedEvent> {
    let kind = neunode_core::kind::Kind::from_u16(u16::try_from(kind)?)?;
    db.with_ledger_write(|| {
        let result = (|| -> Result<FeedEvent> {
            let did = keyring.to_did();
            let store = neunode_storage::feed_store::FeedStore::new(db);
            let latest = store.latest_sequence(&did.0)?;
            let previous = if latest == 0 {
                neunode_core::types::Hash256("0".into())
            } else {
                stored_to_event(
                    &store
                        .get(&did.0, latest)?
                        .ok_or_else(|| anyhow::anyhow!("missing feed head"))?,
                )?
                .compute_hash()?
            };
            let mut event = FeedEvent::new(
                kind,
                did,
                latest.checked_add(1).ok_or_else(|| anyhow::anyhow!("sequence exhausted"))?,
                previous,
                content,
            )?;
            event.tags = tags
                .iter()
                .map(|tag| {
                    let (key, value) = tag.split_once('=').unwrap_or((tag, ""));
                    neunode_feed::event::EventTag { key: key.into(), value: value.into() }
                })
                .collect();
            event.validate()?;
            event.id = event.compute_id()?;
            let (ed, _) = keyring.to_bytes();
            event.sign(&ed.try_into().map_err(|_| anyhow::anyhow!("invalid Ed25519 key"))?)?;
            persist_event(db, &event, &serialize_authenticated_event(&event, keyring)?)?;
            Ok(event)
        })();
        result
            .map_err(|error| neunode_storage::error::StorageError::Serialization(error.to_string()))
    })
    .map_err(Into::into)
}

fn persist_event(
    db: &neunode_storage::db::NeunodeDb,
    event: &FeedEvent,
    wire: &[u8],
) -> Result<()> {
    let stored = feed_event_to_stored(event);
    let key = neunode_storage::cf::feed_event_key(
        &neunode_storage::cf::did_hash_16(&event.author.0),
        event.sequence,
    );
    let value = neunode_storage::codec::serialize(&stored)?;
    db.batch_put_raw(&[
        (neunode_storage::cf::CF_FEED_EVENTS, &key, &value),
        (neunode_storage::cf::CF_FEED_INDEX, event.id.0.as_bytes(), wire),
        (neunode_storage::cf::CF_FEED_STATE, event.author.0.as_bytes(), wire),
    ])?;
    Ok(())
}

pub fn authenticated_position(bytes: &[u8]) -> Result<(String, u64)> {
    let envelope: AuthenticatedEvent = serde_json::from_slice(bytes)?;
    Ok((envelope.event.author.0, envelope.event.sequence))
}

pub fn relay_head(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    value["relay_nonce"] = serde_json::Value::String(crate::keystore::random_id());
    Ok(serde_json::to_vec(&value)?)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AuthenticatedEvent {
    event: FeedEvent,
    identity: neunode_identity::keyring::PublicKeyBundle,
    binding_signature: String,
}

fn identity_binding(identity: &neunode_identity::keyring::PublicKeyBundle) -> Result<Vec<u8>> {
    Ok([b"NNFEED_KEY_BINDING_V1".as_slice(), &serde_json::to_vec(identity)?].concat())
}

pub fn serialize_authenticated_event(
    event: &FeedEvent,
    keyring: &neunode_identity::keyring::Keyring,
) -> Result<Vec<u8>> {
    let identity = keyring.export_public();
    let signature = keyring.sign_secp256k1(&identity_binding(&identity)?);
    Ok(serde_json::to_vec(&AuthenticatedEvent {
        event: event.clone(),
        identity,
        binding_signature: hex::encode(signature.to_bytes()),
    })?)
}

pub fn verify_authenticated_event(bytes: &[u8]) -> Result<FeedEvent> {
    let envelope: AuthenticatedEvent = serde_json::from_slice(bytes)?;
    let event = envelope.event;
    let identity = envelope.identity;
    let secp = k256::ecdsa::VerifyingKey::from_sec1_bytes(&identity.secp256k1)?;
    let did = format!(
        "did:neunode:0x{}",
        hex::encode(neunode_crypto::secp256k1::verifying_key_to_address(&secp))
    );
    anyhow::ensure!(
        identity.did.0 == did && event.author == identity.did,
        "event author is not bound to the identity keys"
    );
    let signature = k256::ecdsa::Signature::from_slice(&hex::decode(envelope.binding_signature)?)?;
    anyhow::ensure!(
        neunode_crypto::secp256k1::verify_signature(
            &secp,
            &identity_binding(&identity)?,
            &signature
        ),
        "invalid identity key binding"
    );
    let ed: [u8; 32] =
        identity.ed25519.try_into().map_err(|_| anyhow::anyhow!("invalid Ed25519 key"))?;
    event.validate()?;
    anyhow::ensure!(
        event.compute_id()? == event.id && event.verify_signature(&ed),
        "invalid event ID or signature"
    );
    Ok(event)
}

pub fn ingest_authenticated_event(
    db: &neunode_storage::db::NeunodeDb,
    bytes: &[u8],
) -> Result<Option<FeedEvent>> {
    let event = verify_authenticated_event(bytes)?;
    let stored = feed_event_to_stored(&event);
    db.with_ledger_write(|| {
        let store = neunode_storage::feed_store::FeedStore::new(db);
        if let Some(existing) = store.get(&event.author.0, event.sequence)? {
            if existing == stored {
                return Ok(false);
            }
            return Err(neunode_storage::error::StorageError::Serialization(
                "conflicting feed event".into(),
            ));
        }
        let latest = store.latest_sequence(&event.author.0)?;
        let previous = if latest == 0 {
            neunode_core::types::Hash256("0".into())
        } else {
            let row = store.get(&event.author.0, latest)?.ok_or_else(|| {
                neunode_storage::error::StorageError::Serialization("missing feed head".into())
            })?;
            stored_to_event(&row).and_then(|event| Ok(event.compute_hash()?)).map_err(|error| {
                neunode_storage::error::StorageError::Serialization(error.to_string())
            })?
        };
        if latest.checked_add(1) != Some(event.sequence) || event.prev_hash != previous {
            return Err(neunode_storage::error::StorageError::Serialization(
                "feed sequence gap or invalid previous hash".into(),
            ));
        }
        persist_event(db, &event, bytes).map_err(|error| {
            neunode_storage::error::StorageError::Serialization(error.to_string())
        })?;
        Ok(true)
    })
    .map(|inserted| inserted.then_some(event))
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use neunode_core::kind::Kind;
    use neunode_core::types::{Did, Hash256};

    fn test_event() -> FeedEvent {
        FeedEvent::new(
            Kind::BountyPost,
            Did("did:neunode:test_agent_001".to_string()),
            0,
            Hash256("0".to_string()),
            "test content for wire".to_string(),
        )
        .expect("event creation should succeed")
    }

    #[test]
    fn roundtrip_serialization() {
        let event = test_event();
        let bytes = serialize_feed_event(&event).unwrap();
        let back = deserialize_feed_event(&bytes).unwrap();
        assert_eq!(event.id, back.id);
        assert_eq!(event.kind, back.kind);
        assert_eq!(event.author, back.author);
        assert_eq!(event.content, back.content);
        assert_eq!(event.sequence, back.sequence);
    }

    #[test]
    fn deserialize_invalid_json_fails() {
        assert!(deserialize_feed_event(b"not json").is_err());
    }

    #[test]
    fn deserialize_empty_fails() {
        assert!(deserialize_feed_event(b"").is_err());
    }

    #[test]
    fn to_stored_preserves_fields() {
        let event = test_event();
        let stored = feed_event_to_stored(&event);
        assert_eq!(stored.kind, event.kind.as_u16());
        assert_eq!(stored.agent_did, event.author.0);
        assert_eq!(stored.sequence, event.sequence);
    }

    #[test]
    fn rejects_tampering_impersonation_forks_and_sequence_gaps() {
        let app = crate::testutil::test_state();
        let author = neunode_identity::keyring::Keyring::generate();
        let first =
            create_event(&app.db, &author, 0, "first".into(), &["evidence=yes".into()]).unwrap();
        assert_eq!(stored_to_event(&feed_event_to_stored(&first)).unwrap(), first);
        assert_eq!(first.compute_id().unwrap(), first.id);
        assert!(first.verify_signature(&author.ed25519_public_key().to_bytes()));
        let receiver = crate::testutil::test_state();
        let bytes = serialize_authenticated_event(&first, &author).unwrap();
        assert!(ingest_authenticated_event(&receiver.db, &bytes).unwrap().is_some());
        assert!(ingest_authenticated_event(&receiver.db, &bytes).unwrap().is_none());
        let mut envelope: AuthenticatedEvent = serde_json::from_slice(&bytes).unwrap();
        envelope.event.content = "tampered".into();
        assert!(ingest_authenticated_event(&receiver.db, &serde_json::to_vec(&envelope).unwrap())
            .is_err());
        envelope.event = first.clone();
        envelope.identity.did = neunode_core::types::Did("did:neunode:someone-else".into());
        assert!(ingest_authenticated_event(&receiver.db, &serde_json::to_vec(&envelope).unwrap())
            .is_err());
        let second = create_event(&app.db, &author, 1, "second".into(), &[]).unwrap();
        let third = create_event(&app.db, &author, 0, "third".into(), &[]).unwrap();
        assert!(ingest_authenticated_event(
            &receiver.db,
            &serialize_authenticated_event(&third, &author).unwrap()
        )
        .is_err());
        assert!(ingest_authenticated_event(
            &receiver.db,
            &serialize_authenticated_event(&second, &author).unwrap()
        )
        .unwrap()
        .is_some());
        let mut fork = second.clone();
        fork.content = "signed fork".into();
        fork.id = fork.compute_id().unwrap();
        let (ed, _) = author.to_bytes();
        fork.sign(&ed.try_into().unwrap()).unwrap();
        assert!(ingest_authenticated_event(
            &receiver.db,
            &serialize_authenticated_event(&fork, &author).unwrap()
        )
        .is_err());
        let stored = neunode_storage::feed_store::FeedStore::new(&receiver.db)
            .get(&second.author.0, second.sequence)
            .unwrap()
            .unwrap();
        assert_eq!(stored_to_event(&stored).unwrap(), second);
    }
}

//! Integration tests for the identity creation flow.
//!
//! Verifies end-to-end: keyring generation → DID derivation → agent card
//! signing → DID document construction → peer ID derivation → export/reimport.

use std::collections::HashMap;

use neunode_core::types::{AgentLifecycle, Did};
use neunode_identity::agent_card::{AgentCard, AgentCardBuilder};
use neunode_identity::did::did_to_peer_id;
use neunode_identity::document::DidDocument;
use neunode_identity::keyring::Keyring;

// ---------------------------------------------------------------------------
// Test 1: Full identity creation — keyring → DID → agent card → DID document
// ---------------------------------------------------------------------------

#[test]
fn full_identity_creation_flow() {
    // Generate keyring with dual keys
    let keyring = Keyring::generate();

    // Derive both DID types
    let did_neunode = keyring.to_did().unwrap();
    let did_key = keyring.to_did_key();

    // Verify DID formats
    assert!(did_neunode.is_neunode(), "should be did:neunode");
    assert!(did_key.is_key(), "should be did:key");
    assert!(did_neunode.as_str().starts_with("did:neunode:0x"));
    assert!(did_key.as_str().starts_with("did:key:z6Mk"));

    // Create and sign agent card
    let card =
        AgentCard::new("integration-agent", &keyring, vec!["inference".into()], HashMap::new())
            .unwrap();
    let signed = card.sign(&keyring);
    assert!(signed.verify(), "signed card should verify");

    // Build DID document
    let doc = DidDocument::from_keyring(&keyring).unwrap();
    assert_eq!(doc.verification_method.len(), 2, "should have Ed25519 + secp256k1 VMs");
    assert_eq!(doc.did(), did_neunode);
}

// ---------------------------------------------------------------------------
// Test 2: Dual-key keyring produces both Ed25519 and secp256k1 keys
// ---------------------------------------------------------------------------

#[test]
fn keyring_generates_dual_keys() {
    let keyring = Keyring::generate();

    // Ed25519 public key is 32 bytes
    let ed_pub = keyring.ed25519_public_key();
    assert_eq!(ed_pub.to_bytes().len(), 32, "Ed25519 pubkey should be 32 bytes");

    // secp256k1 public key is 65 bytes (uncompressed SEC1)
    let secp_pub = keyring.secp256k1_public_key().unwrap();
    assert_eq!(secp_pub.len(), 65, "secp256k1 pubkey should be 65 bytes");
    assert_eq!(secp_pub[0], 0x04, "uncompressed SEC1 prefix");
}

// ---------------------------------------------------------------------------
// Test 3: DID is correctly derived from keyring's Ethereum address
// ---------------------------------------------------------------------------

#[test]
fn did_derived_from_ethereum_address() {
    let keyring = Keyring::generate();
    let eth_addr = keyring.ethereum_address().unwrap();
    let did = keyring.to_did().unwrap();

    // Ethereum address format: 0x + 40 hex chars
    assert!(eth_addr.starts_with("0x"), "should be 0x-prefixed");
    assert_eq!(eth_addr.len(), 42, "should be 42 chars");

    // DID contains the Ethereum address
    assert!(did.as_str().contains(&eth_addr), "did:neunode should contain the Ethereum address");
}

// ---------------------------------------------------------------------------
// Test 4: Agent card sign + verify roundtrip, tamper detection
// ---------------------------------------------------------------------------

#[test]
fn agent_card_sign_verify_and_tamper_detection() {
    let keyring = Keyring::generate();
    let card =
        AgentCard::new("tamper-test", &keyring, vec!["training".into()], HashMap::new()).unwrap();

    // Sign with correct key
    let signed = card.sign(&keyring);
    assert!(signed.verify(), "correctly signed card should verify");

    // Tamper with name — verification should fail
    let mut tampered = signed;
    tampered.card.name = "tampered-name".to_string();
    assert!(!tampered.verify(), "tampered card should fail verification");
}

// ---------------------------------------------------------------------------
// Test 5: DID document contains correct verification methods and references
// ---------------------------------------------------------------------------

#[test]
fn did_document_verification_methods() {
    let keyring = Keyring::generate();
    let doc = DidDocument::from_keyring(&keyring).unwrap();
    let did_str = keyring.to_did().unwrap().as_str().to_string();

    // Check verification method IDs reference the DID
    let ed_vm_id = format!("{did_str}#keys-1");
    let secp_vm_id = format!("{did_str}#keys-2");

    let ed_vm = doc.verify_method(&ed_vm_id).expect("Ed25519 VM should exist");
    assert_eq!(ed_vm.vm_type, "Ed25519VerificationKey2020");
    assert!(ed_vm.public_key_multibase.starts_with('z'));

    let secp_vm = doc.verify_method(&secp_vm_id).expect("secp256k1 VM should exist");
    assert_eq!(secp_vm.vm_type, "EcdsaSecp256k1VerificationKey2019");

    // Authentication references Ed25519, assertion references secp256k1
    assert_eq!(doc.authentication, vec![ed_vm_id]);
    assert_eq!(doc.assertion_method, vec![secp_vm_id]);
}

// ---------------------------------------------------------------------------
// Test 6: Peer ID derived from did:key, fails for did:neunode
// ---------------------------------------------------------------------------

#[test]
fn peer_id_from_did_key_roundtrip() {
    let keyring = Keyring::generate();
    let did_key = keyring.to_did_key();

    // Derive peer ID from did:key
    let peer_id = did_to_peer_id(&did_key).expect("should derive peer ID from did:key");
    assert!(peer_id.as_str().starts_with("12D3Koo"), "libp2p PeerId should start with 12D3Koo");
    assert!(peer_id.as_str().len() > 40);

    // did:neunode cannot derive peer ID directly
    let did_neunode = keyring.to_did().unwrap();
    assert!(did_to_peer_id(&did_neunode).is_err(), "did:neunode should fail peer ID derivation");
}

// ---------------------------------------------------------------------------
// Test 7: Ethereum address computation matches expected format
// ---------------------------------------------------------------------------

#[test]
fn ethereum_address_computation_consistent() {
    let keyring = Keyring::generate();

    // Address should be consistent across calls
    let addr1 = keyring.ethereum_address().unwrap();
    let addr2 = keyring.ethereum_address().unwrap();
    assert_eq!(addr1, addr2, "address should be deterministic for same keyring");

    // Should be valid hex after 0x prefix
    assert!(addr1[2..].chars().all(|c: char| c.is_ascii_hexdigit()));
}

// ---------------------------------------------------------------------------
// Test 8: Export + reimport preserves identity
// ---------------------------------------------------------------------------

#[test]
fn export_reimport_preserves_identity() {
    let keyring = Keyring::generate();
    let original_did = keyring.to_did().unwrap();
    let original_did_key = keyring.to_did_key();

    // Export bytes
    let (ed_bytes, secp_bytes) = keyring.to_bytes();

    // Reimport
    let ed_arr: [u8; 32] = ed_bytes.try_into().expect("32 bytes");
    let secp_arr: [u8; 32] = secp_bytes.try_into().expect("32 bytes");
    let restored = Keyring::from_bytes(&ed_arr, &secp_arr).expect("should reconstruct keyring");

    // Verify identity preserved
    assert_eq!(restored.to_did().unwrap(), original_did, "DID should be preserved after reimport");
    assert_eq!(
        restored.to_did_key(),
        original_did_key,
        "did:key should be preserved after reimport"
    );
    assert_eq!(
        restored.ethereum_address().unwrap(),
        keyring.ethereum_address().unwrap(),
        "Ethereum address should match"
    );
}

// ---------------------------------------------------------------------------
// Test 9: Multiple keyrings produce unique DIDs
// ---------------------------------------------------------------------------

#[test]
fn multiple_keyrings_unique_dids() {
    let kr1 = Keyring::generate();
    let kr2 = Keyring::generate();
    let kr3 = Keyring::generate();

    let dids: Vec<Did> = vec![kr1.to_did().unwrap(), kr2.to_did().unwrap(), kr3.to_did().unwrap()];
    let did_keys: Vec<Did> = vec![kr1.to_did_key(), kr2.to_did_key(), kr3.to_did_key()];

    // All DIDs should be unique
    assert_ne!(dids[0], dids[1], "did:neunode should be unique per keyring");
    assert_ne!(dids[1], dids[2]);
    assert_ne!(dids[0], dids[2]);

    // All did:key should be unique
    assert_ne!(did_keys[0], did_keys[1]);
    assert_ne!(did_keys[1], did_keys[2]);

    // Ethereum addresses should be unique
    assert_ne!(kr1.ethereum_address().unwrap(), kr2.ethereum_address().unwrap());
}

// ---------------------------------------------------------------------------
// Test 10: Agent card builder + CID + serde roundtrip
// ---------------------------------------------------------------------------

#[test]
fn agent_card_builder_cid_and_serde_roundtrip() {
    let keyring = Keyring::generate();

    // Use builder pattern
    let card = AgentCardBuilder::new("builder-integration-test")
        .capability("inference")
        .capability("training")
        .metadata("gpu", "H100")
        .metadata("framework", "pytorch")
        .build(&keyring)
        .expect("builder should succeed with valid keyring");

    assert_eq!(card.name, "builder-integration-test");
    assert_eq!(card.capabilities, vec!["inference", "training"]);
    assert_eq!(card.metadata.get("gpu").expect("gpu key"), "H100");
    assert_eq!(card.lifecycle, AgentLifecycle::Created);

    // CID is deterministic
    let cid1 = card.to_cid();
    let cid2 = card.to_cid();
    assert_eq!(cid1, cid2, "CID should be deterministic");

    // Serde roundtrip
    let json = serde_json::to_string(&card).expect("serialize card");
    let back: AgentCard = serde_json::from_str(&json).expect("deserialize card");
    assert_eq!(card.did, back.did);
    assert_eq!(card.capabilities, back.capabilities);
    assert_eq!(card.metadata, back.metadata);
}

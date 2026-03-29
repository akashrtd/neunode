use neunode_core::{Did, NeunodeError, Result};
use serde::{Deserialize, Serialize};

use crate::keyring::Keyring;

const ED25519_VM_TYPE: &str = "Ed25519VerificationKey2020";
const SECP256K1_VM_TYPE: &str = "EcdsaSecp256k1VerificationKey2019";

/// W3C DID Core verification method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub vm_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: String,
}

/// W3C DID Core service endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceEndpoint {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// W3C DID Core Document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DidDocument {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    #[serde(rename = "assertionMethod")]
    pub assertion_method: Vec<String>,
    #[serde(rename = "keyAgreement")]
    pub key_agreement: Vec<String>,
    pub service: Vec<ServiceEndpoint>,
}

impl DidDocument {
    /// Build a DID Document from a dual-key keyring.
    pub fn from_keyring(keyring: &Keyring) -> Self {
        let did = keyring.to_did();
        let did_str = did.as_str().to_string();

        let ed25519_pub = keyring.ed25519_public_key().to_bytes();
        let ed25519_multibase = format!("z{}", multibase_base58btc_ed25519(&ed25519_pub));

        let secp_pub = keyring.secp256k1_public_key();
        let secp_multibase = format!("z{}", multibase_base58btc_secp256k1(&secp_pub));

        let ed_vm_id = format!("{did_str}#keys-1");
        let secp_vm_id = format!("{did_str}#keys-2");

        let ed_vm = VerificationMethod {
            id: ed_vm_id.clone(),
            vm_type: ED25519_VM_TYPE.to_string(),
            controller: did_str.clone(),
            public_key_multibase: ed25519_multibase,
        };

        let secp_vm = VerificationMethod {
            id: secp_vm_id.clone(),
            vm_type: SECP256K1_VM_TYPE.to_string(),
            controller: did_str.clone(),
            public_key_multibase: secp_multibase,
        };

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
                "https://w3id.org/security/suites/secp256k1-2019/v1".to_string(),
            ],
            id: did_str,
            controller: None,
            verification_method: vec![ed_vm, secp_vm],
            authentication: vec![ed_vm_id],
            assertion_method: vec![secp_vm_id],
            key_agreement: vec![],
            service: vec![],
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| NeunodeError::SerializationError(e.to_string()))
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| NeunodeError::SerializationError(e.to_string()))
    }

    /// Look up a verification method by its ID.
    pub fn verify_method(&self, method_id: &str) -> Option<&VerificationMethod> {
        self.verification_method.iter().find(|vm| vm.id == method_id)
    }

    /// The DID subject of this document.
    pub fn did(&self) -> Did {
        Did(self.id.clone())
    }
}

fn multibase_base58btc_ed25519(pubkey: &[u8; 32]) -> String {
    let vk = neunode_crypto::ed25519::verifying_key_from_bytes(pubkey)
        .expect("valid ed25519 public key");
    crate::did::generate_did_key(&vk).as_str().trim_start_matches("did:key:").to_string()
}

fn multibase_base58btc_secp256k1(pubkey_uncompressed: &[u8]) -> String {
    let mut combined = Vec::with_capacity(2 + pubkey_uncompressed.len());
    combined.extend_from_slice(&[0xe7, 0x01]); // secp256k1-pub multicodec
    combined.extend_from_slice(pubkey_uncompressed);
    base58btc_encode_simple(&combined)
}

const B58: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn base58btc_encode_simple(data: &[u8]) -> String {
    let leading_zeros = data.iter().take_while(|&&b| b == 0).count();
    let mut digits: Vec<u8> = Vec::new();
    for &byte in data {
        let mut carry = byte as u32;
        for d in digits.iter_mut() {
            carry += (*d as u32) * 256;
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
        result.push(B58[d as usize] as char);
    }
    if result.is_empty() {
        result.push('1');
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_doc() -> DidDocument {
        let kr = Keyring::generate();
        DidDocument::from_keyring(&kr)
    }

    #[test]
    fn from_keyring_has_correct_context() {
        let doc = make_doc();
        assert!(doc.context.contains(&"https://www.w3.org/ns/did/v1".to_string()));
    }

    #[test]
    fn from_keyring_has_two_verification_methods() {
        let doc = make_doc();
        assert_eq!(doc.verification_method.len(), 2);
        assert_eq!(doc.verification_method[0].vm_type, ED25519_VM_TYPE);
        assert_eq!(doc.verification_method[1].vm_type, SECP256K1_VM_TYPE);
    }

    #[test]
    fn from_keyring_ids_reference_did() {
        let kr = Keyring::generate();
        let doc = DidDocument::from_keyring(&kr);
        let did = kr.to_did();
        let did_str = did.as_str();
        assert_eq!(doc.id, did_str);
        assert!(doc.verification_method[0].id.starts_with(did_str));
        assert!(doc.verification_method[1].id.starts_with(did_str));
        assert!(doc.authentication[0].starts_with(did_str));
        assert!(doc.assertion_method[0].starts_with(did_str));
    }

    #[test]
    fn from_keyring_ed25519_key_multibase() {
        let doc = make_doc();
        let ed_vm = &doc.verification_method[0];
        assert!(ed_vm.public_key_multibase.starts_with('z'));
        assert!(ed_vm.public_key_multibase.len() > 40);
    }

    #[test]
    fn from_keyring_secp256k1_key_multibase() {
        let doc = make_doc();
        let secp_vm = &doc.verification_method[1];
        assert!(secp_vm.public_key_multibase.starts_with('z'));
    }

    #[test]
    fn from_keyring_controller_is_none() {
        let doc = make_doc();
        assert!(doc.controller.is_none());
    }

    #[test]
    fn from_keyring_empty_service_and_key_agreement() {
        let doc = make_doc();
        assert!(doc.service.is_empty());
        assert!(doc.key_agreement.is_empty());
    }

    #[test]
    fn json_roundtrip() {
        let doc = make_doc();
        let json = doc.to_json().expect("to_json");
        let back = DidDocument::from_json(&json).expect("from_json");
        assert_eq!(doc, back);
    }

    #[test]
    fn json_is_valid_structure() {
        let doc = make_doc();
        let json = doc.to_json().expect("to_json");
        let val: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(val["@context"], serde_json::json!(doc.context));
        assert!(val["verificationMethod"].is_array());
        assert!(val["authentication"].is_array());
    }

    #[test]
    fn verify_method_lookup_found() {
        let kr = Keyring::generate();
        let doc = DidDocument::from_keyring(&kr);
        let ed_id = format!("{}#keys-1", kr.to_did().as_str());
        let vm = doc.verify_method(&ed_id);
        assert!(vm.is_some());
        assert_eq!(vm.unwrap().vm_type, ED25519_VM_TYPE);
    }

    #[test]
    fn verify_method_lookup_secp256k1() {
        let kr = Keyring::generate();
        let doc = DidDocument::from_keyring(&kr);
        let secp_id = format!("{}#keys-2", kr.to_did().as_str());
        let vm = doc.verify_method(&secp_id);
        assert!(vm.is_some());
        assert_eq!(vm.unwrap().vm_type, SECP256K1_VM_TYPE);
    }

    #[test]
    fn verify_method_not_found() {
        let doc = make_doc();
        assert!(doc.verify_method("nonexistent").is_none());
    }

    #[test]
    fn did_accessor() {
        let kr = Keyring::generate();
        let doc = DidDocument::from_keyring(&kr);
        assert_eq!(doc.did(), kr.to_did());
    }

    #[test]
    fn authentication_references_ed25519() {
        let kr = Keyring::generate();
        let doc = DidDocument::from_keyring(&kr);
        let ed_id = format!("{}#keys-1", kr.to_did().as_str());
        assert_eq!(doc.authentication, vec![ed_id]);
    }

    #[test]
    fn assertion_method_references_secp256k1() {
        let kr = Keyring::generate();
        let doc = DidDocument::from_keyring(&kr);
        let secp_id = format!("{}#keys-2", kr.to_did().as_str());
        assert_eq!(doc.assertion_method, vec![secp_id]);
    }
}

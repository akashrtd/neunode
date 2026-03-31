// @neunode/sdk — Identity types mirroring neunode-identity (agent_card, document, keyring)

import type { Did, PeerId, AgentLifecycle } from './core.js';

/** Agent's Ed25519 + secp256k1 public key bundle, tied to a DID. */
export interface PublicKeyBundle {
  readonly ed25519: Uint8Array;
  readonly secp256k1: Uint8Array;
  readonly did: Did;
}

/** Verifiable agent profile card with capabilities, lifecycle state, and key bundle. */
export interface AgentCard {
  readonly did: Did;
  readonly name: string;
  readonly version: number;
  readonly capabilities: readonly string[];
  readonly lifecycle: AgentLifecycle;
  readonly peer_id: PeerId;
  readonly public_key_bundle: PublicKeyBundle;
  readonly metadata: Readonly<Record<string, string>>;
  readonly created_at: number;
  readonly updated_at: number;
}

/** An AgentCard with an Ed25519 detached signature and timestamp. */
export interface SignedAgentCard {
  readonly card: AgentCard;
  readonly signature: Uint8Array;
  readonly signed_at: number;
}

export interface VerificationMethod {
  readonly id: string;
  readonly vm_type: string;
  readonly controller: string;
  readonly public_key_multibase: string;
}

export interface ServiceEndpoint {
  readonly id: string;
  readonly service_type: string;
  readonly service_endpoint: string;
}

/** W3C DID Document with verification methods, authentication, and service endpoints. */
export interface DidDocument {
  readonly '@context': string | readonly string[];
  readonly id: Did;
  readonly controller?: string;
  readonly verificationMethod?: readonly VerificationMethod[];
  readonly authentication?: readonly string[];
  readonly assertionMethod?: readonly string[];
  readonly keyAgreement?: readonly string[];
  readonly service?: readonly ServiceEndpoint[];
}

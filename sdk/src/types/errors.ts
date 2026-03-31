// @neunode/sdk — Error types mirroring neunode-core/src/error.rs

export interface NeunodeErrorDetails {
  readonly code: string;
  readonly message: string;
}

export type NeunodeError =
  | { readonly code: 'InvalidDid'; readonly message: string }
  | { readonly code: 'KeyNotFound'; readonly message: string }
  | { readonly code: 'SignatureVerificationFailed'; readonly message: string }
  | { readonly code: 'KeyRotationFailed'; readonly message: string }
  | { readonly code: 'InvalidEvent'; readonly message: string }
  | { readonly code: 'SigchainBroken'; readonly seq: number; readonly message: string }
  | { readonly code: 'InvalidKind'; readonly kind_value: number; readonly message: string }
  | { readonly code: 'SchemaValidationFailed'; readonly message: string }
  | { readonly code: 'StorageError'; readonly message: string }
  | { readonly code: 'NotFound'; readonly message: string }
  | { readonly code: 'AlreadyExists'; readonly message: string }
  | { readonly code: 'InsufficientBalance'; readonly have: bigint; readonly need: bigint; readonly message: string }
  | { readonly code: 'StakingFailed'; readonly message: string }
  | { readonly code: 'UnbondingPeriodNotElapsed'; readonly message: string }
  | { readonly code: 'InvalidStateTransition'; readonly from: string; readonly to: string; readonly message: string }
  | { readonly code: 'TimeoutExpired'; readonly message: string }
  | { readonly code: 'EscrowError'; readonly message: string }
  | { readonly code: 'ConnectionFailed'; readonly message: string }
  | { readonly code: 'PeerNotFound'; readonly message: string }
  | { readonly code: 'GossipsubError'; readonly message: string }
  | { readonly code: 'ConfigError'; readonly message: string }
  | { readonly code: 'InvalidArgument'; readonly message: string }
  | { readonly code: 'CryptoError'; readonly message: string }
  | { readonly code: 'EncodingError'; readonly message: string }
  | { readonly code: 'IoError'; readonly message: string }
  | { readonly code: 'SerializationError'; readonly message: string };

export const ExitCode = {
  Success: 0,
  GeneralError: 1,
  UsageError: 2,
  NetworkError: 10,
  Timeout: 11,
  AuthError: 20,
  InsufficientResources: 30,
  NotFound: 40,
  RateLimit: 50,
  Conflict: 60,
} as const;

export type ExitCode = (typeof ExitCode)[keyof typeof ExitCode];

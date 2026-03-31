// @neunode/sdk — CLI output types mirroring agnetd/src/output.rs

export const OutputFormat = {
  Human: 'Human',
  Json: 'Json',
  JsonCompact: 'JsonCompact',
  Ndjson: 'Ndjson',
} as const;

export type OutputFormat = (typeof OutputFormat)[keyof typeof OutputFormat];

export interface SuccessEnvelope<T> {
  readonly data: T;
  readonly success: true;
}

export interface ErrorEnvelope {
  readonly error: string;
  readonly success: false;
}

export type CliOutput<T> = SuccessEnvelope<T> | ErrorEnvelope;

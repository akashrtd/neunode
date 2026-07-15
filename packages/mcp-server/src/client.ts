/**
 * HTTP client for the agnetd daemon REST API.
 *
 * All requests go through agnetd's `/api/v1/*` endpoints and return data
 * wrapped in the standard JSON envelope: { data, success } | { error, success }.
 */

import type {
  ApiEnvelope,
  BalanceResponse,
  AllBalancesResponse,
  BountyActionResponse,
  BountyListItem,
  BountyResponse,
  CreateIdentityResponse,
  FeedEventResponse,
  IdentityListItem,
  IdentityResponse,
  InferenceRequestResponse,
  MeshStatusResponse,
  ModelEntry,
  ModelResponse,
  PeersResponse,
  PostFeedResponse,
  ProviderEntry,
  StakeResponse,
  StakeStatusResponse,
  TransferResponse,
  UnstakeResponse,
} from "./types.js";

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

export class AgnetdClientError extends Error {
  constructor(
    message: string,
    public readonly code: string,
  ) {
    super(message);
    this.name = "AgnetdClientError";
  }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

export class AgnetdClient {
  private readonly baseUrl: string;
  private readonly timeout: number;

  constructor(baseUrl: string, timeout = 30_000) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.timeout = timeout;
  }

  // -----------------------------------------------------------------------
  // Identity
  // -----------------------------------------------------------------------

  async createIdentity(params: {
    name: string;
    method?: string;
  }): Promise<CreateIdentityResponse> {
    return this.post<CreateIdentityResponse>("/api/v1/identity/create", {
      name: params.name,
      method: params.method ?? "key",
    });
  }

  async listIdentities(): Promise<ReadonlyArray<IdentityListItem>> {
    return this.get<ReadonlyArray<IdentityListItem>>("/api/v1/identity/list");
  }

  async whoami(): Promise<IdentityResponse> {
    return this.get<IdentityResponse>("/api/v1/identity");
  }

  async getIdentity(did: string): Promise<IdentityListItem> {
    const list = await this.listIdentities();
    const match = list.find((item) => item.did === did);
    if (!match) {
      throw new AgnetdClientError(`identity '${did}' not found`, "NOT_FOUND");
    }
    return match;
  }

  // -----------------------------------------------------------------------
  // Feed
  // -----------------------------------------------------------------------

  async postFeed(params: {
    kind: number;
    content: string;
    tags?: ReadonlyArray<string>;
  }): Promise<PostFeedResponse> {
    return this.post<PostFeedResponse>("/api/v1/feed", {
      kind: params.kind,
      content: params.content,
      tags: params.tags,
    });
  }

  async readFeed(params?: {
    kind?: number;
    author?: string;
    limit?: number;
  }): Promise<ReadonlyArray<FeedEventResponse>> {
    const query = buildQuery(params);
    return this.get<ReadonlyArray<FeedEventResponse>>(`/api/v1/feed${query}`);
  }

  // -----------------------------------------------------------------------
  // Inference
  // -----------------------------------------------------------------------

  async listModels(provider?: string): Promise<ReadonlyArray<ModelEntry>> {
    const query = buildQuery({ provider });
    const resp = await this.get<{ models: ReadonlyArray<ModelEntry> }>(
      `/api/v1/inference/models${query}`,
    );
    return resp.models;
  }

  async listProviders(model?: string): Promise<ReadonlyArray<ProviderEntry>> {
    const query = buildQuery({ model });
    const resp = await this.get<{ providers: ReadonlyArray<ProviderEntry> }>(
      `/api/v1/inference/providers${query}`,
    );
    return resp.providers;
  }

  async requestInference(params: {
    model: string;
    prompt: string;
    max_tokens?: number;
    temperature?: number;
  }): Promise<InferenceRequestResponse> {
    return this.post<InferenceRequestResponse>("/api/v1/inference/request", {
      model: params.model,
      prompt: params.prompt,
      max_tokens: params.max_tokens ?? 256,
      temperature: params.temperature ?? 0.7,
    });
  }

  // -----------------------------------------------------------------------
  // Bounty
  // -----------------------------------------------------------------------

  async createBounty(params: {
    title: string;
    description: string;
    reward: number;
    token?: string;
    claim_deadline?: number;
    work_deadline?: number;
  }): Promise<BountyResponse> {
    return this.post<BountyResponse>("/api/v1/bounties", {
      title: params.title,
      description: params.description,
      reward: params.reward,
      token: params.token ?? "compute",
      claim_deadline: params.claim_deadline ?? 72,
      work_deadline: params.work_deadline ?? 168,
    });
  }

  async listBounties(params?: {
    state?: string;
    creator?: string;
    limit?: number;
  }): Promise<ReadonlyArray<BountyListItem>> {
    const query = buildQuery(params);
    return this.get<ReadonlyArray<BountyListItem>>(`/api/v1/bounties${query}`);
  }

  async getBounty(id: string): Promise<BountyResponse> {
    return this.get<BountyResponse>(`/api/v1/bounties/${encodeURIComponent(id)}`);
  }

  async claimBounty(
    id: string,
    stake: number,
  ): Promise<BountyActionResponse> {
    return this.post<BountyActionResponse>(
      `/api/v1/bounties/${encodeURIComponent(id)}/claim`,
      { stake },
    );
  }

  async submitBounty(
    id: string,
    params: { artifact: string; evidence?: string },
  ): Promise<BountyActionResponse> {
    return this.post<BountyActionResponse>(
      `/api/v1/bounties/${encodeURIComponent(id)}/submit`,
      params,
    );
  }

  async reviewBounty(
    id: string,
    params: { score: number; feedback: string },
  ): Promise<BountyActionResponse> {
    return this.post<BountyActionResponse>(
      `/api/v1/bounties/${encodeURIComponent(id)}/review`,
      params,
    );
  }

  // -----------------------------------------------------------------------
  // Token
  // -----------------------------------------------------------------------

  async getBalance(token?: string): Promise<BalanceResponse | AllBalancesResponse> {
    const query = buildQuery(token ? { token } : undefined);
    return this.get<BalanceResponse | AllBalancesResponse>(
      `/api/v1/tokens/balance${query}`,
    );
  }

  async transfer(params: {
    to: string;
    amount: number;
    token?: string;
  }): Promise<TransferResponse> {
    return this.post<TransferResponse>("/api/v1/tokens/transfer", {
      to: params.to,
      amount: params.amount,
      token: params.token ?? "compute",
    });
  }

  async stake(params: {
    amount: number;
    token?: string;
  }): Promise<StakeResponse> {
    return this.post<StakeResponse>("/api/v1/tokens/stake", {
      amount: params.amount,
      token: params.token ?? "compute",
    });
  }

  async unstake(amount: number): Promise<UnstakeResponse> {
    return this.post<UnstakeResponse>("/api/v1/tokens/unstake", { amount });
  }

  async getStakingInfo(): Promise<StakeStatusResponse> {
    return this.get<StakeStatusResponse>("/api/v1/tokens/stake-status");
  }

  // -----------------------------------------------------------------------
  // Model
  // -----------------------------------------------------------------------

  async registerModel(params: {
    name: string;
    path: string;
  }): Promise<{
    status: string;
    model_id: string;
    source: string;
    context_length: number;
    input_price_per_million: number;
    output_price_per_million: number;
  }> {
    return this.post("/api/v1/models", params);
  }

  async listRegisteredModels(provider?: string): Promise<ReadonlyArray<ModelResponse>> {
    const query = buildQuery(provider ? { provider } : undefined);
    return this.get<ReadonlyArray<ModelResponse>>(`/api/v1/models${query}`);
  }

  async getModel(modelId: string): Promise<ModelResponse> {
    return this.get<ModelResponse>(`/api/v1/models/${encodeURIComponent(modelId)}`);
  }

  async getLineage(cid: string): Promise<unknown> {
    return this.get(`/api/v1/lineage/${encodeURIComponent(cid)}`);
  }

  // -----------------------------------------------------------------------
  // Mesh
  // -----------------------------------------------------------------------

  async getPeers(): Promise<PeersResponse> {
    return this.get<PeersResponse>("/api/v1/mesh/peers");
  }

  async getNetworkInfo(): Promise<MeshStatusResponse> {
    return this.get<MeshStatusResponse>("/api/v1/mesh/status");
  }

  async discover(addr: string): Promise<{ addr: string; status: string }> {
    return this.post("/api/v1/mesh/connect", { addr });
  }

  // -----------------------------------------------------------------------
  // Health
  // -----------------------------------------------------------------------

  async health(): Promise<{ status: string }> {
    return this.get("/api/v1/health");
  }

  // -----------------------------------------------------------------------
  // Generic request helpers
  // -----------------------------------------------------------------------

  private async get<T>(path: string): Promise<T> {
    return this.request<T>("GET", path);
  }

  private async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("POST", path, body);
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const init: RequestInit = {
        method,
        headers,
        signal: controller.signal,
      };
      if (body !== undefined) {
        (init as Record<string, unknown>).body = JSON.stringify(body);
      }

      const response = await fetch(url, init);

      const text = await response.text();

      if (!response.ok) {
        let envelope: ApiEnvelope<unknown>;
        try {
          envelope = JSON.parse(text) as ApiEnvelope<unknown>;
        } catch {
          throw new AgnetdClientError(
            `HTTP ${response.status}: ${text}`,
            `HTTP_${response.status}`,
          );
        }
        if (!envelope.success) {
          throw new AgnetdClientError(
            envelope.error.message,
            envelope.error.code,
          );
        }
        throw new AgnetdClientError(
          `HTTP ${response.status}`,
          `HTTP_${response.status}`,
        );
      }

      const envelope = JSON.parse(text) as ApiEnvelope<T>;
      if (!envelope.success) {
        throw new AgnetdClientError(
          envelope.error.message,
          envelope.error.code,
        );
      }
      return envelope.data;
    } catch (err) {
      if (err instanceof AgnetdClientError) throw err;
      if (err instanceof DOMException && err.name === "AbortError") {
        throw new AgnetdClientError(
          `request timed out after ${this.timeout}ms`,
          "TIMEOUT",
        );
      }
      if (
        err instanceof TypeError &&
        err.message.includes("fetch failed")
      ) {
        throw new AgnetdClientError(
          `cannot connect to agnetd at ${this.baseUrl} — is the daemon running?`,
          "CONNECTION_REFUSED",
        );
      }
      throw new AgnetdClientError(
        err instanceof Error ? err.message : String(err),
        "NETWORK_ERROR",
      );
    } finally {
      clearTimeout(timeoutId);
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function buildQuery(
  params?: Record<string, unknown>,
): string {
  if (!params) return "";
  const entries = Object.entries(params).filter(
    ([, v]) => v !== undefined && v !== null,
  );
  if (entries.length === 0) return "";
  const qs = entries
    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
    .join("&");
  return `?${qs}`;
}

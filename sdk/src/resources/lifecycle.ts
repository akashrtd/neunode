import type { NeunodeClient } from "../client/client.js";
import type { Did } from "../types/core.js";

export type LifecycleState =
	| "CREATED"
	| "ACTIVE"
	| "HIBERNATING"
	| "IDLE"
	| "ZOMBIE"
	| "DEAD";

export interface LifecycleStatus {
	readonly did: Did;
	readonly state: LifecycleState;
	readonly last_activity: number;
	readonly elapsed_secs: number;
	readonly activated_at: number | null;
	readonly warning: string | null;
}

export interface LifecycleNoRecord {
	readonly message: string;
}

export type LifecycleStatusResult = LifecycleStatus | LifecycleNoRecord;

export interface LifecycleAck {
	readonly message: string;
}

export interface LifecycleAgentSummary {
	readonly did: Did;
	readonly state: LifecycleState;
	readonly last_activity: number;
}

export interface LifecycleTransition {
	readonly did: Did;
	readonly from: LifecycleState;
	readonly to: LifecycleState;
}

export interface LifecycleReapResult {
	readonly transitions: readonly LifecycleTransition[];
	readonly count: number;
}

export interface LifecycleResource {
	status(): Promise<LifecycleStatusResult>;
	activate(): Promise<LifecycleAck>;
	hibernate(): Promise<LifecycleAck>;
	reactivate(): Promise<LifecycleAck>;
	list(): Promise<readonly LifecycleAgentSummary[]>;
	reap(): Promise<LifecycleReapResult>;
}

export function createLifecycleResource(
	client: NeunodeClient,
): LifecycleResource {
	const http = () => {
		if (!client.http)
			throw new Error("HTTP transport required for lifecycle operations");
		return client.http;
	};
	return {
		async status() {
			return http().get<LifecycleStatusResult>("/api/v1/lifecycle/status");
		},
		async activate() {
			return http().post<LifecycleAck>("/api/v1/lifecycle/activate");
		},
		async hibernate() {
			return http().post<LifecycleAck>("/api/v1/lifecycle/hibernate");
		},
		async reactivate() {
			return http().post<LifecycleAck>("/api/v1/lifecycle/reactivate");
		},
		async list() {
			return http().get<readonly LifecycleAgentSummary[]>(
				"/api/v1/lifecycle/list",
			);
		},
		async reap() {
			return http().post<LifecycleReapResult>("/api/v1/lifecycle/reap");
		},
	};
}

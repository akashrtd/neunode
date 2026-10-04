import type { NeunodeClient } from "../client/client.js";

export type BreakerName = "token_volume" | "reputation" | "bounty_drain";
export interface BreakerStatus {
	readonly name: BreakerName;
	readonly open: boolean;
	readonly trip_count: number;
	readonly tripped_at: number | null;
	readonly mode: "manual";
}
export interface SecurityResource {
	breakers(): Promise<BreakerStatus[]>;
	setBreaker(name: BreakerName, open: boolean): Promise<BreakerStatus>;
}
export function createSecurityResource(
	client: NeunodeClient,
): SecurityResource {
	return {
		breakers: () => client.http.get("/api/v1/security/breakers"),
		setBreaker: (name, open) =>
			client.http.post(`/api/v1/security/breakers/${name}`, { open }),
	};
}

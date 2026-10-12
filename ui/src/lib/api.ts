// Talks to the daemon's HTTP API (docs/decisions/0014). The page is served by
// the daemon (or, while developing, by Vite, which forwards /api to it), so
// every path is relative to this page's own address.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { Added } from '@ottie/api-types/Added';
import type { ApiError } from '@ottie/api-types/ApiError';
import type { Job } from '@ottie/api-types/Job';

const BASE = '/api/v1';
/** The protocol of the events WebSocket; the token rides along as a second one. */
const EVENTS_PROTOCOL = 'ottie.v1';
const TOKEN_PROTOCOL_PREFIX = 'ottie.token.';

/** The daemon refused the token. */
export class Unauthorized extends Error {
	constructor() {
		super('The token was refused.');
	}
}

/** Any other refusal. `body` holds the daemon's code (to translate, see
 * `explain.ts`) and English text; it's null if the answer had no body. */
export class ApiFailure extends Error {
	constructor(
		readonly status: number,
		readonly body: ApiError | null
	) {
		super(body?.error ?? `The daemon answered ${status}.`);
	}
}

export class Api {
	constructor(private readonly token: string) {}

	listJobs(): Promise<Job[]> {
		return this.request('GET', '/jobs');
	}

	addJob(job: AddJob): Promise<Added> {
		return this.request('POST', '/jobs', job);
	}

	pause(id: string): Promise<void> {
		return this.request('POST', `/jobs/${encodeURIComponent(id)}/pause`);
	}

	resume(id: string): Promise<void> {
		return this.request('POST', `/jobs/${encodeURIComponent(id)}/resume`);
	}

	cancel(id: string): Promise<void> {
		return this.request('POST', `/jobs/${encodeURIComponent(id)}/cancel`);
	}

	/** Takes the job off the list (stopping it first if it runs). */
	remove(id: string): Promise<void> {
		return this.request('DELETE', `/jobs/${encodeURIComponent(id)}`);
	}

	/** Opens the live events stream. */
	events(): WebSocket {
		const url = new URL(`${BASE}/events`, location.href);
		url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
		return new WebSocket(url, [EVENTS_PROTOCOL, TOKEN_PROTOCOL_PREFIX + this.token]);
	}

	private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
		const headers: Record<string, string> = { Authorization: `Bearer ${this.token}` };
		if (body !== undefined) {
			headers['Content-Type'] = 'application/json';
		}
		const response = await fetch(BASE + path, {
			method,
			headers,
			body: body === undefined ? undefined : JSON.stringify(body)
		});
		if (response.status === 401) {
			throw new Unauthorized();
		}
		if (!response.ok) {
			const body = await response.json().then(
				(answer: ApiError) => answer,
				() => null
			);
			throw new ApiFailure(response.status, body);
		}
		if (response.status === 204) {
			return undefined as T;
		}
		return (await response.json()) as T;
	}
}

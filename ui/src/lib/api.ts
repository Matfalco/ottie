// Talks to the daemon's HTTP API (docs/decisions/0014). The page is served by
// the daemon (or, while developing, by Vite, which forwards /api to it), so
// every path is relative to this page's own address.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { Added } from '@ottie/api-types/Added';
import type { AddedCategory } from '@ottie/api-types/AddedCategory';
import type { Category } from '@ottie/api-types/Category';
import type { CategoryBody } from '@ottie/api-types/CategoryBody';
import type { MatchLink } from '@ottie/api-types/MatchLink';
import type { Matched } from '@ottie/api-types/Matched';
import type { ApiError } from '@ottie/api-types/ApiError';
import type { Job } from '@ottie/api-types/Job';
import type { JobChange } from '@ottie/api-types/JobChange';
import type { Settings } from '@ottie/api-types/Settings';
import type { SettingsChange } from '@ottie/api-types/SettingsChange';
import type { AddedView } from '@ottie/api-types/AddedView';
import type { View } from '@ottie/api-types/View';
import type { ViewBody } from '@ottie/api-types/ViewBody';

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

	/** Changes the job's own settings given (speed limit, priority). */
	changeJob(id: string, change: JobChange): Promise<void> {
		return this.request('PATCH', `/jobs/${encodeURIComponent(id)}`, change);
	}

	/** Moves the job to another category, as the user's choice. */
	setCategory(id: string, category: number): Promise<void> {
		return this.request('POST', `/jobs/${encodeURIComponent(id)}/category`, { category });
	}

	listCategories(): Promise<Category[]> {
		return this.request('GET', '/categories');
	}

	addCategory(category: CategoryBody): Promise<AddedCategory> {
		return this.request('POST', '/categories', category);
	}

	/** Replaces every setting of the category. */
	updateCategory(id: number, category: CategoryBody): Promise<void> {
		return this.request('PUT', `/categories/${id}`, category);
	}

	/** Its downloads move to General first. */
	removeCategory(id: number): Promise<void> {
		return this.request('DELETE', `/categories/${id}`);
	}

	/** The category a download of `url` would get (saved in `destDir`, if given). */
	matchCategory(url: string, destDir?: string): Promise<Matched> {
		const link: MatchLink = { url, destDir };
		return this.request('POST', '/categories/match', link);
	}

	getSettings(): Promise<Settings> {
		return this.request('GET', '/settings');
	}

	/** Changes only the fields given. */
	changeSettings(change: SettingsChange): Promise<void> {
		return this.request('PATCH', '/settings', change);
	}

	listViews(): Promise<View[]> {
		return this.request('GET', '/views');
	}

	addView(view: ViewBody): Promise<AddedView> {
		return this.request('POST', '/views', view);
	}

	/** Replaces the view, name included. */
	updateView(id: number, view: ViewBody): Promise<void> {
		return this.request('PUT', `/views/${id}`, view);
	}

	removeView(id: number): Promise<void> {
		return this.request('DELETE', `/views/${id}`);
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

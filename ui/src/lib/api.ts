// Talks to the daemon's HTTP API (docs/decisions/0014). The page is served by
// the daemon (or, while developing, by Vite, which forwards /api to it), so
// every path is relative to this page's own address.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { AddJobs } from '@ottie/api-types/AddJobs';
import type { Added } from '@ottie/api-types/Added';
import type { AddedJobs } from '@ottie/api-types/AddedJobs';
import type { AddedCategory } from '@ottie/api-types/AddedCategory';
import type { Category } from '@ottie/api-types/Category';
import type { CategoryBody } from '@ottie/api-types/CategoryBody';
import type { MatchLink } from '@ottie/api-types/MatchLink';
import type { Matched } from '@ottie/api-types/Matched';
import type { ApiError } from '@ottie/api-types/ApiError';
import type { Job } from '@ottie/api-types/Job';
import type { JobChange } from '@ottie/api-types/JobChange';
import type { Preview } from '@ottie/api-types/Preview';
import type { PreviewLinks } from '@ottie/api-types/PreviewLinks';
import type { Reorder } from '@ottie/api-types/Reorder';
import type { ReorderJobs } from '@ottie/api-types/ReorderJobs';
import type { Settings } from '@ottie/api-types/Settings';
import type { SettingsChange } from '@ottie/api-types/SettingsChange';
import type { SpeedHistory } from '@ottie/api-types/SpeedHistory';
import type { StreamLink } from '@ottie/api-types/StreamLink';
import type { AddedSlot } from '@ottie/api-types/AddedSlot';
import type { AddedView } from '@ottie/api-types/AddedView';
import type { ReorderSlots } from '@ottie/api-types/ReorderSlots';
import type { SlotBody } from '@ottie/api-types/SlotBody';
import type { SlotList } from '@ottie/api-types/SlotList';
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

	/** Adds the jobs that can be, in order; says which were refused and why. */
	addJobs(jobs: AddJob[]): Promise<AddedJobs> {
		const body: AddJobs = { jobs };
		return this.request('POST', '/jobs/batch', body);
	}

	/** The links in `text`, and what adding each would do (saved in
	 * `destDir`, if given). Adds nothing. */
	previewLinks(text: string, destDir?: string): Promise<Preview> {
		const body: PreviewLinks = { text, destDir };
		return this.request('POST', '/jobs/preview', body);
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

	/** Moves the jobs in the queue, among the waiting ones of their priority. */
	reorder(ids: string[], to: Reorder): Promise<void> {
		const body: ReorderJobs = { ids, to };
		return this.request('POST', '/jobs/reorder', body);
	}

	/** A link that plays the job's file, even unfinished, without the token
	 * (docs/decisions/0029). Always the same one while it's in use. */
	streamLink(id: string): Promise<StreamLink> {
		return this.request('POST', `/jobs/${encodeURIComponent(id)}/stream`);
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

	/** The time slots in their order, and the daemon's time zone. */
	listSlots(): Promise<SlotList> {
		return this.request('GET', '/slots');
	}

	/** The total download speed over the last hour, one sample per second. */
	speedHistory(): Promise<SpeedHistory> {
		return this.request('GET', '/speed');
	}

	addSlot(slot: SlotBody): Promise<AddedSlot> {
		return this.request('POST', '/slots', slot);
	}

	updateSlot(id: number, slot: SlotBody): Promise<void> {
		return this.request('PUT', `/slots/${id}`, slot);
	}

	removeSlot(id: number): Promise<void> {
		return this.request('DELETE', `/slots/${id}`);
	}

	/** Every slot's ID, once, in the new order. */
	reorderSlots(ids: number[]): Promise<void> {
		const body: ReorderSlots = { ids };
		return this.request('POST', '/slots/reorder', body);
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

// What the downloads page shows, kept live: the job list from `GET /jobs`,
// updated by the events WebSocket, which reconnects on its own.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { CategoryBody } from '@ottie/api-types/CategoryBody';
import type { JobChange } from '@ottie/api-types/JobChange';
import type { Matched } from '@ottie/api-types/Matched';
import type { Settings } from '@ottie/api-types/Settings';
import type { SettingsChange } from '@ottie/api-types/SettingsChange';
import type { Event as ApiEvent } from '@ottie/api-types/Event';
import type { ViewBody } from '@ottie/api-types/ViewBody';
import { Api, ApiFailure, Unauthorized } from './api';
import type { Action } from './bulk';
import { GENERAL, applyCategoryEvent, sorted, type Categories } from './categories';
import { explainRefusal } from './explain';
import { m } from './paraglide/messages.js';
import { applyEvent, newestFirst, nextRate, type Jobs, type Rate } from './jobs';
import { addSample, totalSpeed } from './speed';
import { applyViewEvent, sortedViews, type Views } from './views';

export type Connection = 'connecting' | 'live' | 'offline';

const FIRST_RETRY_MS = 500;
const LONGEST_RETRY_MS = 10_000;

export class Session {
	jobs: Jobs = $state({});
	rates: Record<string, Rate> = $state({});
	connection: Connection = $state('connecting');
	/** The last thing that went wrong, for a message on the page. */
	error: string | null = $state(null);
	/** Updated every second, so "speed" and "retrying in" stay current. */
	now = $state(Date.now());
	/** Total speed, one sample per second since the page opened (the last
	 * few minutes), for the dashboard's graph. */
	history: number[] = $state.raw([]);
	list = $derived(newestFirst(this.jobs));
	categories: Categories = $state({});
	/** General first, then by name. */
	categoryList = $derived(sorted(this.categories));
	/** Null until the daemon has answered. */
	settings: Settings | null = $state(null);
	views: Views = $state({});
	/** Oldest first. */
	viewList = $derived(sortedViews(this.views));

	private readonly api: Api;
	private socket: WebSocket | null = null;
	/** Events that arrive while the list is being fetched: applied after it. */
	private pending: ApiEvent[] | null = null;
	private retryMs = FIRST_RETRY_MS;
	private timers: ReturnType<typeof setTimeout>[] = [];
	private stopped = false;

	constructor(
		token: string,
		private readonly onUnauthorized: () => void
	) {
		this.api = new Api(token);
	}

	start(): void {
		this.connect();
		const clock = setInterval(() => {
			this.now = Date.now();
			this.history = addSample(this.history, totalSpeed(this.rates, this.now));
		}, 1000);
		this.timers.push(clock);
	}

	stop(): void {
		this.stopped = true;
		this.socket?.close();
		this.timers.forEach((timer) => clearTimeout(timer));
	}

	async add(job: AddJob): Promise<boolean> {
		return this.attempt(() => this.api.addJob(job));
	}

	/** The category a download of `url` (saved in `destDir`, if given)
	 * would get; null if the daemon can't tell (adding it will say why). */
	async match(url: string, destDir?: string): Promise<Matched | null> {
		try {
			return await this.api.matchCategory(url, destDir);
		} catch {
			return null;
		}
	}

	/** Does it to every job in `ids` at once; any failure shows on the page. */
	async run(action: Action, ids: string[]): Promise<void> {
		const request = {
			resume: (id: string) => this.api.resume(id),
			pause: (id: string) => this.api.pause(id),
			cancel: (id: string) => this.api.cancel(id),
			remove: (id: string) => this.api.remove(id)
		}[action];
		await Promise.all(ids.map((id) => this.attempt(() => request(id))));
	}

	/** Moves the jobs to `category`, as the user's choice. */
	async move(ids: string[], category: number): Promise<void> {
		await Promise.all(ids.map((id) => this.attempt(() => this.api.setCategory(id, category))));
	}

	/** Changes one download's own speed limit or priority; every page hears
	 * of it through a `changed` event. Resolves to null once done, else to
	 * why not. */
	async changeJob(id: string, change: JobChange): Promise<string | null> {
		try {
			await this.api.changeJob(id, change);
			return null;
		} catch (error) {
			return this.why(error);
		}
	}

	/** Adds a category (`id` null) or replaces one's settings. Resolves to
	 * null when saved, else to why not (in the page's language). */
	async saveCategory(id: number | null, body: CategoryBody): Promise<string | null> {
		try {
			await (id === null ? this.api.addCategory(body) : this.api.updateCategory(id, body));
			return null;
		} catch (error) {
			return this.why(error);
		}
	}

	removeCategory(id: number): Promise<boolean> {
		return this.attempt(() => this.api.removeCategory(id));
	}

	/** Saves a new view (`id` null) or replaces one; every page hears of it
	 * through a `viewChanged` event. Resolves to null once saved, else to
	 * why not. */
	async saveView(id: number | null, body: ViewBody): Promise<string | null> {
		try {
			await (id === null ? this.api.addView(body) : this.api.updateView(id, body));
			return null;
		} catch (error) {
			return this.why(error);
		}
	}

	removeView(id: number): Promise<boolean> {
		return this.attempt(() => this.api.removeView(id));
	}

	/** Changes the daemon's settings; every page (here and on other devices)
	 * hears of it through the `settingsChanged` event. Resolves to null
	 * once done, else to why not. */
	async changeSettings(change: SettingsChange): Promise<string | null> {
		try {
			await this.api.changeSettings(change);
			return null;
		} catch (error) {
			return this.why(error);
		}
	}

	/** Ends the first-start setup: General's folder (if it changed), then the
	 * categories, in order. Resolves to null once done, else to why not; what
	 * was created stays, and trying again skips it. */
	async finishSetup(downloads: string | null, bodies: CategoryBody[]): Promise<string | null> {
		const general = this.categories[GENERAL];
		if (general && downloads !== null && downloads !== general.folder) {
			const { name, extensions, speedLimit, maxConcurrent } = general;
			const body = { name, folder: downloads, extensions, speedLimit, maxConcurrent };
			const why = await this.saveCategory(GENERAL, body);
			if (why !== null) {
				return why;
			}
		}
		for (const body of bodies) {
			const why = await this.saveCategory(null, body);
			if (why !== null) {
				return why;
			}
		}
		try {
			await this.api.changeSettings({ setupDone: true });
			if (this.settings) {
				this.settings.setupDone = true;
			}
			return null;
		} catch (error) {
			return this.why(error);
		}
	}

	/** Runs a request; on failure shows why and returns false. */
	private async attempt(request: () => Promise<unknown>): Promise<boolean> {
		try {
			await request();
			this.error = null;
			return true;
		} catch (error) {
			this.fail(error);
			return false;
		}
	}

	private fail(error: unknown): void {
		const why = this.why(error);
		if (why !== null) {
			this.error = why;
		}
	}

	/** What went wrong, for the page; null after signing out (a refused token). */
	private why(error: unknown): string | null {
		if (error instanceof Unauthorized) {
			this.stop();
			this.onUnauthorized();
			return null;
		}
		if (error instanceof ApiFailure) {
			return explainRefusal(error.body, error.message);
		}
		// fetch() failed: the daemon is stopped or unreachable.
		return m.connection_offline();
	}

	private connect(): void {
		if (this.stopped) {
			return;
		}
		this.connection = 'connecting';
		let opened = false;
		const socket = this.api.events();
		this.socket = socket;
		socket.onopen = () => {
			opened = true;
			this.retryMs = FIRST_RETRY_MS;
			void this.resync();
		};
		socket.onmessage = (message: MessageEvent<string>) => {
			this.receive(JSON.parse(message.data) as ApiEvent);
		};
		socket.onclose = () => {
			if (this.stopped) {
				return;
			}
			this.connection = 'offline';
			// A refused token closes the socket without saying why; a plain
			// request tells a wrong token from a stopped daemon.
			if (!opened) {
				this.api.listJobs().catch((error: unknown) => {
					if (error instanceof Unauthorized) {
						this.fail(error);
					}
				});
			}
			this.timers.push(setTimeout(() => this.connect(), this.retryMs));
			this.retryMs = Math.min(this.retryMs * 2, LONGEST_RETRY_MS);
		};
	}

	private receive(event: ApiEvent): void {
		if (this.pending) {
			this.pending.push(event);
			return;
		}
		if (event.type === 'progress') {
			this.rates[event.id] = nextRate(this.rates[event.id], event.downloaded, Date.now());
		} else if (event.type === 'removed') {
			delete this.rates[event.id];
		} else if (event.type === 'settingsChanged') {
			this.settings = event.settings;
		}
		applyCategoryEvent(this.categories, event);
		applyViewEvent(this.views, event);
		if (applyEvent(this.jobs, event)) {
			void this.resync();
		}
	}

	/** Fetches the whole list, then replays what arrived meanwhile. Events are
	 * full states in order, so the replay always ends on the latest state. */
	private async resync(): Promise<void> {
		if (this.pending) {
			return; // Already fetching.
		}
		this.pending = [];
		try {
			const [jobs, categories, settings, views] = await Promise.all([
				this.api.listJobs(),
				this.api.listCategories(),
				this.api.getSettings(),
				this.api.listViews()
			]);
			this.jobs = Object.fromEntries(jobs.map((job) => [job.id, job]));
			this.categories = Object.fromEntries(categories.map((category) => [category.id, category]));
			this.settings = settings;
			this.views = Object.fromEntries(views.map((view) => [view.id, view]));
			this.connection = 'live';
		} catch (error) {
			this.fail(error);
		} finally {
			const missed = this.pending ?? [];
			this.pending = null;
			missed.forEach((event) => this.receive(event));
		}
	}
}

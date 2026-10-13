// What the downloads page shows, kept live: the job list from `GET /jobs`,
// updated by the events WebSocket, which reconnects on its own.

import type { Event as ApiEvent } from '@ottie/api-types/Event';
import { Api, ApiFailure, Unauthorized } from './api';
import type { Action } from './bulk';
import { explainRefusal } from './explain';
import { m } from './paraglide/messages.js';
import { applyEvent, newestFirst, nextRate, type Jobs, type Rate } from './jobs';
import { addSample, totalSpeed } from './speed';

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

	async add(url: string, destDir?: string): Promise<boolean> {
		return this.attempt(() => this.api.addJob({ url, destDir }));
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
		if (error instanceof Unauthorized) {
			this.stop();
			this.onUnauthorized();
		} else if (error instanceof ApiFailure) {
			this.error = explainRefusal(error.body, error.message);
		} else {
			// fetch() failed: the daemon is stopped or unreachable.
			this.error = m.connection_offline();
		}
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
		}
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
			const jobs = await this.api.listJobs();
			this.jobs = Object.fromEntries(jobs.map((job) => [job.id, job]));
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

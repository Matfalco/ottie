// Ottie's own tray menu in the desktop app (docs/decisions/0034): its status
// line, and the downloads "Pause all" and "Resume all" act on.

import type { Job } from '@ottie/api-types/Job';
import { formatCount, formatSpeed } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import type { Connection } from './session.svelte';

const RUNNING: Job['state'][] = ['connecting', 'downloading', 'retrying'];

/** "2 downloading, 5 waiting · 1.5 MB/s", "Nothing downloading". */
export function trayStatus(
	jobs: Job[],
	speed: number,
	connection: Connection,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	if (connection !== 'live') {
		return m.connection_connecting({}, options);
	}
	const running = jobs.filter((job) => RUNNING.includes(job.state)).length;
	const waiting = jobs.filter((job) => job.state === 'queued').length;
	const parts = [
		running > 0 ? m.tray_running({ count: formatCount(running, locale) }, options) : null,
		waiting > 0 ? m.tray_waiting({ count: formatCount(waiting, locale) }, options) : null
	].filter((part) => part !== null);
	if (parts.length === 0) {
		return m.tray_idle({}, options);
	}
	const status = parts.join(', ');
	return running > 0 ? `${status} · ${formatSpeed(speed, locale)}` : status;
}

/** "Pause all", in two steps: the waiting downloads first, so pausing a
 * running one doesn't start one of them, then the running ones. */
export function pauseSteps(jobs: Job[]): { waiting: string[]; running: string[] } {
	const ids = (states: Job['state'][]) =>
		jobs.filter((job) => !job.moving && states.includes(job.state)).map((job) => job.id);
	return { waiting: ids(['queued']), running: ids(RUNNING) };
}

/** "Resume all": the paused downloads. Failed ones need a look first. */
export function resumeTargets(jobs: Job[]): string[] {
	return jobs.filter((job) => !job.moving && job.state === 'paused').map((job) => job.id);
}

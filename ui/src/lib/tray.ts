// Ottie's own tray menu in the desktop app (docs/decisions/0034): its status
// line and the downloads it shows with their bars.

import type { Job } from '@ottie/api-types/Job';
import { formatCount, formatSpeed } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import type { Connection } from './session.svelte';

const RUNNING: Job['state'][] = ['connecting', 'downloading', 'retrying'];
/** The running downloads the menu shows with their bars, at most. */
const SHOWN = 3;

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

/** The running downloads the menu shows, in the list's order, and how many
 * more run. Never just one more: it takes the place of the "+1" line. */
export function trayRunning(jobs: Job[]): { shown: Job[]; more: number } {
	const running = jobs.filter((job) => RUNNING.includes(job.state));
	const shown = running.length > SHOWN + 1 ? running.slice(0, SHOWN) : running;
	return { shown, more: running.length - shown.length };
}

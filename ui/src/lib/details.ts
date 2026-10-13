// What the panel under the Downloads list says about the selected download.

import type { Job } from '@ottie/api-types/Job';
import type { Priority } from '@ottie/api-types/Priority';
import type { Action } from './bulk';
import { fileKind } from './filetypes';
import { displayName, formatAdded, formatBytes, formatDuration, formatSpeed } from './format';
import { addedAt } from './jobs';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import { canPause } from './status';

/** In the order the panel offers them. */
export const PRIORITIES: Priority[] = ['low', 'normal', 'high', 'urgent'];

export function priorityName(priority: Priority, locale: Locale = getLocale()): string {
	const options = { locale };
	switch (priority) {
		case 'low':
			return m.priority_low({}, options);
		case 'normal':
			return m.priority_normal({}, options);
		case 'high':
			return m.priority_high({}, options);
		case 'urgent':
			return m.priority_urgent({}, options);
	}
}

/** The panel's main button: what you'd most likely do next, if anything. */
export function primaryAction(
	job: Job,
	locale: Locale = getLocale()
): { action: Action; label: string } | null {
	const options = { locale };
	if (canPause(job)) {
		return { action: 'pause', label: m.pause({}, options) };
	}
	if (job.state === 'failed') {
		return { action: 'resume', label: m.det_try_again({}, options) };
	}
	if (job.state === 'paused') {
		return { action: 'resume', label: m.resume({}, options) };
	}
	return null;
}

/** Videos and music, which can be played before they finish (later). */
export function isPlayable(job: Job): boolean {
	const kind = fileKind(displayName(job));
	return kind === 'video' || kind === 'audio';
}

export interface Fact {
	label: string;
	value: string;
}

/** A few numbers that matter in the job's state. `speed` is bytes per
 * second now; `maxRetries` is the daemon's setting, if known. */
export function facts(
	job: Job,
	speed: number,
	now: number,
	maxRetries: number | null,
	locale: Locale = getLocale()
): Fact[] {
	const options = { locale };
	const unknown = m.det_unknown({}, options);
	const size =
		job.total !== null
			? formatBytes(job.total, locale)
			: job.state === 'completed'
				? formatBytes(job.downloaded, locale)
				: unknown;
	const fact = (label: string, value: string): Fact => ({ label, value });
	const sizeFact = fact(m.col_size({}, options), size);
	const retries = fact(
		m.det_retries({}, options),
		maxRetries === null
			? String(job.retries)
			: m.det_of({ count: String(job.retries), max: String(maxRetries) }, options)
	);
	const added = addedAt(job.id);
	const addedFact = fact(
		m.col_added({}, options),
		added ? formatAdded(added, new Date(now), locale) : unknown
	);

	switch (job.state) {
		case 'connecting':
		case 'downloading': {
			const left =
				speed > 0 && job.total !== null
					? formatDuration((job.total - job.downloaded) / speed, locale)
					: '—';
			return [
				fact(m.col_speed({}, options), speed > 0 ? formatSpeed(speed, locale) : '—'),
				fact(m.col_left({}, options), left),
				sizeFact,
				addedFact
			];
		}
		case 'retrying': {
			const next = job.retryAt === null ? '—' : formatDuration((job.retryAt - now) / 1000, locale);
			return [fact(m.det_next_try({}, options), next), retries, sizeFact, addedFact];
		}
		case 'queued':
			return [
				sizeFact,
				fact(m.det_priority({}, options), priorityName(job.priority, locale)),
				addedFact
			];
		case 'paused':
			return [
				fact(m.det_downloaded({}, options), formatBytes(job.downloaded, locale)),
				sizeFact,
				fact(
					m.det_resumable({}, options),
					job.resumable ? m.det_yes({}, options) : m.det_no({}, options)
				),
				addedFact
			];
		case 'failed':
			return [retries, sizeFact, addedFact];
		case 'completed':
			return [sizeFact, addedFact];
		case 'cancelled':
			return [addedFact];
	}
}

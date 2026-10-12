// What a job row says and which buttons it offers.

import type { Job } from '@ottie/api-types/Job';
import { explainProblem } from './explain';
import { formatBytes, formatDuration, formatSpeed } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** "12.3 MB of 45.6 MB", or just what's downloaded when the size is unknown. */
function amount(job: Job, locale: Locale): string {
	const done = formatBytes(job.downloaded, locale);
	return job.total === null
		? done
		: m.status_amount({ done, total: formatBytes(job.total, locale) }, { locale });
}

/** One line under the file name. `now` is `Date.now()`. */
export function describe(
	job: Job,
	bytesPerSec: number,
	now: number,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	const why = () => explainProblem(job.problem, job.error, locale);
	switch (job.state) {
		case 'queued':
			return job.retries > 0
				? join([m.status_waiting_retry({}, options), why()])
				: m.status_waiting({}, options);
		case 'connecting':
			return m.status_connecting({}, options);
		case 'downloading': {
			const parts = [amount(job, locale)];
			if (bytesPerSec > 0) {
				parts.push(formatSpeed(bytesPerSec, locale));
				if (job.total !== null) {
					const time = formatDuration((job.total - job.downloaded) / bytesPerSec, locale);
					parts.push(m.status_left({ time }, options));
				}
			}
			return join(parts);
		}
		case 'paused':
			return join([m.status_paused({}, options), amount(job, locale)]);
		case 'completed':
			return join([formatBytes(job.total ?? job.downloaded, locale), m.status_done({}, options)]);
		case 'failed':
			return m.status_failed({ reason: why() }, options);
		case 'retrying': {
			const count = String(job.retries);
			const retry =
				job.retryAt === null
					? m.status_retrying({ count }, options)
					: m.status_retrying_in(
							{ time: formatDuration((job.retryAt - now) / 1000, locale), count },
							options
						);
			return join([retry, why()]);
		}
		case 'cancelled':
			return m.status_cancelled({}, options);
	}
}

/** "a · b", skipping empty parts. */
function join(parts: string[]): string {
	return parts.filter(Boolean).join(' · ');
}

/** Between 0 and 1, or null when the size is unknown. */
export function fraction(job: Job): number | null {
	if (job.state === 'completed') {
		return 1;
	}
	if (job.total === null || job.total === 0) {
		return null;
	}
	return Math.min(1, job.downloaded / job.total);
}

export function canPause(job: Job): boolean {
	return ['queued', 'connecting', 'downloading', 'retrying'].includes(job.state);
}

export function canResume(job: Job): boolean {
	return job.state === 'paused' || job.state === 'failed';
}

export function canCancel(job: Job): boolean {
	return job.state !== 'completed' && job.state !== 'cancelled';
}

// What a job row says and which buttons it offers.

import type { Job } from '@ottie/api-types/Job';
import { formatBytes, formatDuration, formatSpeed } from './format';

/** "12.3 MB of 45.6 MB", or just what's downloaded when the size is unknown. */
function amount(job: Job): string {
	return job.total === null
		? formatBytes(job.downloaded)
		: `${formatBytes(job.downloaded)} of ${formatBytes(job.total)}`;
}

/** One line under the file name. `now` is `Date.now()`. */
export function describe(job: Job, bytesPerSec: number, now: number): string {
	switch (job.state) {
		case 'queued':
			return job.retries > 0
				? `Waiting for a free slot to retry · ${job.error ?? ''}`.trimEnd()
				: 'Waiting for a free slot';
		case 'connecting':
			return 'Connecting…';
		case 'downloading': {
			const parts = [amount(job)];
			if (bytesPerSec > 0) {
				parts.push(formatSpeed(bytesPerSec));
				if (job.total !== null) {
					parts.push(`${formatDuration((job.total - job.downloaded) / bytesPerSec)} left`);
				}
			}
			return parts.join(' · ');
		}
		case 'paused':
			return `Paused · ${amount(job)}`;
		case 'completed':
			return `${formatBytes(job.total ?? job.downloaded)} · Done`;
		case 'failed':
			return `Failed: ${job.error ?? 'unknown error'}`;
		case 'retrying': {
			const wait = job.retryAt === null ? '' : ` in ${formatDuration((job.retryAt - now) / 1000)}`;
			const why = job.error ? ` · ${job.error}` : '';
			return `Retrying${wait} (retry ${job.retries})${why}`;
		}
		case 'cancelled':
			return 'Cancelled';
	}
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

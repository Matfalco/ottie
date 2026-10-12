// Shared by the unit tests.

import type { Job } from '@ottie/api-types/Job';

/** A queued job with nothing known yet; override what a test needs. */
export function job(overrides: Partial<Job> = {}): Job {
	return {
		id: '2026.10.12-14.03.07-1',
		url: 'https://example.com/files/movie%20night.mkv?token=x',
		destDir: '/downloads',
		fileName: null,
		state: 'queued',
		downloaded: 0,
		total: null,
		resumable: false,
		error: null,
		problem: null,
		priority: 'normal',
		speedLimit: 0,
		category: 1,
		retries: 0,
		retryAt: null,
		...overrides
	};
}

/** French separates thousands and units with no-break spaces; tests read
 * better with plain ones. */
export function plain(text: string): string {
	return text.replace(/[  ]/g, ' ');
}

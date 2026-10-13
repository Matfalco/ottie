// Shared by the unit tests.

import type { Category } from '@ottie/api-types/Category';
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
		streaming: false,
		category: 1,
		retries: 0,
		retryAt: null,
		ranges: [],
		moving: false,
		...overrides
	};
}

/** General with a folder and no limits; override what a test needs (with
 * another ID, it's one of the user's own). */
export function category(overrides: Partial<Category> = {}): Category {
	return {
		id: 1,
		name: 'General',
		folder: '/downloads',
		extensions: [],
		speedLimit: 0,
		maxConcurrent: 0,
		builtIn: (overrides.id ?? 1) === 1 ? 'general' : null,
		...overrides
	};
}

/** French separates thousands and units with no-break spaces; tests read
 * better with plain ones. */
export function plain(text: string): string {
	return text.replace(/[  ]/g, ' ');
}

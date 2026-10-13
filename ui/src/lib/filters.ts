// The views in the sidebar: which jobs each one shows, and how many.

import type { Job } from '@ottie/api-types/Job';
import type { JobState } from '@ottie/api-types/JobState';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

export type Filter = 'all' | 'active' | 'queued' | 'stopped' | 'completed';

/** In sidebar order. */
export const FILTERS: Filter[] = ['all', 'active', 'queued', 'stopped', 'completed'];

/** The states each view shows (`all` shows every job). "Stopped" is what
 * won't go on by itself: paused, failed or cancelled. */
const STATES: Record<Exclude<Filter, 'all'>, JobState[]> = {
	active: ['connecting', 'downloading', 'retrying'],
	queued: ['queued'],
	stopped: ['paused', 'failed', 'cancelled'],
	completed: ['completed']
};

export function matches(job: Job, filter: Filter): boolean {
	return filter === 'all' || STATES[filter].includes(job.state);
}

export function countByFilter(jobs: Job[]): Record<Filter, number> {
	const counts: Record<Filter, number> = { all: 0, active: 0, queued: 0, stopped: 0, completed: 0 };
	for (const job of jobs) {
		for (const filter of FILTERS) {
			if (matches(job, filter)) {
				counts[filter] += 1;
			}
		}
	}
	return counts;
}

/** The filter named by the address's `?status=`; All when missing or unknown. */
export function parseFilter(value: string | null): Filter {
	return FILTERS.find((filter) => filter === value) ?? 'all';
}

/** The query that selects `filter` on the Downloads page (none for All). */
export function filterQuery(filter: Filter): string {
	return filter === 'all' ? '' : `?status=${filter}`;
}

/** Whether a job just added shows up in this view (it starts out queued,
 * then becomes active). */
export function showsNewJobs(filter: Filter): boolean {
	return filter === 'all' || filter === 'queued' || filter === 'active';
}

export function filterName(filter: Filter, locale: Locale = getLocale()): string {
	const options = { locale };
	switch (filter) {
		case 'all':
			return m.filter_all({}, options);
		case 'active':
			return m.filter_active({}, options);
		case 'queued':
			return m.filter_queued({}, options);
		case 'stopped':
			return m.filter_stopped({}, options);
		case 'completed':
			return m.filter_completed({}, options);
	}
}

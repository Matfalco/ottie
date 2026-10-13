// Which downloads the Downloads page shows: the filters, kept in the address
// (`?status=paused&category=3&q=iso&added=today`) so links, bookmarks and the
// Dashboard's tiles can open a filtered list; the quick filters above the
// list; and the four status groups.

import type { Category } from '@ottie/api-types/Category';
import type { Job } from '@ottie/api-types/Job';
import type { JobState } from '@ottie/api-types/JobState';
import { categoryName } from './categories';
import { displayName } from './format';
import { addedAt } from './jobs';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** What the Status filter offers: a job's state, with "connecting" counted
 * as downloading (it only lasts until the server answers). */
export type Status = Exclude<JobState, 'connecting'>;

/** In the order the filter lists them. */
export const STATUSES: Status[] = [
	'downloading',
	'retrying',
	'queued',
	'paused',
	'failed',
	'cancelled',
	'completed'
];

export function statusOf(job: Job): Status {
	return job.state === 'connecting' ? 'downloading' : job.state;
}

/** Running, or about to retry: what the Downloads tab counts in its badge. */
export function isActive(job: Job): boolean {
	return ['connecting', 'downloading', 'retrying'].includes(job.state);
}

export type Added = 'any' | 'today' | 'week';
export const ADDED: Added[] = ['any', 'today', 'week'];

export interface Filters {
	/** In `STATUSES` order; null: any status. */
	statuses: Status[] | null;
	/** Category IDs, ascending; null: any category. */
	categories: number[] | null;
	/** Part of the name, in any case; empty: any name. */
	query: string;
	added: Added;
}

export const NO_FILTERS: Filters = { statuses: null, categories: null, query: '', added: 'any' };

const DAY_MS = 24 * 60 * 60 * 1000;

/** `now` decides what "today" and "the past 7 days" mean (local time). */
export function matches(job: Job, filters: Filters, now: Date): boolean {
	const query = filters.query.trim().toLocaleLowerCase();
	if (filters.statuses !== null && !filters.statuses.includes(statusOf(job))) {
		return false;
	}
	if (filters.categories !== null && !filters.categories.includes(job.category)) {
		return false;
	}
	if (query !== '' && !displayName(job).toLocaleLowerCase().includes(query)) {
		return false;
	}
	if (filters.added === 'any') {
		return true;
	}
	const added = addedAt(job.id);
	if (added === null) {
		return false;
	}
	const since =
		filters.added === 'today'
			? new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
			: now.getTime() - 7 * DAY_MS;
	return added.getTime() >= since;
}

export function count(jobs: Job[], filters: Filters, now: Date): number {
	return jobs.filter((job) => matches(job, filters, now)).length;
}

/** How many of the four filters narrow the list (the badge on "Filters and sorting"). */
export function activeCount(filters: Filters): number {
	return [
		filters.statuses !== null,
		filters.categories !== null,
		filters.query.trim() !== '',
		filters.added !== 'any'
	].filter(Boolean).length;
}

export function sameFilters(a: Filters, b: Filters): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

function inOrder(statuses: Status[]): Status[] {
	return STATUSES.filter((status) => statuses.includes(status));
}

/** Ticks or unticks one status; all of them ticked means any status. */
export function toggleStatus(filters: Filters, status: Status): Filters {
	const now = filters.statuses ?? STATUSES;
	const next = now.includes(status) ? now.filter((each) => each !== status) : [...now, status];
	return { ...filters, statuses: next.length === STATUSES.length ? null : inOrder(next) };
}

/** Ticks or unticks one category; all of `all` ticked means any category. */
export function toggleCategory(filters: Filters, id: number, all: number[]): Filters {
	const now = filters.categories ?? all;
	const next = now.includes(id) ? now.filter((each) => each !== id) : [...now, id];
	const everything = all.every((each) => next.includes(each));
	return { ...filters, categories: everything ? null : next.sort((a, b) => a - b) };
}

/** The filters the address names. A list that's there but names nothing
 * known (`?status=`) is empty: it shows nothing, which is what unticking
 * every box means. */
export function parseFilters(params: Pick<URLSearchParams, 'has' | 'get' | 'getAll'>): Filters {
	const statuses = params.has('status')
		? inOrder(params.getAll('status').filter((value): value is Status => isStatus(value)))
		: null;
	const categories = params.has('category')
		? [
				...new Set(
					params
						.getAll('category')
						.filter((value) => /^\d+$/.test(value))
						.map(Number)
				)
			].sort((a, b) => a - b)
		: null;
	return {
		statuses: statuses?.length === STATUSES.length ? null : statuses,
		categories,
		query: params.get('q') ?? '',
		added: ADDED.find((added) => added === params.get('added')) ?? 'any'
	};
}

function isStatus(value: string): boolean {
	return STATUSES.some((status) => status === value);
}

/** The query that selects `filters` on the Downloads page ('' for none). */
export function filtersQuery(filters: Filters): string {
	const params = new URLSearchParams();
	const list = (name: string, values: (string | number)[] | null) => {
		if (values === null) {
			return;
		}
		if (values.length === 0) {
			params.append(name, '');
		}
		for (const value of values) {
			params.append(name, String(value));
		}
	};
	list('status', filters.statuses);
	list('category', filters.categories);
	if (filters.query !== '') {
		params.set('q', filters.query);
	}
	if (filters.added !== 'any') {
		params.set('added', filters.added);
	}
	const query = params.toString();
	return query === '' ? '' : `?${query}`;
}

/** Whether a download just added shows up with these filters: it starts
 * out queued, and its category isn't known yet. */
export function showsNewJobs(filters: Filters): boolean {
	return (
		(filters.statuses === null || filters.statuses.includes('queued')) &&
		filters.categories === null &&
		filters.query.trim() === ''
	);
}

/** How the list is grouped when grouping is on, in this order. The
 * Dashboard's tiles are the same four. */
export type Group = 'running' | 'attention' | 'waiting' | 'finished';
export const GROUPS: Group[] = ['running', 'attention', 'waiting', 'finished'];

const GROUP_STATUSES: Record<Group, Status[]> = {
	running: ['downloading'],
	attention: ['retrying', 'failed'],
	waiting: ['queued', 'paused'],
	finished: ['cancelled', 'completed']
};

export function groupOf(job: Job): Group {
	const status = statusOf(job);
	return GROUPS.find((group) => GROUP_STATUSES[group].includes(status)) ?? 'finished';
}

/** The filters that show one group. */
export function groupFilters(group: Group): Filters {
	return { ...NO_FILTERS, statuses: GROUP_STATUSES[group] };
}

/** A pill above the list. */
export interface QuickFilter {
	key: string;
	name: string;
	filters: Filters;
}

/** The pills always shown, then the ones behind "…": paused, added today,
 * and one per category given (the caller passes those that have downloads). */
export function quickFilters(
	categories: Category[],
	locale: Locale = getLocale()
): { main: QuickFilter[]; more: QuickFilter[] } {
	const options = { locale };
	const statuses = (key: string, name: string, statuses: Status[]): QuickFilter => ({
		key,
		name,
		filters: { ...NO_FILTERS, statuses }
	});
	return {
		main: [
			{ key: 'all', name: m.filter_all({}, options), filters: NO_FILTERS },
			statuses('active', m.filter_active({}, options), ['downloading', 'retrying']),
			statuses('attention', m.filter_attention({}, options), ['retrying', 'failed']),
			statuses('queued', m.filter_queued({}, options), ['queued']),
			statuses('completed', m.filter_completed({}, options), ['completed'])
		],
		more: [
			statuses('paused', m.filter_paused({}, options), ['paused']),
			{
				key: 'today',
				name: m.filter_today({}, options),
				filters: { ...NO_FILTERS, added: 'today' }
			},
			...categories.map((category) => ({
				key: `category-${category.id}`,
				name: categoryName(category, locale),
				filters: { ...NO_FILTERS, categories: [category.id] }
			}))
		]
	};
}

export function statusName(status: Status, locale: Locale = getLocale()): string {
	const options = { locale };
	switch (status) {
		case 'downloading':
			return m.state_downloading({}, options);
		case 'retrying':
			return m.state_retrying({}, options);
		case 'queued':
			return m.state_queued({}, options);
		case 'paused':
			return m.state_paused({}, options);
		case 'failed':
			return m.state_failed({}, options);
		case 'cancelled':
			return m.state_cancelled({}, options);
		case 'completed':
			return m.state_completed({}, options);
	}
}

export function groupName(group: Group, locale: Locale = getLocale()): string {
	const options = { locale };
	switch (group) {
		case 'running':
			return m.group_running({}, options);
		case 'attention':
			return m.group_attention({}, options);
		case 'waiting':
			return m.group_waiting({}, options);
		case 'finished':
			return m.group_finished({}, options);
	}
}

export function addedName(added: Added, locale: Locale = getLocale()): string {
	const options = { locale };
	switch (added) {
		case 'any':
			return m.added_any({}, options);
		case 'today':
			return m.added_today({}, options);
		case 'week':
			return m.added_week({}, options);
	}
}

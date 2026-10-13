import { describe, expect, it } from 'vitest';
import {
	NO_FILTERS,
	STATUSES,
	activeCount,
	count,
	filtersQuery,
	groupFilters,
	groupName,
	groupOf,
	matches,
	parseFilters,
	quickFilters,
	showsNewJobs,
	statusName,
	toggleCategory,
	toggleStatus,
	type Filters
} from './filters';
import { category, job } from './testing';

// The sample job's ID says it was added 2026-10-12 14:03:07 UTC.
const NOW = new Date('2026-10-12T18:00:00Z');
const params = (query: string) => new URLSearchParams(query);

describe('filters', () => {
	it('match by status, connecting counting as downloading', () => {
		const downloading: Filters = { ...NO_FILTERS, statuses: ['downloading'] };
		expect(matches(job({ state: 'connecting' }), downloading, NOW)).toBe(true);
		expect(matches(job({ state: 'queued' }), downloading, NOW)).toBe(false);
		expect(matches(job({ state: 'queued' }), NO_FILTERS, NOW)).toBe(true);
		expect(matches(job(), { ...NO_FILTERS, statuses: [] }, NOW)).toBe(false);
	});

	it('match by category, name and date added', () => {
		const iso = job({ category: 3, fileName: 'Ubuntu-24.04.iso' });
		expect(matches(iso, { ...NO_FILTERS, categories: [2, 3] }, NOW)).toBe(true);
		expect(matches(iso, { ...NO_FILTERS, categories: [2] }, NOW)).toBe(false);
		expect(matches(iso, { ...NO_FILTERS, query: ' ubuntu ' }, NOW)).toBe(true);
		expect(matches(iso, { ...NO_FILTERS, query: 'debian' }, NOW)).toBe(false);
		// Without a file name yet, the name comes from the link.
		expect(matches(job(), { ...NO_FILTERS, query: 'night' }, NOW)).toBe(true);

		const week: Filters = { ...NO_FILTERS, added: 'week' };
		expect(matches(job(), week, NOW)).toBe(true);
		expect(matches(job(), week, new Date('2026-10-20T00:00:00Z'))).toBe(false);
		expect(matches(job({ id: 'not-an-id' }), week, NOW)).toBe(false);
	});

	it('count what narrows the list', () => {
		expect(activeCount(NO_FILTERS)).toBe(0);
		expect(activeCount({ statuses: [], categories: [1], query: ' ', added: 'today' })).toBe(3);
		expect(count([job(), job({ state: 'completed' })], groupFilters('waiting'), NOW)).toBe(1);
	});

	it('tick and untick, all ticked meaning any', () => {
		const noPaused = toggleStatus(NO_FILTERS, 'paused');
		expect(noPaused.statuses).toEqual(STATUSES.filter((status) => status !== 'paused'));
		expect(toggleStatus(noPaused, 'paused').statuses).toBeNull();
		// Kept in the list's order, whatever the order of the clicks.
		const some = toggleStatus({ ...NO_FILTERS, statuses: ['failed'] }, 'downloading');
		expect(some.statuses).toEqual(['downloading', 'failed']);

		const all = [1, 2, 5];
		const only = toggleCategory(toggleCategory(NO_FILTERS, 5, all), 1, all);
		expect(only.categories).toEqual([2]);
		expect(toggleCategory(toggleCategory(only, 5, all), 1, all).categories).toBeNull();
	});

	it('round-trip through the address', () => {
		const filters: Filters = {
			statuses: ['retrying', 'failed'],
			categories: [2, 7],
			query: 'big file',
			added: 'today'
		};
		const query = filtersQuery(filters);
		expect(query).toBe(
			'?status=retrying&status=failed&category=2&category=7&q=big+file&added=today'
		);
		expect(parseFilters(params(query))).toEqual(filters);
		expect(filtersQuery(NO_FILTERS)).toBe('');
		expect(parseFilters(params(''))).toEqual(NO_FILTERS);
		// Unticking everything shows nothing, and survives the trip.
		const none: Filters = { ...NO_FILTERS, statuses: [], categories: [] };
		expect(parseFilters(params(filtersQuery(none)))).toEqual(none);
	});

	it('read odd addresses sensibly', () => {
		expect(parseFilters(params('status=nonsense')).statuses).toEqual([]);
		expect(parseFilters(params('status=failed&status=queued&status=failed')).statuses).toEqual([
			'queued',
			'failed'
		]);
		const every = STATUSES.map((status) => `status=${status}`).join('&');
		expect(parseFilters(params(every)).statuses).toBeNull();
		expect(parseFilters(params('category=3&category=x&category=1&category=3')).categories).toEqual([
			1, 3
		]);
		expect(parseFilters(params('added=yesterday')).added).toBe('any');
	});

	it('know where new jobs show up', () => {
		expect(showsNewJobs(NO_FILTERS)).toBe(true);
		expect(showsNewJobs(groupFilters('waiting'))).toBe(true);
		expect(showsNewJobs(groupFilters('finished'))).toBe(false);
		expect(showsNewJobs({ ...NO_FILTERS, categories: [1] })).toBe(false);
	});

	it('put every state in one group', () => {
		expect(groupOf(job({ state: 'connecting' }))).toBe('running');
		expect(groupOf(job({ state: 'retrying' }))).toBe('attention');
		expect(groupOf(job({ state: 'paused' }))).toBe('waiting');
		expect(groupOf(job({ state: 'cancelled' }))).toBe('finished');
	});

	it('offer quick filters, with one per category given', () => {
		const pills = quickFilters([category(), category({ id: 4, name: 'Videos' })], 'en');
		expect(pills.main.map((pill) => pill.name)).toEqual([
			'All',
			'Active',
			'Needs attention',
			'Queued',
			'Completed'
		]);
		expect(pills.more.map((pill) => pill.name)).toEqual([
			'Paused',
			'Added today',
			'General',
			'Videos'
		]);
		expect(pills.more[3].filters).toEqual({ ...NO_FILTERS, categories: [4] });
		expect(new Set([...pills.main, ...pills.more].map((pill) => pill.key)).size).toBe(9);
	});

	it('have names in each language', () => {
		expect(statusName('cancelled', 'en')).toBe('Cancelled');
		expect(statusName('cancelled', 'fr')).toBe('Annulé');
		expect(groupName('attention', 'fr')).toBe('À vérifier');
		expect(quickFilters([], 'fr').main[0].name).toBe('Tous');
	});
});

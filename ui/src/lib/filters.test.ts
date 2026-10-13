import { describe, expect, it } from 'vitest';
import {
	countByFilter,
	filterName,
	filterQuery,
	matches,
	parseFilter,
	showsNewJobs
} from './filters';
import { job } from './testing';

describe('filters', () => {
	it('put every state in one view besides All', () => {
		expect(matches(job({ state: 'retrying' }), 'active')).toBe(true);
		expect(matches(job({ state: 'paused' }), 'stopped')).toBe(true);
		expect(matches(job({ state: 'cancelled' }), 'stopped')).toBe(true);
		expect(matches(job({ state: 'completed' }), 'stopped')).toBe(false);
		expect(matches(job({ state: 'completed' }), 'all')).toBe(true);
	});

	it('count the jobs in each view', () => {
		const states = ['queued', 'downloading', 'connecting', 'failed', 'completed'] as const;
		expect(countByFilter(states.map((state) => job({ state })))).toEqual({
			all: 5,
			active: 2,
			queued: 1,
			stopped: 1,
			completed: 1
		});
	});

	it('round-trip through the address', () => {
		expect(parseFilter(null)).toBe('all');
		expect(parseFilter('nonsense')).toBe('all');
		expect(parseFilter('stopped')).toBe('stopped');
		expect(filterQuery('stopped')).toBe('?status=stopped');
		expect(filterQuery('all')).toBe('');
	});

	it('know where new jobs show up', () => {
		expect(showsNewJobs('queued')).toBe(true);
		expect(showsNewJobs('completed')).toBe(false);
	});

	it('have a name in each language', () => {
		expect(filterName('stopped', 'en')).toBe('Stopped');
		expect(filterName('stopped', 'fr')).toBe('Arrêtés');
	});
});

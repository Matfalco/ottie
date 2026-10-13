import { describe, expect, it } from 'vitest';
import {
	DEFAULT_ARRANGEMENT,
	arrange,
	directionName,
	groupNote,
	parseArrangement,
	sortBy,
	sortJobs
} from './arrange';
import { job, plain } from './testing';

const a = job({ id: '2026.10.12-10.00.00-1', fileName: 'b-file 10.iso', total: 500 });
const b = job({ id: '2026.10.12-11.00.00-2', fileName: 'B-file 9.iso', total: null });
const c = job({
	id: '2026.10.12-12.00.00-3',
	fileName: 'a.zip',
	state: 'completed',
	total: null,
	downloaded: 2000
});
const names = (jobs: { fileName: string | null }[]) => jobs.map((each) => each.fileName);

describe('arrangement', () => {
	it('sorts by date added, newest first by default', () => {
		expect(names(sortJobs([a, c, b], 'added', 'desc', 'en'))).toEqual([
			'a.zip',
			'B-file 9.iso',
			'b-file 10.iso'
		]);
		expect(names(sortJobs([c, a, b], 'added', 'asc', 'en'))).toEqual([
			'b-file 10.iso',
			'B-file 9.iso',
			'a.zip'
		]);
	});

	it('sorts names naturally, ignoring case', () => {
		expect(names(sortJobs([a, b, c], 'name', 'asc', 'en'))).toEqual([
			'a.zip',
			'B-file 9.iso',
			'b-file 10.iso'
		]);
	});

	it('puts unknown sizes and progress last in both directions', () => {
		// A finished download's size is what it downloaded.
		expect(names(sortJobs([a, b, c], 'size', 'desc', 'en'))).toEqual([
			'a.zip',
			'b-file 10.iso',
			'B-file 9.iso'
		]);
		expect(names(sortJobs([b, c, a], 'size', 'asc', 'en'))).toEqual([
			'b-file 10.iso',
			'a.zip',
			'B-file 9.iso'
		]);
		expect(names(sortJobs([b, a, c], 'progress', 'asc', 'en'))).toEqual([
			'b-file 10.iso',
			'a.zip',
			'B-file 9.iso'
		]);
	});

	it('turns the order around on a second click', () => {
		const bySize = sortBy(DEFAULT_ARRANGEMENT, 'size');
		expect(bySize).toEqual({ sort: 'size', direction: 'desc', grouped: true });
		expect(sortBy(bySize, 'size').direction).toBe('asc');
		expect(sortBy(bySize, 'name').direction).toBe('asc');
	});

	it('groups by status, leaving empty groups out', () => {
		const sections = arrange([a, c, b], DEFAULT_ARRANGEMENT, 'en');
		expect(sections.map((section) => section.group)).toEqual(['waiting', 'finished']);
		expect(names(sections[0].jobs)).toEqual(['B-file 9.iso', 'b-file 10.iso']);
		const flat = arrange([a, c, b], { ...DEFAULT_ARRANGEMENT, grouped: false }, 'en');
		expect(flat).toEqual([{ group: null, jobs: [c, b, a] }]);
		expect(arrange([], { ...DEFAULT_ARRANGEMENT, grouped: false }, 'en')).toEqual([]);
	});

	it('notes the speed and the slots in group headers', () => {
		const facts = { speed: 2 * 1024 * 1024, running: 2, slots: 3 };
		expect(groupNote('running', facts, 'en')).toBe('2.0 MB/s');
		expect(groupNote('waiting', facts, 'en')).toBe('2 of 3 slots in use');
		expect(groupNote('waiting', { ...facts, slots: null }, 'en')).toBe('');
		expect(groupNote('finished', facts, 'en')).toBe('');
		expect(plain(groupNote('running', facts, 'fr'))).toBe('2,0 Mo/s');
	});

	it('names directions by what they mean', () => {
		expect(directionName('added', 'desc', 'en')).toBe('Newest first');
		expect(directionName('size', 'asc', 'fr')).toBe('Plus petits d’abord');
	});

	it('reads back what was saved, keeping defaults for the rest', () => {
		const saved = { sort: 'name', direction: 'asc', grouped: false };
		expect(parseArrangement(JSON.stringify(saved))).toEqual(saved);
		expect(parseArrangement(null)).toEqual(DEFAULT_ARRANGEMENT);
		expect(parseArrangement('{oops')).toEqual(DEFAULT_ARRANGEMENT);
		expect(parseArrangement('{"sort":"colour","grouped":false}')).toEqual({
			...DEFAULT_ARRANGEMENT,
			grouped: false
		});
	});
});

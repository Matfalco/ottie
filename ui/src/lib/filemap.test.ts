import { describe, expect, it } from 'vitest';
import { barParts, connections, received, strip } from './filemap';
import { job } from './testing';

const split = job({
	state: 'downloading',
	total: 400,
	downloaded: 150,
	// Out of order on purpose: the map sorts them.
	ranges: [
		{ start: 300, end: 400, done: 0 },
		{ start: 0, end: 200, done: 100 },
		{ start: 200, end: 300, done: 50 }
	]
});

describe('file map', () => {
	it('knows what arrived where', () => {
		expect(received(split).map((part) => part.start)).toEqual([0, 200, 300]);
		// One connection: one part from the start.
		const single = job({ state: 'paused', total: 1000, downloaded: 250 });
		expect(received(single)).toEqual([{ start: 0, end: 1000, done: 250 }]);
		expect(received(job({ state: 'completed', total: 10, downloaded: 10 }))).toEqual([
			{ start: 0, end: 10, done: 10 }
		]);
		expect(received(job({ total: null }))).toEqual([]);
	});

	it('splits the bar by connection, sized by share', () => {
		expect(barParts(split)).toEqual([
			{ share: 0.5, fill: 0.5 },
			{ share: 0.25, fill: 0.5 },
			{ share: 0.25, fill: 0 }
		]);
		expect(barParts(job({ state: 'downloading', total: 100, downloaded: 10 }))).toBeNull();
	});

	it('places each range on the bar', () => {
		expect(strip(split, true)).toEqual([
			{ start: 0, end: 0.5, reached: 0.25, writing: true },
			{ start: 0.5, end: 0.75, reached: 0.625, writing: true },
			{ start: 0.75, end: 1, reached: 0.75, writing: true }
		]);
		// Not running: nothing is being written.
		const paused = strip({ ...split, state: 'paused' }, false);
		expect(paused?.map((part) => part.writing)).toEqual([false, false, false]);
		// A finished range isn't being written either.
		const ended = job({ ...split, ranges: [{ start: 0, end: 400, done: 400 }] });
		expect(strip(ended, true)).toEqual([{ start: 0, end: 1, reached: 1, writing: false }]);
		expect(strip(job({ total: null }), true)).toBeNull();
	});

	it('says how many connections', () => {
		expect(connections(split, 'en')).toBe('3 ranges');
		expect(connections(job(), 'fr')).toBe('Une connexion');
	});
});

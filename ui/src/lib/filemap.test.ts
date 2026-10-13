import { describe, expect, it } from 'vitest';
import { barParts, connections, fileMap, received } from './filemap';
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

	it('cuts the file into blocks', () => {
		// 8 blocks of 50 bytes: 0–100 arrived, 200–250 arrived, writing at 100 and 250.
		expect(fileMap(split, 8, true)).toEqual([
			'done',
			'done',
			'arriving',
			'left',
			'done',
			'arriving',
			'arriving',
			'left'
		]);
		// Not running: nothing is arriving; a part-filled block says so.
		const paused = fileMap({ ...split, state: 'paused' }, 4, false);
		expect(paused).toEqual(['done', 'left', 'partial', 'left']);
		expect(fileMap(job({ total: null }), 8, true)).toBeNull();
	});

	it('says how many connections', () => {
		expect(connections(split, 'en')).toBe('3 ranges');
		expect(connections(job(), 'fr')).toBe('Une connexion');
	});
});

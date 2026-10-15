import { describe, expect, it } from 'vitest';
import { addSample, niceTop, paths, points, secondsAt, totalSpeed } from './speed';

const KB = 1024;
const MB = 1024 * 1024;

describe('speed history', () => {
	it('adds up the fresh rates only', () => {
		const rates = {
			a: { at: 1000, downloaded: 0, bytesPerSec: 300 },
			b: { at: 1000, downloaded: 0, bytesPerSec: 200 },
			stale: { at: -9000, downloaded: 0, bytesPerSec: 999 }
		};
		expect(totalSpeed(rates, 2000)).toBe(500);
	});

	it('keeps the last samples', () => {
		expect(addSample([1, 2, 3], 4, 3)).toEqual([2, 3, 4]);
		expect(addSample([], 7)).toEqual([7]);
	});

	it('rounds the scale up to 1, 2 or 5 of the unit the labels show', () => {
		expect(niceTop(0)).toBe(100 * KB);
		expect(niceTop(3.1 * MB)).toBe(5 * MB);
		expect(niceTop(1.5 * MB)).toBe(2 * MB);
		expect(niceTop(2 * MB)).toBe(2 * MB);
		expect(niceTop(640 * KB)).toBe(MB);
	});
});

describe('graph shapes', () => {
	it('puts the newest sample at the right edge, one step per second', () => {
		const at = points([0, 50, 100], 100, 10, 100, 11);
		expect(at).toEqual([
			{ x: 80, y: 10 },
			{ x: 90, y: 5 },
			{ x: 100, y: 0 }
		]);
	});

	it('draws a line and closes the area along the bottom', () => {
		const { line, area } = paths(
			[
				{ x: 0, y: 4 },
				{ x: 10, y: 2 }
			],
			8
		);
		expect(line).toBe('M0.0,4.0L10.0,2.0');
		expect(area).toBe('M0.0,4.0L10.0,2.0L10.0,8L0.0,8Z');
		expect(paths([], 8)).toEqual({ line: '', area: '' });
	});

	it('finds the second under the pointer, now at the end of the line', () => {
		// Left to right: now on the right.
		expect(secondsAt(500, 100, 400, false)).toBe(0);
		expect(secondsAt(100, 100, 400, false)).toBe(299);
		expect(secondsAt(50, 100, 400, false)).toBe(299);
		// Right to left: now on the left.
		expect(secondsAt(100, 100, 400, true)).toBe(0);
		expect(secondsAt(500, 100, 400, true)).toBe(299);
		expect(secondsAt(300, 100, 400, true)).toBe(150);
	});
});

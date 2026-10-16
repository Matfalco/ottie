import { describe, expect, it } from 'vitest';
import {
	DEFAULT_SPAN,
	addSample,
	nextSample,
	niceTop,
	overSpan,
	parseSpan,
	paths,
	points,
	pointsAgo,
	totalSpeed
} from './speed';

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

	it("carries the daemon's last sample on while the page measures its own", () => {
		expect(nextSample([10, 20], 0, 500)).toBe(20);
		expect(nextSample([10, 20], 5, 3000)).toBe(5);
		expect(nextSample([], 7, 500)).toBe(7);
	});

	it('keeps the last samples', () => {
		expect(addSample([1, 2, 3], 4, 3)).toEqual([2, 3, 4]);
		expect(addSample([], 7)).toEqual([7]);
	});

	it('shows the chosen span, averaging seconds when it is long', () => {
		expect(overSpan([1, 2, 3, 4, 5], 3)).toEqual({ samples: [3, 4, 5], step: 1 });
		// Six seconds in two points of three: counted back from now.
		expect(overSpan([9, 1, 2, 3, 4, 5, 6], 6, 2)).toEqual({ samples: [2, 5], step: 3 });
		// Fewer seconds than a whole step: the oldest point averages what there is.
		expect(overSpan([2, 4, 6], 3600, 300)).toEqual({ samples: [4], step: 12 });
		expect(overSpan([], 300)).toEqual({ samples: [], step: 1 });
	});

	it('remembers only the spans it offers', () => {
		expect(parseSpan('1800')).toBe(1800);
		expect(parseSpan('42')).toBe(DEFAULT_SPAN);
		expect(parseSpan(null)).toBe(DEFAULT_SPAN);
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

	it('finds the point under the pointer, now at the end of the line', () => {
		// Left to right: now on the right.
		expect(pointsAgo(500, 100, 400, false)).toBe(0);
		expect(pointsAgo(100, 100, 400, false)).toBe(299);
		expect(pointsAgo(50, 100, 400, false)).toBe(299);
		// Right to left: now on the left.
		expect(pointsAgo(100, 100, 400, true)).toBe(0);
		expect(pointsAgo(500, 100, 400, true)).toBe(299);
		expect(pointsAgo(300, 100, 400, true)).toBe(150);
	});
});

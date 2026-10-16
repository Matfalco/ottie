import { describe, expect, it } from 'vitest';
import { MANY, moves, plan, type Spot } from './motion';

describe('moves', () => {
	it('follows the system unless the device says otherwise', () => {
		expect(moves('system', false)).toBe(true);
		expect(moves('system', true)).toBe(false);
		expect(moves('on', true)).toBe(true);
		expect(moves('off', false)).toBe(false);
	});
});

/** A row of the usual size at `top`. */
const row = (top: number, height = 40): Spot => ({ left: 0, top, width: 600, height });
const spots = (entries: [string, Spot][]) => new Map(entries);

describe('plan', () => {
	it('slides the rows that changed places back from where they were', () => {
		const before = spots([
			['a', row(0)],
			['b', row(40)],
			['c', row(80)]
		]);
		const after = spots([
			['b', row(0)],
			['a', row(40)],
			['c', row(80)]
		]);
		expect(plan(before, after, 800)).toEqual({
			moved: [
				{ key: 'b', dx: 0, dy: 40, sx: 1, sy: 1 },
				{ key: 'a', dx: 0, dy: -40, sx: 1, sy: 1 }
			],
			entered: [],
			fading: [],
			hidden: []
		});
	});

	it('fades the rows that come and go, the others sliding into place', () => {
		const before = spots([
			['a', row(0)],
			['b', row(40)]
		]);
		const after = spots([
			['new', row(0)],
			['b', row(40)]
		]);
		const steps = plan(before, after, 800);
		expect(steps.entered).toEqual(['new']);
		expect(steps.fading).toEqual(['a']);
		expect(steps.moved).toEqual([]);
	});

	it('lets a crowd come and go at once', () => {
		const many = Array.from({ length: MANY + 1 }, (_, i): [string, Spot] => [`${i}`, row(i * 40)]);
		const kept = spots([['kept', row(0)]]);
		const steps = plan(spots([...many, ['kept', row(400)]]), kept, 800);
		expect(steps.fading).toEqual([]);
		expect(steps.hidden).toHaveLength(MANY + 1);
		expect(steps.moved).toEqual([{ key: 'kept', dx: 0, dy: 400, sx: 1, sy: 1 }]);
		expect(plan(kept, spots(many), 800).entered).toEqual([]);
	});

	it("doesn't move rows that stay out of sight", () => {
		const before = spots([['far', row(2000)]]);
		const after = spots([['far', row(2040)]]);
		expect(plan(before, after, 800).moved).toEqual([]);
		// Coming into sight, it slides.
		expect(plan(spots([['far', row(900)]]), spots([['far', row(700)]]), 800).moved).toHaveLength(1);
	});

	it('tells how much to scale back a row that changed size', () => {
		const before = spots([['tab', { left: 100, top: 0, width: 50, height: 30 }]]);
		const after = spots([['tab', { left: 200, top: 0, width: 100, height: 30 }]]);
		expect(plan(before, after, 800).moved).toEqual([
			{ key: 'tab', dx: -100, dy: 0, sx: 0.5, sy: 1 }
		]);
	});
});

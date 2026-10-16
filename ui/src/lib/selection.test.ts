import { describe, expect, it } from 'vitest';
import { NOTHING, click, keepShown, selectAll, step, type Selection } from './selection';

const ORDER = ['a', 'b', 'c', 'd', 'e'];
const PLAIN = { toggle: false, range: false };
const CTRL = { toggle: true, range: false };
const SHIFT = { toggle: false, range: true };
const ids = (selection: Selection) => [...selection.ids].sort();

describe('selection', () => {
	it('selects one on a plain click', () => {
		const one = click(NOTHING, 'b', PLAIN, ORDER);
		expect(ids(click(one, 'd', PLAIN, ORDER))).toEqual(['d']);
	});

	it('adds and takes away with Ctrl', () => {
		let selection = click(NOTHING, 'b', PLAIN, ORDER);
		selection = click(selection, 'd', CTRL, ORDER);
		expect(ids(selection)).toEqual(['b', 'd']);
		expect(ids(click(selection, 'b', CTRL, ORDER))).toEqual(['d']);
	});

	it('selects a range with Shift, either way, from the last click', () => {
		const start = click(NOTHING, 'b', PLAIN, ORDER);
		expect(ids(click(start, 'd', SHIFT, ORDER))).toEqual(['b', 'c', 'd']);
		const back = click(click(start, 'd', SHIFT, ORDER), 'a', SHIFT, ORDER);
		expect(ids(back)).toEqual(['a', 'b']);
		const added = click(click(start, 'e', CTRL, ORDER), 'c', { toggle: true, range: true }, ORDER);
		expect(ids(added)).toEqual(['b', 'c', 'd', 'e']);
	});

	it('treats Shift without an anchor as a plain click', () => {
		expect(ids(click(NOTHING, 'c', SHIFT, ORDER))).toEqual(['c']);
	});

	it('selects all, and forgets what is no longer shown', () => {
		expect(ids(selectAll(ORDER))).toEqual(ORDER);
		const selection = click(click(NOTHING, 'b', PLAIN, ORDER), 'd', CTRL, ORDER);
		const kept = keepShown(selection, ['a', 'b', 'c']);
		expect(ids(kept)).toEqual(['b']);
		expect(kept.anchor).toBe(null);
		expect(keepShown(selection, ORDER)).toBe(selection);
	});

	it('moves with the arrow keys from the last click', () => {
		const start = click(NOTHING, 'b', PLAIN, ORDER);
		expect(ids(step(start, ORDER, 'down', false))).toEqual(['c']);
		expect(ids(step(start, ORDER, 'up', false))).toEqual(['a']);
		expect(ids(step(start, ORDER, 'first', false))).toEqual(['a']);
		expect(ids(step(start, ORDER, 'last', false))).toEqual(['e']);
		// Stops at either end.
		expect(ids(step(step(start, ORDER, 'first', false), ORDER, 'up', false))).toEqual(['a']);
		expect(ids(step(step(start, ORDER, 'last', false), ORDER, 'down', false))).toEqual(['e']);
	});

	it('starts at the top with Down and the bottom with Up', () => {
		expect(ids(step(NOTHING, ORDER, 'down', false))).toEqual(['a']);
		expect(ids(step(NOTHING, ORDER, 'up', false))).toEqual(['e']);
		expect(step(NOTHING, [], 'down', false)).toBe(NOTHING);
	});

	it('extends from the anchor with Shift, either way', () => {
		const start = click(NOTHING, 'c', PLAIN, ORDER);
		const two = step(step(start, ORDER, 'down', true), ORDER, 'down', true);
		expect(ids(two)).toEqual(['c', 'd', 'e']);
		// Back past the anchor: the range turns around it.
		let back = two;
		for (let i = 0; i < 4; i++) {
			back = step(back, ORDER, 'up', true);
		}
		expect(ids(back)).toEqual(['a', 'b', 'c']);
		expect(ids(step(start, ORDER, 'first', true))).toEqual(['a', 'b', 'c']);
		// Then a plain arrow moves on from where the range ended.
		expect(ids(step(back, ORDER, 'down', false))).toEqual(['b']);
	});

	it('moves on after a Shift+click from where it ended', () => {
		const range = click(click(NOTHING, 'b', PLAIN, ORDER), 'd', SHIFT, ORDER);
		expect(ids(step(range, ORDER, 'down', true))).toEqual(['b', 'c', 'd', 'e']);
	});

	it('forgets a cursor no longer shown', () => {
		const selection = click(NOTHING, 'd', PLAIN, ORDER);
		const kept = keepShown(selection, ['a', 'b', 'c']);
		expect(kept.cursor).toBe(null);
		expect(ids(step(kept, ['a', 'b', 'c'], 'down', false))).toEqual(['a']);
	});
});

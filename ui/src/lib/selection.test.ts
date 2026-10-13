import { describe, expect, it } from 'vitest';
import { NOTHING, click, keepShown, selectAll, type Selection } from './selection';

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
});

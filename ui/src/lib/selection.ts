// Which downloads are selected, the way file managers do it: a click selects
// one, Ctrl+click adds or takes away one, Shift+click selects the range from
// the last click (the anchor). The arrow keys do the same from the cursor
// (docs/decisions/0046). Every function returns a new selection.

import type { Step } from './shortcuts';

export interface Selection {
	ids: ReadonlySet<string>;
	/** Where the next Shift+click range starts. */
	anchor: string | null;
	/** Where the arrow keys move from: the last row clicked or reached. */
	cursor: string | null;
}

export const NOTHING: Selection = { ids: new Set(), anchor: null, cursor: null };

/** The keys held during the click. */
export interface Keys {
	/** Ctrl (Cmd on macOS), or a click on the row's checkbox. */
	toggle: boolean;
	/** Shift. */
	range: boolean;
}

/** `order` is the IDs as shown, top to bottom. */
export function click(selection: Selection, id: string, keys: Keys, order: string[]): Selection {
	if (keys.range && selection.anchor !== null) {
		const from = order.indexOf(selection.anchor);
		const to = order.indexOf(id);
		if (from !== -1 && to !== -1) {
			const range = order.slice(Math.min(from, to), Math.max(from, to) + 1);
			// With Ctrl too, the range is added to what was selected.
			const ids = keys.toggle ? new Set([...selection.ids, ...range]) : new Set(range);
			return { ids, anchor: selection.anchor, cursor: id };
		}
	}
	if (keys.toggle) {
		const ids = new Set(selection.ids);
		if (ids.has(id)) {
			ids.delete(id);
		} else {
			ids.add(id);
		}
		return { ids, anchor: id, cursor: id };
	}
	return { ids: new Set([id]), anchor: id, cursor: id };
}

export function selectAll(order: string[]): Selection {
	const first = order[0] ?? null;
	return { ids: new Set(order), anchor: first, cursor: first };
}

/** An arrow key, Home or End: selects the next row (previous, first, last)
 * from the cursor; with `extend` (Shift), the range from the anchor to it.
 * With no cursor, Down starts at the top and Up at the bottom. */
export function step(selection: Selection, order: string[], to: Step, extend: boolean): Selection {
	const last = order.length - 1;
	const from = selection.cursor ?? selection.anchor;
	const at = from === null ? -1 : order.indexOf(from);
	const index = {
		up: at === -1 ? last : Math.max(0, at - 1),
		down: at === -1 ? 0 : Math.min(last, at + 1),
		first: 0,
		last
	}[to];
	const id = order[index];
	if (id === undefined) {
		return selection; // Nothing shown.
	}
	if (!extend) {
		return { ids: new Set([id]), anchor: id, cursor: id };
	}
	const anchor =
		selection.anchor !== null && order.includes(selection.anchor)
			? selection.anchor
			: (order[at] ?? id);
	const start = order.indexOf(anchor);
	const range = order.slice(Math.min(start, index), Math.max(start, index) + 1);
	return { ids: new Set(range), anchor, cursor: id };
}

/** Without the IDs that aren't shown any more (removed, or outside the
 * view). Returns `selection` itself when nothing changes. */
export function keepShown(selection: Selection, order: string[]): Selection {
	const shown = new Set(order);
	const kept = (id: string | null) => (id !== null && shown.has(id) ? id : null);
	if (
		[...selection.ids].every((id) => shown.has(id)) &&
		kept(selection.anchor) === selection.anchor &&
		kept(selection.cursor) === selection.cursor
	) {
		return selection;
	}
	const ids = new Set([...selection.ids].filter((id) => shown.has(id)));
	return { ids, anchor: kept(selection.anchor), cursor: kept(selection.cursor) };
}

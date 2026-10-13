// Which downloads are selected, the way file managers do it: a click selects
// one, Ctrl+click adds or takes away one, Shift+click selects the range from
// the last click (the anchor). Every function returns a new selection.

export interface Selection {
	ids: ReadonlySet<string>;
	/** Where the next Shift+click range starts. */
	anchor: string | null;
}

export const NOTHING: Selection = { ids: new Set(), anchor: null };

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
			return { ids, anchor: selection.anchor };
		}
	}
	if (keys.toggle) {
		const ids = new Set(selection.ids);
		if (ids.has(id)) {
			ids.delete(id);
		} else {
			ids.add(id);
		}
		return { ids, anchor: id };
	}
	return { ids: new Set([id]), anchor: id };
}

export function selectAll(order: string[]): Selection {
	return { ids: new Set(order), anchor: order[0] ?? null };
}

/** Without the IDs that aren't shown any more (removed, or outside the
 * view). Returns `selection` itself when nothing changes. */
export function keepShown(selection: Selection, order: string[]): Selection {
	const shown = new Set(order);
	if ([...selection.ids].every((id) => shown.has(id))) {
		return selection;
	}
	const ids = new Set([...selection.ids].filter((id) => shown.has(id)));
	const anchor = selection.anchor !== null && shown.has(selection.anchor) ? selection.anchor : null;
	return { ids, anchor };
}

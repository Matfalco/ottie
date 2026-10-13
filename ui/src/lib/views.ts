// Saved views (docs/decisions/0026): filters plus a sort order, kept by the
// daemon so every device has them, shown as pills after the quick filters.

import type { Arrangement } from '@ottie/api-types/Arrangement';
import type { Event as ApiEvent } from '@ottie/api-types/Event';
import type { View } from '@ottie/api-types/View';
import type { ViewBody } from '@ottie/api-types/ViewBody';
import { STATUSES, sameFilters, type Filters } from './filters';

export type Views = Record<number, View>;

/** Applies a view event; other events are left to the rest. */
export function applyViewEvent(views: Views, event: ApiEvent): void {
	if (event.type === 'viewChanged') {
		views[event.view.id] = event.view;
	} else if (event.type === 'viewRemoved') {
		delete views[event.id];
	}
}

/** Oldest first, the order they were saved in. */
export function sortedViews(views: Views): View[] {
	return Object.values(views).sort((a, b) => a.id - b.id);
}

/** The view's filters as the Downloads page holds them. "Connecting"
 * counts as downloading, and every status ticked means any status. */
export function viewFilters(view: View): Filters {
	const { states, categories, nameContains, added } = view.filters;
	const picked = states?.map((state) => (state === 'connecting' ? 'downloading' : state));
	const statuses = picked ? STATUSES.filter((status) => picked.includes(status)) : null;
	return {
		statuses: statuses?.length === STATUSES.length ? null : statuses,
		categories: categories ? [...new Set(categories)].sort((a, b) => a - b) : null,
		query: nameContains,
		added
	};
}

/** What to send to save these filters and this arrangement as a view. */
export function viewBody(name: string, filters: Filters, arrangement: Arrangement): ViewBody {
	return {
		name: name.trim(),
		filters: {
			states: filters.statuses,
			categories: filters.categories,
			nameContains: filters.query.trim(),
			added: filters.added
		},
		arrangement
	};
}

export function sameArrangement(a: Arrangement, b: Arrangement): boolean {
	return a.sort === b.sort && a.direction === b.direction && a.grouped === b.grouped;
}

/** Whether the list shows exactly this view: its filters, laid out its way.
 * Typed text counts without its surrounding spaces, as the view saves it. */
export function showsView(view: View, filters: Filters, arrangement: Arrangement): boolean {
	const typed = { ...filters, query: filters.query.trim() };
	return sameFilters(viewFilters(view), typed) && sameArrangement(view.arrangement, arrangement);
}

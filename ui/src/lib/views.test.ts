import { describe, expect, it } from 'vitest';
import type { View } from '@ottie/api-types/View';
import { DEFAULT_ARRANGEMENT } from './arrange';
import { NO_FILTERS } from './filters';
import { applyViewEvent, showsView, sortedViews, viewBody, viewFilters, type Views } from './views';

function view(id: number, filters: Partial<View['filters']> = {}): View {
	return {
		id,
		name: `View ${id}`,
		filters: { states: null, categories: null, nameContains: '', added: 'any', ...filters },
		arrangement: DEFAULT_ARRANGEMENT
	};
}

describe('viewFilters', () => {
	it('reads the daemon’s lists the way the page keeps them', () => {
		const filters = viewFilters(
			view(1, { states: ['failed', 'connecting', 'retrying'], categories: [4, 1, 4] })
		);
		expect(filters.statuses).toEqual(['downloading', 'retrying', 'failed']);
		expect(filters.categories).toEqual([1, 4]);
	});

	it('counts every status as any, and keeps an empty list empty', () => {
		const every = viewFilters(
			view(1, {
				states: ['completed', 'cancelled', 'failed', 'paused', 'queued', 'retrying', 'downloading']
			})
		);
		expect(every.statuses).toBeNull();
		expect(viewFilters(view(1, { states: [] })).statuses).toEqual([]);
	});

	it('round-trips through what is saved', () => {
		const filters = {
			statuses: ['queued' as const],
			categories: [3],
			query: 'iso',
			added: 'week' as const
		};
		const body = viewBody(' ISOs ', filters, DEFAULT_ARRANGEMENT);
		expect(body.name).toBe('ISOs');
		const saved: View = { id: 1, ...body };
		expect(viewFilters(saved)).toEqual(filters);
	});
});

describe('showsView', () => {
	it('needs the same filters and the same arrangement', () => {
		const iso = view(1, { nameContains: 'iso' });
		const filters = { ...NO_FILTERS, query: 'iso ' };
		expect(showsView(iso, filters, DEFAULT_ARRANGEMENT)).toBe(true);
		expect(showsView(iso, filters, { ...DEFAULT_ARRANGEMENT, grouped: false })).toBe(false);
		expect(showsView(iso, NO_FILTERS, DEFAULT_ARRANGEMENT)).toBe(false);
	});
});

describe('applyViewEvent', () => {
	it('adds, replaces and drops views, oldest first', () => {
		const views: Views = {};
		applyViewEvent(views, { type: 'viewChanged', view: view(2) });
		applyViewEvent(views, { type: 'viewChanged', view: view(1) });
		applyViewEvent(views, { type: 'viewChanged', view: { ...view(2), name: 'Renamed' } });
		expect(sortedViews(views).map((each) => each.name)).toEqual(['View 1', 'Renamed']);
		applyViewEvent(views, { type: 'viewRemoved', id: 1 });
		applyViewEvent(views, { type: 'resync' });
		expect(Object.keys(views)).toEqual(['2']);
	});
});

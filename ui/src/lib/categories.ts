// Categories as the daemon describes them, kept up to date by events, and the
// small conversions the Categories page needs.

import type { Category } from '@ottie/api-types/Category';
import type { Event as ApiEvent } from '@ottie/api-types/Event';
import { formatCount, formatSpeed } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

export type Categories = Record<number, Category>;

/** The built-in category: it always exists and can't be removed. */
export const GENERAL = 1;

/** How many tile colours the themes define (`--hue-1` to `--hue-6`). */
const HUES = 6;

/** The colour of a category's tile, as a CSS value: General takes the
 * accent, Other the faint text colour, the others one of the themes'
 * category colours, picked by ID (so it never changes). */
export function categoryHue(category: Category): string {
	if (category.builtIn === 'general') {
		return 'var(--accent)';
	}
	if (category.builtIn === 'other') {
		return 'var(--faint)';
	}
	return `var(--hue-${(category.id % HUES) + 1})`;
}

/** The extensions a row shows, and how many it leaves out. */
export function someExtensions(extensions: string[], room: number): [string[], number] {
	return [extensions.slice(0, room), Math.max(0, extensions.length - room)];
}

/** Applies a category event; other events are left to `jobs.ts`. */
export function applyCategoryEvent(categories: Categories, event: ApiEvent): void {
	if (event.type === 'categoryChanged') {
		categories[event.category.id] = event.category;
	} else if (event.type === 'categoryRemoved') {
		delete categories[event.id];
	}
}

const rank = (category: Category) =>
	category.builtIn === 'general' ? 0 : category.builtIn === 'other' ? 2 : 1;

/** General first, then by name, then Other. */
export function sorted(categories: Categories, locale: Locale = getLocale()): Category[] {
	return Object.values(categories).sort(
		(a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name, locale)
	);
}

/** The name to show. The daemon names its built-in categories "General"
 * and "Other"; they're shown in the page's language, unless renamed. */
export function categoryName(category: Category, locale: Locale = getLocale()): string {
	if (category.builtIn === 'general' && category.name === 'General') {
		return m.cat_general({}, { locale });
	}
	if (category.builtIn === 'other' && category.name === 'Other') {
		return m.cat_other({}, { locale });
	}
	return category.name;
}

/** "mp4, .MKV webm" → ["mp4", ".MKV", "webm"]; the daemon normalizes the rest. */
export function parseExtensions(text: string): string[] {
	return text.split(/[\s,;]+/).filter(Boolean);
}

/** "Up to 2 at once · 1.0 MB/s at most", or what isn't limited. */
export function describeLimits(category: Category, locale: Locale = getLocale()): string {
	const options = { locale };
	const running =
		category.maxConcurrent === 0
			? m.cat_no_cap({}, options)
			: m.cat_cap({ count: formatCount(category.maxConcurrent, locale) }, options);
	const speed =
		category.speedLimit === 0
			? m.cat_no_speed_limit({}, options)
			: m.cat_speed_limit({ speed: formatSpeed(category.speedLimit, locale) }, options);
	return `${running} · ${speed}`;
}

/** A folder for a new category: a subfolder named after it, inside
 * General's folder (`D:\Downloads` → `D:\Downloads\Videos`). */
export function suggestFolder(general: string | null, name: string): string {
	if (!general || !name.trim()) {
		return general ?? '';
	}
	const separator = general.includes('\\') ? '\\' : '/';
	const base = general.endsWith(separator) ? general.slice(0, -1) : general;
	return `${base}${separator}${name.trim()}`;
}

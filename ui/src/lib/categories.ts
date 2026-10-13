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

/** Applies a category event; other events are left to `jobs.ts`. */
export function applyCategoryEvent(categories: Categories, event: ApiEvent): void {
	if (event.type === 'categoryChanged') {
		categories[event.category.id] = event.category;
	} else if (event.type === 'categoryRemoved') {
		delete categories[event.id];
	}
}

/** General first, then by name. */
export function sorted(categories: Categories, locale: Locale = getLocale()): Category[] {
	return Object.values(categories).sort((a, b) =>
		a.id === GENERAL ? -1 : b.id === GENERAL ? 1 : a.name.localeCompare(b.name, locale)
	);
}

/** The name to show. The daemon calls the built-in category "General"; that
 * is shown in the page's language, unless the user renamed it. */
export function categoryName(category: Category, locale: Locale = getLocale()): string {
	return category.id === GENERAL && category.name === 'General'
		? m.cat_general({}, { locale })
		: category.name;
}

/** "mp4, .MKV webm" → ["mp4", ".MKV", "webm"]; the daemon normalizes the rest. */
export function parseExtensions(text: string): string[] {
	return text.split(/[\s,;]+/).filter(Boolean);
}

export type SpeedUnit = 'kB' | 'MB';
const UNIT_BYTES: Record<SpeedUnit, number> = { kB: 1024, MB: 1024 * 1024 };

/** A limit in bytes per second as the editor shows it (0 = none → empty). */
export function limitToField(bytesPerSec: number): { value: string; unit: SpeedUnit } {
	if (bytesPerSec === 0) {
		return { value: '', unit: 'MB' };
	}
	const unit: SpeedUnit = bytesPerSec >= UNIT_BYTES.MB ? 'MB' : 'kB';
	const value = Math.round((bytesPerSec / UNIT_BYTES[unit]) * 100) / 100;
	return { value: String(value), unit };
}

/** Back to bytes per second; empty, zero or nonsense means no limit. */
export function fieldToLimit(value: string, unit: SpeedUnit): number {
	const number = Number(value.replace(',', '.'));
	return Number.isFinite(number) && number > 0 ? Math.round(number * UNIT_BYTES[unit]) : 0;
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

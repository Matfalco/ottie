// The desktop app's window for links copied elsewhere (docs/decisions/0043):
// what it says of each link, how long it stays, and what became of the
// links once added.

import type { AddedJobs } from '@ottie/api-types/AddedJobs';
import type { FoundLink } from '@ottie/api-types/FoundLink';
import { categoryName, type Categories } from './categories';
import { explainRefusal } from './explain';
import { formatCount } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** How long the window stays when left alone, in milliseconds; the time
 * starts again when the pointer leaves it or another link comes. */
export const SHOWN_FOR = 20_000;
/** Links listed by name; the others are counted. */
export const LISTED = 3;

/** A link as the window lists it. */
export interface CaughtRow {
	url: string;
	/** The file name the link suggests. */
	name: string;
	/** The category it goes in (automatic), if known. */
	category: string | null;
	host: string;
}

/** The rows for the links the daemon found; those it can't add are left out
 * (the app never offers them). */
export function caughtRows(
	found: FoundLink[],
	categories: Categories,
	locale: Locale = getLocale()
): CaughtRow[] {
	return found.flatMap((link) => {
		if (link.kind !== 'link') {
			return [];
		}
		const category = categories[link.matched.category];
		return [
			{
				url: link.url,
				name: link.name,
				category: category ? categoryName(category, locale) : null,
				host: hostOf(link.url)
			}
		];
	});
}

function hostOf(url: string): string {
	try {
		return new URL(url).host;
	} catch {
		return '';
	}
}

/** "Copied link", or "Copied links: 3". */
export function caughtTitle(count: number, locale: Locale = getLocale()): string {
	return count === 1
		? m.caught_one({}, { locale })
		: m.caught_several({ count: formatCount(count, locale) }, { locale });
}

/** "2 more", under the links listed; null if none are left out. */
export function othersText(count: number, locale: Locale = getLocale()): string | null {
	const others = count - LISTED;
	return others > 0 ? m.caught_others({ count: formatCount(others, locale) }, { locale }) : null;
}

/** What became of `sent` (in the order sent): those added, and why the
 * others weren't, by link. */
export function outcome(
	sent: string[],
	answer: AddedJobs,
	locale: Locale = getLocale()
): { added: string[]; refused: Record<string, string> } {
	const added = sent.filter((_, index) => answer.ids[index] != null);
	const refused = Object.fromEntries(
		answer.refused.flatMap((refusal) => {
			const url = sent[refusal.index];
			return url === undefined
				? []
				: [[url, explainRefusal(refusal, refusal.error, locale)] as const];
		})
	);
	return { added, refused };
}

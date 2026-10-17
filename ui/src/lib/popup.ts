// The browser extension's popup (docs/decisions/0055): this page, in a
// frame of the extension's own. Which downloads it lists, and what it
// says to the extension and hears from it.

import type { Job } from '@ottie/api-types/Job';
import { GROUPS, groupOf } from './filters';

/** The downloads the popup lists, at most. */
const SHOWN = 6;

/** Running ones first, then those that need attention, the waiting, the
 * finished; each group newest first (as `jobs` comes). Never just one more:
 * it takes the place of the "+1" line. */
export function popupRows(jobs: Job[]): { rows: Job[]; more: number } {
	const ordered = GROUPS.flatMap((group) => jobs.filter((job) => groupOf(job) === group));
	const rows = ordered.length > SHOWN + 1 ? ordered.slice(0, SHOWN) : ordered;
	return { rows, more: ordered.length - rows.length };
}

/** A page of a browser extension, as `MessageEvent.origin` gives it. */
export function isExtensionOrigin(origin: string): boolean {
	return /^(chrome|moz|safari-web)-extension:\/\/[^/]+$/.test(origin);
}

/** What the popup page tells the extension: it's ready for the token, how
 * tall it is, or that the token was refused. */
export type ToExtension =
	| { type: 'ottie-popup-ready' }
	| { type: 'ottie-popup-size'; height: number }
	| { type: 'ottie-popup-refused' };

/** The token in the extension's answer to `ottie-popup-ready`. */
export function tokenIn(data: unknown): string | null {
	if (typeof data !== 'object' || data === null) {
		return null;
	}
	const { type, token } = data as { type?: unknown; token?: unknown };
	return type === 'ottie-token' && typeof token === 'string' && /^[\w.~-]+$/.test(token)
		? token
		: null;
}

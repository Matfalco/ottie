// Which of the browser's downloads go to Ottie (docs/decisions/0055): every
// one, unless the person turned that off, or Ottie couldn't download it the
// same way, or it's one of the exceptions they chose.

import { isExcluded } from './sites';
import type { ExtensionSettings } from './settings';

/** What the browser tells about a download it started (both browsers'
 * `DownloadItem` have these). */
export interface BrowserDownload {
	url: string;
	incognito: boolean;
	state: string;
	/** Bytes, if the server said: 0 (Chrome) or -1 (Firefox) when not. */
	totalBytes: number;
	byExtensionId?: string;
}

/** Why a download stays in the browser. */
export type Skip =
	/** Taking over is turned off. */
	| 'off'
	/** Not signed in to Ottie. */
	| 'signedOut'
	/** A private window: Ottie would keep it in its list. */
	| 'private'
	/** blob:, data:, file:… only the browser can fetch those. */
	| 'scheme'
	/** Already finished or stopped before it got here. */
	| 'notRunning'
	| 'small'
	| 'excluded'
	/** It came from a form's POST. */
	| 'posted'
	/** Ottie's own address (a play link, a page's file). */
	| 'ottie'
	/** Started by an extension, this one or another. */
	| 'extension';

const SCHEMES = new Set(['http:', 'https:', 'ftp:']);

/** null: Ottie takes it. */
export function skipReason(
	item: BrowserDownload,
	settings: ExtensionSettings,
	posted: (url: string) => boolean
): Skip | null {
	if (!settings.takeOver) {
		return 'off';
	}
	if (settings.pairing === null) {
		return 'signedOut';
	}
	if (item.incognito) {
		return 'private';
	}
	let url: URL;
	try {
		url = new URL(item.url);
	} catch {
		return 'scheme';
	}
	if (!SCHEMES.has(url.protocol)) {
		return 'scheme';
	}
	if (item.byExtensionId !== undefined) {
		return 'extension';
	}
	if (item.state !== 'in_progress') {
		return 'notRunning';
	}
	if (url.origin === settings.pairing.address) {
		return 'ottie';
	}
	if (item.totalBytes > 0 && item.totalBytes < settings.minSize) {
		return 'small';
	}
	if (isExcluded(item.url, settings.excluded)) {
		return 'excluded';
	}
	if (posted(item.url)) {
		return 'posted';
	}
	return null;
}

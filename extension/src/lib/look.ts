// The look and language Ottie shares with its devices (docs/decisions/0022,
// 0041), kept by the extension for its own pages, which show before Ottie
// answers or without it.

import type { Appearance } from '@ottie/api-types/Appearance';
import type { Settings } from '@ottie/api-types/Settings';
import { closestLanguage } from '#ui/languages.js';
import { isLocale, locales, type Locale } from '#ui/paraglide/runtime.js';

export interface Look {
	/** Ottie's shared language; null: the browser's. */
	language: string | null;
	/** Ottie's shared look; null: as the system says, sea themes. */
	appearance: Appearance | null;
}

const KEY = 'look';

export async function loadLook(): Promise<Look> {
	const stored = (await chrome.storage.local.get(KEY))[KEY] as Partial<Look> | undefined;
	return { language: stored?.language ?? null, appearance: stored?.appearance ?? null };
}

/** Keeps Ottie's shared look and language, from its settings. */
export async function saveLook(settings: Settings): Promise<void> {
	const look: Look = { language: settings.language, appearance: settings.appearance };
	await chrome.storage.local.set({ [KEY]: look });
}

/** The language a page shows: Ottie's shared one, else the closest to the
 * browser's, else English. */
export function pageLanguage(look: Look, browser: readonly string[]): Locale {
	if (look.language !== null && isLocale(look.language)) {
		return look.language;
	}
	const found = closestLanguage(browser, locales);
	return found !== undefined && isLocale(found) ? found : 'en';
}

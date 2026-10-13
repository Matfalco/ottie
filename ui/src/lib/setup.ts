// The first-start setup: starter categories the user can keep, edit or add
// to. The daemon only creates what the user confirms here.

import type { Category } from '@ottie/api-types/Category';
import type { CategoryBody } from '@ottie/api-types/CategoryBody';
import { suggestFolder } from './categories';
import { EXTENSIONS, type KnownKind } from './filetypes';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

export interface Starter {
	/** Tells the rows apart, even after a rename. */
	key: string;
	body: CategoryBody;
	chosen: boolean;
	/** The folder is a subfolder of the downloads folder, named after the
	 * category, until the user sets another. */
	folderFollows: boolean;
}

const NAMES: Record<KnownKind, (locale: Locale) => string> = {
	video: (locale) => m.setup_videos({}, { locale }),
	audio: (locale) => m.setup_music({}, { locale }),
	image: (locale) => m.setup_images({}, { locale }),
	archive: (locale) => m.setup_archives({}, { locale }),
	document: (locale) => m.setup_documents({}, { locale }),
	program: (locale) => m.setup_programs({}, { locale })
};

/** One per kind of file, all ticked, named in the page's language. */
export function starters(locale: Locale = getLocale()): Starter[] {
	return (Object.keys(NAMES) as KnownKind[]).map((kind) => ({
		key: kind,
		body: {
			name: NAMES[kind](locale),
			folder: null,
			extensions: [...EXTENSIONS[kind]],
			speedLimit: 0,
			maxConcurrent: 0
		},
		chosen: true,
		folderFollows: true
	}));
}

/** Where the starter's downloads will go. */
export function folderOf(starter: Starter, downloads: string): string | null {
	const folder = starter.folderFollows
		? suggestFolder(downloads, starter.body.name)
		: starter.body.folder;
	return folder?.trim() || null;
}

/** The starter as the category editor shows it. */
export function asCategory(starter: Starter, downloads: string): Category {
	return { id: 0, ...starter.body, folder: folderOf(starter, downloads), builtIn: null };
}

/** The starter after the editor saved `body`: its folder keeps following
 * the downloads folder only if the user left it as suggested. */
export function edited(starter: Starter, body: CategoryBody, downloads: string): Starter {
	return {
		...starter,
		body,
		folderFollows: body.folder === suggestFolder(downloads, body.name)
	};
}

/** What to create: the ticked starters, except those whose name is already
 * taken (made by an earlier attempt that stopped halfway). */
export function toCreate(
	list: Starter[],
	downloads: string,
	existing: Category[],
	locale: Locale = getLocale()
): CategoryBody[] {
	const taken = new Set(existing.map((category) => category.name.toLocaleLowerCase(locale)));
	return list
		.filter((starter) => starter.chosen)
		.filter((starter) => !taken.has(starter.body.name.trim().toLocaleLowerCase(locale)))
		.map((starter) => ({ ...starter.body, folder: folderOf(starter, downloads) }));
}

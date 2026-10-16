// The first-start setup (docs/decisions/0020, 0050): its steps, where it
// carries on, and the starter categories the user can keep, edit or add to.
// The daemon only creates what the user confirms here.

import type { Category } from '@ottie/api-types/Category';
import type { CategoryBody } from '@ottie/api-types/CategoryBody';
import type { SetupStep } from '@ottie/api-types/SetupStep';
import { suggestFolder } from './categories';
import { EXTENSIONS, type KnownKind } from './filetypes';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** Every step, in order: the desktop app's own come last. */
export const STEPS: readonly SetupStep[] = [
	'look',
	'downloads',
	'categories',
	'startup',
	'clipboard',
	'notifications'
];

/** Steps only the desktop app has. */
const DESKTOP_STEPS: readonly SetupStep[] = ['startup', 'clipboard', 'notifications'];

/** The steps shown here: the desktop app's only in the desktop app. */
export function stepsHere(inApp: boolean): SetupStep[] {
	return STEPS.filter((step) => inApp || !DESKTOP_STEPS.includes(step));
}

/** Which of `steps` the setup opens on: the one after `passed`, the last
 * step someone went past (in this front end or another), else the first.
 * If every step here is behind it, the last one, so it can still be ended. */
export function startAt(steps: SetupStep[], passed: SetupStep | null): number {
	if (passed === null) {
		return 0;
	}
	const after = STEPS.indexOf(passed);
	const next = steps.findIndex((step) => STEPS.indexOf(step) > after);
	return next === -1 ? steps.length - 1 : next;
}

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

/** The starters whose name no category has yet: those an earlier attempt
 * made, or a setup run before, aren't offered again. */
export function notMade(
	list: Starter[],
	existing: Category[],
	locale: Locale = getLocale()
): Starter[] {
	const taken = new Set(existing.map((category) => category.name.toLocaleLowerCase(locale)));
	return list.filter((starter) => !taken.has(starter.body.name.trim().toLocaleLowerCase(locale)));
}

/** What to create: the ticked starters, except those whose name is already
 * taken (made by an earlier attempt that stopped halfway). */
export function toCreate(
	list: Starter[],
	downloads: string,
	existing: Category[],
	locale: Locale = getLocale()
): CategoryBody[] {
	return notMade(list, existing, locale)
		.filter((starter) => starter.chosen)
		.map((starter) => ({ ...starter.body, folder: folderOf(starter, downloads) }));
}

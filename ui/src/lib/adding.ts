// The step between pasting a link and adding it: which category (the one
// its file type matches, or one the user picks) and which folder.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { Matched } from '@ottie/api-types/Matched';
import { categoryName, type Categories } from './categories';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** Automatic, or a category's ID (the user's choice, which stays). */
export type Choice = 'auto' | number;

/** "Automatic: Videos (.mp4)", or "Music (its folder)" when the folder
 * decided; just "Automatic" until the daemon answers. */
export function automaticLabel(
	matched: Matched | null,
	categories: Categories,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	const found = matched && categories[matched.category];
	if (!found) {
		return m.add_automatic({}, options);
	}
	const category = categoryName(found, locale);
	if (matched.byFolder) {
		return found.builtIn === 'other'
			? m.add_automatic_other({ category }, options)
			: m.add_automatic_folder({ category }, options);
	}
	// An extension no category lists explains nothing: General took it.
	return matched.extension && found.extensions.includes(matched.extension)
		? m.add_automatic_by({ category, extension: matched.extension }, options)
		: m.add_automatic_as({ category }, options);
}

/** The folder the download gets if the user types none, if any. */
export function defaultFolder(
	choice: Choice,
	matched: Matched | null,
	categories: Categories
): string | null {
	const id = choice === 'auto' ? matched?.category : choice;
	return id === undefined ? null : (categories[id]?.folder ?? null);
}

/** The body of `POST /jobs`. Automatic sends no category, so the daemon
 * may still move it once it knows the file's real name. */
export function jobRequest(url: string, choice: Choice, folder: string): AddJob {
	const job: AddJob = { url: url.trim() };
	if (choice !== 'auto') {
		job.category = choice;
	}
	if (folder.trim()) {
		job.destDir = folder.trim();
	}
	return job;
}

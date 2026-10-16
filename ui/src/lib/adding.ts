// The step between pasting a link and adding it: which category (the one
// its file type matches, or one the user picks), which folder, whether it
// comes in streaming mode, and what to do if its file is already there.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { Existing } from '@ottie/api-types/Existing';
import type { Matched } from '@ottie/api-types/Matched';
import { categoryName, type Categories } from './categories';
import { extensionKind, isMedia } from './filetypes';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** Automatic, or a category's ID (the user's choice, which stays). */
export type Choice = 'auto' | number;

/** What the daemon says adding a link would do. */
export interface Looked {
	/** The category it would get. */
	matched: Matched;
	/** The file name the link suggests; the server may give another. */
	name: string;
	/** A file with that name is already in the folder it would go to. */
	exists: boolean;
}

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

/** Whether the download comes in streaming mode if the user leaves it be:
 * videos and music, when the settings say so. The daemon decides the same
 * way, by the link's extension (and later by the file's real name). */
export function streamsByDefault(matched: Matched | null, streamMedia: boolean): boolean {
	return streamMedia && matched?.extension != null && isMedia(extensionKind(matched.extension));
}

/** What a file already there gets when someone is asked: the setting's
 * answer, unless it's to ask (then rename, which keeps both). */
export function existingByDefault(setting: Existing | undefined): 'rename' | 'replace' {
	return setting === 'replace' ? 'replace' : 'rename';
}

/** A start and a stop time, in milliseconds; null for none. */
export interface Times {
	start: number | null;
	stop: number | null;
}

/** The body of `POST /jobs`. Automatic sends no category, so the daemon
 * may still move it once it knows the file's real name; `streaming` null
 * leaves streaming mode to the daemon the same way, and `existing` null
 * what happens to a file already there (the setting's, docs/decisions/0049). */
export function jobRequest(
	url: string,
	choice: Choice,
	folder: string,
	streaming: boolean | null = null,
	times: Times = { start: null, stop: null },
	existing: Existing | null = null
): AddJob {
	const job: AddJob = { url: url.trim() };
	if (choice !== 'auto') {
		job.category = choice;
	}
	if (folder.trim()) {
		job.destDir = folder.trim();
	}
	if (streaming !== null) {
		job.streaming = streaming;
	}
	if (times.start !== null) {
		job.startAt = times.start;
	}
	if (times.stop !== null) {
		job.stopAt = times.stop;
	}
	if (existing !== null) {
		job.existing = existing;
	}
	return job;
}

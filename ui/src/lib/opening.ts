// Opening a download's file or its folder, in the desktop app
// (docs/decisions/0045): which download a double-click or Enter opens, and
// what to say when one couldn't open.

import type { Job } from '@ottie/api-types/Job';
import { displayName } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import type { OpenRefusal } from './window';

/** A finished file, there to open. */
export function canOpen(job: Job): boolean {
	return job.state === 'completed' && job.fileName !== null;
}

/** What Enter opens: the one download selected, if it's finished. */
export function toOpen(selected: Job[]): Job | null {
	const [only] = selected;
	return selected.length === 1 && only && canOpen(only) ? only : null;
}

/** Why the file (or with `folder`, its folder) didn't open. */
export function openProblem(
	why: OpenRefusal,
	job: Job,
	folder: boolean,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	const name = displayName(job);
	if (why === 'failed') {
		return m.open_failed({ name }, options);
	}
	return folder ? m.folder_gone({ name }, options) : m.open_gone({ name }, options);
}

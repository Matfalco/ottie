// Adding several links at once (docs/decisions/0036): the links the daemon
// found in some text, which of them to add and in which category, what to
// do with the files already there (0049), then what became of each.

import type { AddJob } from '@ottie/api-types/AddJob';
import type { AddedJobs } from '@ottie/api-types/AddedJobs';
import type { FoundLink } from '@ottie/api-types/FoundLink';
import { jobRequest, type Choice } from './adding';
import { explainRefusal } from './explain';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** What a link whose file is already in the folder gets: a new name, the
 * old file replaced once it's complete, or not added. */
export type OnDisk = 'rename' | 'replace' | 'skip';

/** One link of the list. */
export interface Row {
	found: FoundLink;
	/** Ticked: added with the others. */
	chosen: boolean;
	/** Automatic, or the category picked for it. */
	choice: Choice;
	/** If its file is already there (`found.exists`). */
	onDisk: OnDisk;
	/** Why the daemon refused it when adding (in the page's language). */
	refused: string | null;
}

/** Rows shown at first, and added each time "Show more" is clicked: a list
 * of thousands would take a while to draw. */
export const SHOWN_AT_ONCE = 300;

/** A sequence in a link, as the daemon reads them (docs/decisions/0037):
 * `[001-120]`, `[a-z]`, `[0-100:10]`, `{small,large}`, not after `\`. */
const SEQUENCE = /(?<!\\)(\[(\d+-\d+|[a-z]-[a-z]|[A-Z]-[A-Z])(:\d+)?\]|\{[^{}[\]]*,[^{}[\]]*\})/;

/** Whether text holds more than one link, or a link with a sequence, so it
 * opens the list rather than going in the link field. */
export function holdsSeveral(text: string): boolean {
	const links = text.split(/\s+/).filter((word) => word.includes('://'));
	return links.length > 1 || links.some((link) => SEQUENCE.test(link));
}

/** Rows for the links found. A link already listed keeps how it was set;
 * a new one gets `choice` and `onDisk`, and is ticked unless it can't be
 * added or is already on the download list. */
export function rowsOf(
	links: FoundLink[],
	previous: Row[],
	choice: Choice,
	onDisk: OnDisk = 'rename'
): Row[] {
	const before = new Map(previous.map((row) => [key(row.found), row]));
	return links.map((found) => {
		const kept = before.get(key(found));
		if (kept) {
			return { ...kept, found };
		}
		const addable = found.kind === 'link' && !found.known;
		return { found, chosen: addable, choice, onDisk, refused: null };
	});
}

function key(found: FoundLink): string {
	return found.kind === 'link' ? found.url : `invalid ${found.text}`;
}

/** The rows ticked. */
export function chosenRows(rows: Row[]): Row[] {
	return rows.filter((row) => row.chosen && row.found.kind === 'link');
}

/** The rows whose file is already in the folder they'd go to. */
export function onDiskRows(rows: Row[]): Row[] {
	return rows.filter((row) => row.found.kind === 'link' && row.found.exists);
}

/** The rows that would be added: ticked, and not skipped for a file
 * already there. */
export function sentRows(rows: Row[]): Row[] {
	return chosenRows(rows).filter((row) => !(isOnDisk(row) && row.onDisk === 'skip'));
}

function isOnDisk(row: Row): boolean {
	return row.found.kind === 'link' && row.found.exists;
}

/** Each row's request for `POST /jobs/batch`, in order. A file already
 * there gets what was chosen for it, which then stays whatever the setting
 * says; the others leave it to the setting, for the name the server gives. */
export function requests(sent: Row[], folder: string): AddJob[] {
	return sent.flatMap((row) => {
		if (row.found.kind !== 'link') {
			return [];
		}
		const existing = isOnDisk(row) && row.onDisk !== 'skip' ? row.onDisk : null;
		return [jobRequest(row.found.url, row.choice, folder, null, undefined, existing)];
	});
}

/** Every row set to `onDisk` (for all the files already there). */
export function onDiskForAll(rows: Row[], onDisk: OnDisk): Row[] {
	return rows.map((row) => ({ ...row, onDisk }));
}

/** A category menu's value as a choice ("auto", or a category's ID). */
export function choiceOf(value: string): Choice {
	return /^\d+$/.test(value) ? Number(value) : 'auto';
}

/** Every row set to `choice` (the category for all). */
export function chooseForAll(rows: Row[], choice: Choice): Row[] {
	return rows.map((row) => ({ ...row, choice }));
}

/** Ticks every row that can be added, or none. */
export function tickAll(rows: Row[], on: boolean): Row[] {
	return rows.map((row) => ({ ...row, chosen: on && row.found.kind === 'link' }));
}

/** The list once `sent` (the chosen rows, in order) were added: without
 * those added; those refused say why, unticked. */
export function afterAdding(
	rows: Row[],
	sent: Row[],
	answer: AddedJobs,
	locale: Locale = getLocale()
): Row[] {
	const added = new Set(
		sent.filter((_, index) => answer.ids[index] != null).map((row) => key(row.found))
	);
	const refused = new Map(
		answer.refused.flatMap((refusal) => {
			const row = sent[refusal.index];
			const why = explainRefusal(refusal, refusal.error, locale);
			return row ? [[key(row.found), why] as const] : [];
		})
	);
	return rows
		.filter((row) => !added.has(key(row.found)))
		.map((row) => {
			const why = refused.get(key(row.found));
			return why === undefined ? row : { ...row, chosen: false, refused: why };
		});
}

/** The rows' links, one per line: the text once some were added, so it
 * still matches the list. */
export function textOf(rows: Row[]): string {
	return rows.map((row) => (row.found.kind === 'link' ? row.found.url : row.found.text)).join('\n');
}

/** What's wrong with a row, if anything: refused when adding, can't be
 * added, or already on the list. */
export function rowNote(row: Row, locale: Locale = getLocale()): string | null {
	if (row.refused !== null) {
		return row.refused;
	}
	if (row.found.kind === 'invalid') {
		return explainRefusal(row.found.refused, row.found.refused.error, locale);
	}
	return row.found.known ? m.batch_known({}, { locale }) : null;
}

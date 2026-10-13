// Where in the file the bytes have arrived: a row's progress bar split by
// connection, and the detail panel's bar, which maps the file.

import type { ByteRange } from '@ottie/api-types/ByteRange';
import type { Job } from '@ottie/api-types/Job';
import { formatCount } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** The parts of the file and how much of each has arrived, in file order.
 * One connection is one part from the start; empty while the size is
 * unknown. */
export function received(job: Job): ByteRange[] {
	if (job.total === null) {
		return job.state === 'completed'
			? [{ start: 0, end: job.downloaded, done: job.downloaded }]
			: [];
	}
	if (job.state === 'completed') {
		return [{ start: 0, end: job.total, done: job.total }];
	}
	if (job.ranges.length > 0) {
		return [...job.ranges].sort((a, b) => a.start - b.start);
	}
	return [{ start: 0, end: job.total, done: Math.min(job.downloaded, job.total) }];
}

/** A row's bar, one piece per connection: its share of the file, and how
 * full it is (0 to 1). Null for one connection: the bar stays whole. */
export function barParts(job: Job): { share: number; fill: number }[] | null {
	const parts = received(job);
	const total = parts.reduce((sum, part) => sum + (part.end - part.start), 0);
	if (parts.length < 2 || total === 0) {
		return null;
	}
	return parts.map((part) => {
		const length = part.end - part.start;
		return { share: length / total, fill: length === 0 ? 1 : part.done / length };
	});
}

/** A range on the detail panel's bar, as fractions of the file (0 to 1):
 * where it starts and ends, and how far it has arrived. */
export interface StripPart {
	start: number;
	end: number;
	reached: number;
	/** A connection is writing at `reached` right now. */
	writing: boolean;
}

/** The detail panel's bar: each range at its exact place in the file (one
 * from the start for one connection). Unfinished ranges are being written
 * only while `running`. Null while the size is unknown. */
export function strip(job: Job, running: boolean): StripPart[] | null {
	const parts = received(job);
	const size = parts.at(-1)?.end ?? 0;
	if (parts.length === 0 || size === 0) {
		return null;
	}
	return parts.map((part) => ({
		start: part.start / size,
		end: part.end / size,
		reached: (part.start + part.done) / size,
		writing: running && part.start + part.done < part.end
	}));
}

/** "4 ranges" or "One connection", above the file map. */
export function connections(job: Job, locale: Locale = getLocale()): string {
	return job.ranges.length > 1
		? m.det_ranges({ count: formatCount(job.ranges.length, locale) }, { locale })
		: m.det_one_connection({}, { locale });
}

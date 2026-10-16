// Where in the file the bytes have arrived: the bar that maps the file, in
// the details panel and, smaller, in each row (docs/decisions/0027, 0044).

import type { ByteRange } from '@ottie/api-types/ByteRange';
import type { Job } from '@ottie/api-types/Job';
import { formatCount } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** The parts of the file and how much of each has arrived, in file order.
 * One connection is one part from the start; empty while the size is
 * unknown. Neighbouring parts that have both fully arrived are one:
 * streaming mode fetches the file in many small pieces, which would
 * otherwise show as a dashed bar. Empty ranges are left out (older daemons
 * can send one where another range starts). */
export function received(job: Job): ByteRange[] {
	if (job.total === null) {
		return job.state === 'completed'
			? [{ start: 0, end: job.downloaded, done: job.downloaded }]
			: [];
	}
	if (job.state === 'completed') {
		return [{ start: 0, end: job.total, done: job.total }];
	}
	const ranges = job.ranges.filter((range) => range.end > range.start);
	if (ranges.length > 0) {
		return merged(ranges.sort((a, b) => a.start - b.start));
	}
	return [{ start: 0, end: job.total, done: Math.min(job.downloaded, job.total) }];
}

/** `ranges` (in file order), with each run of neighbouring finished ones as one. */
function merged(ranges: ByteRange[]): ByteRange[] {
	const full = (range: ByteRange) => range.done >= range.end - range.start;
	const parts: ByteRange[] = [];
	for (const range of ranges) {
		const last = parts.at(-1);
		if (last !== undefined && full(last) && full(range) && last.end === range.start) {
			parts[parts.length - 1] = { start: last.start, end: range.end, done: last.done + range.done };
		} else {
			parts.push(range);
		}
	}
	return parts;
}

/** A range on the bar, as fractions of the file (0 to 1):
 * where it starts and ends, and how far it has arrived. */
export interface StripPart {
	start: number;
	end: number;
	reached: number;
	/** A connection is writing at `reached` right now. */
	writing: boolean;
}

/** The bar: each range at its exact place in the file (one
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

/** "4 ranges" or "One connection", above the file map: the parts it shows. */
export function connections(job: Job, locale: Locale = getLocale()): string {
	const count = job.ranges.length > 1 ? received(job).length : 1;
	return count > 1
		? m.det_ranges({ count: formatCount(count, locale) }, { locale })
		: m.det_one_connection({}, { locale });
}

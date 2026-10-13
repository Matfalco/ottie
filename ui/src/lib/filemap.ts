// Where in the file the bytes have arrived: the progress bar split by
// connection, and the file map in the detail panel.

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

/** A block of the file map: all there, being written now, some there, none. */
export type Block = 'done' | 'arriving' | 'partial' | 'left';

/** The file cut into `count` equal blocks. Where a connection is writing
 * right now (only while `running`) is "arriving". Null while the size is
 * unknown. */
export function fileMap(job: Job, count: number, running: boolean): Block[] | null {
	const parts = received(job);
	const size = parts.at(-1)?.end ?? 0;
	if (parts.length === 0 || size === 0) {
		return null;
	}
	// Where each unfinished part goes on writing.
	const fronts = parts
		.filter((part) => part.start + part.done < part.end)
		.map((part) => part.start + part.done);
	return Array.from({ length: count }, (_, index) => {
		const from = (size * index) / count;
		const to = (size * (index + 1)) / count;
		const covered = parts.reduce((sum, part) => {
			const overlap = Math.min(to, part.start + part.done) - Math.max(from, part.start);
			return sum + Math.max(0, overlap);
		}, 0);
		if (covered >= to - from) {
			return 'done';
		}
		if (running && fronts.some((front) => front >= from && front < to)) {
			return 'arriving';
		}
		return covered > 0 ? 'partial' : 'left';
	});
}

/** "4 ranges" or "One connection", above the file map. */
export function connections(job: Job, locale: Locale = getLocale()): string {
	return job.ranges.length > 1
		? m.det_ranges({ count: formatCount(job.ranges.length, locale) }, { locale })
		: m.det_one_connection({}, { locale });
}

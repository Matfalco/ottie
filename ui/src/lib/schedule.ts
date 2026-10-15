// A download's own start and stop time (docs/decisions/0040): between the
// daemon's milliseconds and the page's date-and-time fields, and how they read.

import type { Job } from '@ottie/api-types/Job';
import type { JobChange } from '@ottie/api-types/JobChange';
import { m } from './paraglide/messages.js';
import { intlLocale } from './languages';
import { getLocale, type Locale } from './paraglide/runtime.js';

/** What a `datetime-local` field holds for `ms`, in the page's time zone
 * ("2026-10-14T23:00"); empty for none. */
export function toField(ms: number | null): string {
	if (ms === null) {
		return '';
	}
	const at = new Date(ms);
	const two = (n: number) => String(n).padStart(2, '0');
	const date = `${String(at.getFullYear()).padStart(4, '0')}-${two(at.getMonth() + 1)}-${two(at.getDate())}`;
	return `${date}T${two(at.getHours())}:${two(at.getMinutes())}`;
}

/** The time a `datetime-local` field holds, in milliseconds; null if it's
 * empty or not a whole date and time. */
export function fromField(value: string): number | null {
	// Without an offset, a date and time is read in the page's time zone.
	if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(value)) {
		return null;
	}
	const ms = new Date(value).getTime();
	return Number.isNaN(ms) ? null : ms;
}

/** Whether the stop time comes after the start time, as the daemon wants
 * (either may be missing). */
export function inOrder(start: number | null, stop: number | null): boolean {
	return start === null || stop === null || stop > start;
}

/** Whether a download can still take a start or stop time: it isn't over. */
export function canSchedule(job: Job): boolean {
	return job.state !== 'completed' && job.state !== 'cancelled' && !job.moving;
}

/** "today at 23:00", "tomorrow at 01:00", "Thu, Oct 16 at 08:00", with the
 * year when it isn't this one. `now` is `Date.now()`. */
export function formatWhen(ms: number, now: number, locale: Locale = getLocale()): string {
	const options = { locale };
	const at = new Date(ms);
	const time = new Intl.DateTimeFormat(intlLocale(locale), { timeStyle: 'short' }).format(at);
	const today = new Date(now);
	const days = dayNumber(at) - dayNumber(today);
	if (days === 0) {
		return m.when_today({ time }, options);
	}
	if (days === 1) {
		return m.when_tomorrow({ time }, options);
	}
	const date = new Intl.DateTimeFormat(intlLocale(locale), {
		weekday: 'short',
		day: 'numeric',
		month: 'short',
		year: at.getFullYear() === today.getFullYear() ? undefined : 'numeric'
	}).format(at);
	return m.when_date({ date, time }, options);
}

/** The same, short enough for a narrow column: "11:00 PM" today, else
 * "10/15/26, 1:15 AM". */
export function formatWhenShort(ms: number, now: number, locale: Locale = getLocale()): string {
	const at = new Date(ms);
	const today = dayNumber(at) === dayNumber(new Date(now));
	const style = today
		? ({ timeStyle: 'short' } as const)
		: ({ dateStyle: 'short', timeStyle: 'short' } as const);
	return new Intl.DateTimeFormat(intlLocale(locale), style).format(at);
}

/** Days since 1970 by the page's calendar, so "tomorrow" ignores the hour
 * (and the clocks changing). */
function dayNumber(at: Date): number {
	return Math.round(Date.UTC(at.getFullYear(), at.getMonth(), at.getDate()) / 86_400_000);
}

/** Which of a download's two times. */
export type TimeKey = 'startAt' | 'stopAt';

/** The `PATCH /jobs/{id}` body that sets one time (null: takes it away). */
export function timeChange(key: TimeKey, ms: number | null): JobChange {
	return key === 'startAt' ? { startAt: ms } : { stopAt: ms };
}

import { describe, expect, it } from 'vitest';
import {
	canSchedule,
	formatWhen,
	formatWhenShort,
	fromField,
	inOrder,
	timeChange,
	toField
} from './schedule';
import { job, plain } from './testing';

// Built from the page's calendar, so the tests pass in any time zone.
const at = (month: number, day: number, hours: number, minutes = 0) =>
	new Date(2026, month - 1, day, hours, minutes).getTime();

describe('start and stop times', () => {
	it('go to and from date-and-time fields in the page’s time zone', () => {
		expect(toField(at(10, 14, 23, 5))).toBe('2026-10-14T23:05');
		expect(toField(at(1, 2, 3))).toBe('2026-01-02T03:00');
		expect(toField(null)).toBe('');
		expect(fromField('2026-10-14T23:05')).toBe(at(10, 14, 23, 5));
		// Seconds are dropped on the way out, kept on the way in.
		expect(toField(at(10, 14, 23, 5) + 42_000)).toBe('2026-10-14T23:05');
		expect(fromField('2026-10-14T23:05:30')).toBe(at(10, 14, 23, 5) + 30_000);
	});

	it('read an empty or half-typed field as no time', () => {
		expect(fromField('')).toBeNull();
		expect(fromField('2026-10-14')).toBeNull();
		expect(fromField('2026-13-45T99:99')).toBeNull();
	});

	it('want the stop time after the start time', () => {
		expect(inOrder(at(10, 14, 23), at(10, 15, 7))).toBe(true);
		expect(inOrder(at(10, 14, 23), at(10, 14, 23))).toBe(false);
		expect(inOrder(at(10, 14, 23), at(10, 14, 22))).toBe(false);
		expect(inOrder(null, at(10, 14, 22))).toBe(true);
		expect(inOrder(at(10, 14, 23), null)).toBe(true);
	});

	it('say today, tomorrow or the date', () => {
		const now = at(10, 14, 9, 30);
		expect(formatWhen(at(10, 14, 23), now, 'en')).toBe('today at 11:00 PM');
		expect(formatWhen(at(10, 14, 23), now, 'fr')).toBe('aujourd’hui à 23:00');
		expect(formatWhen(at(10, 15, 1, 15), now, 'en')).toBe('tomorrow at 1:15 AM');
		expect(formatWhen(at(10, 15, 1, 15), now, 'fr')).toBe('demain à 01:15');
		expect(plain(formatWhen(at(10, 16, 8), now, 'en'))).toBe('Fri, Oct 16 at 8:00 AM');
		expect(formatWhen(at(10, 16, 8), now, 'fr')).toBe('le ven. 16 oct. à 08:00');
		// Another year says which.
		const nextYear = new Date(2027, 0, 5, 8).getTime();
		expect(plain(formatWhen(nextYear, now, 'en'))).toBe('Tue, Jan 5, 2027 at 8:00 AM');
	});

	it('say just the time when it’s today, in a narrow column', () => {
		const now = at(10, 14, 9, 30);
		expect(formatWhenShort(at(10, 14, 23), now, 'en')).toBe('11:00 PM');
		expect(formatWhenShort(at(10, 14, 23), now, 'fr')).toBe('23:00');
		expect(plain(formatWhenShort(at(10, 15, 1, 15), now, 'en'))).toBe('10/15/26, 1:15 AM');
		expect(formatWhenShort(at(10, 15, 1, 15), now, 'fr')).toBe('15/10/2026 01:15');
	});

	it('are offered until the download is over', () => {
		expect(canSchedule(job({ state: 'downloading' }))).toBe(true);
		expect(canSchedule(job({ state: 'failed' }))).toBe(true);
		expect(canSchedule(job({ state: 'completed' }))).toBe(false);
		expect(canSchedule(job({ state: 'cancelled' }))).toBe(false);
		expect(canSchedule(job({ state: 'paused', moving: true }))).toBe(false);
	});

	it('change one time at once, null taking it away', () => {
		expect(timeChange('startAt', 5)).toEqual({ startAt: 5 });
		expect(timeChange('stopAt', null)).toEqual({ stopAt: null });
	});
});

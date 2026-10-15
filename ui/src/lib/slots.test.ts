import { describe, expect, it } from 'vitest';
import type { Slot } from '@ottie/api-types/Slot';
import {
	DAYS,
	applySlotEvent,
	bodyFromSlot,
	bodyOf,
	conflicts,
	dayName,
	describeDays,
	describeEffects,
	describeHours,
	describeTarget,
	draftOf,
	mondayOf,
	moved,
	queueNow,
	shortLabel,
	slotLabel,
	sortedSlots,
	weekBlocks,
	weekLayout,
	dragged,
	lengthOf,
	meeting,
	overlaps,
	slotOfBody,
	type Slots
} from './slots';
import { category, plain } from './testing';

/** Every night 23:00–07:00, stopping downloads; override what a test needs. */
function slot(overrides: Partial<Slot> = {}): Slot {
	return {
		id: 1,
		place: 0,
		name: 'Night',
		enabled: true,
		category: null,
		days: [...DAYS],
		date: null,
		from: '23:00',
		to: '07:00',
		downloads: 'stop',
		letFinish: false,
		stayPaused: false,
		speedLimit: null,
		maxConcurrent: null,
		active: false,
		...overrides
	};
}

const MB = 1024 * 1024;
// A Wednesday.
const MONDAY = mondayOf(new Date(2026, 9, 14, 15, 0));

describe('time slots', () => {
	it('keep the order of the list, and follow the events', () => {
		const slots: Slots = {};
		applySlotEvent(slots, { type: 'slotChanged', slot: slot({ id: 1, place: 3 }) });
		applySlotEvent(slots, { type: 'slotChanged', slot: slot({ id: 2, place: 1 }) });
		expect(sortedSlots(slots).map((each) => each.id)).toEqual([2, 1]);
		applySlotEvent(slots, { type: 'slotRemoved', id: 2 });
		expect(Object.keys(slots)).toEqual(['1']);
	});

	it('name their days, with ranges for three in a row', () => {
		expect(describeDays(slot(), 'en')).toBe('Every day');
		expect(describeDays(slot(), 'fr')).toBe('Tous les jours');
		const work = slot({ days: ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'] });
		expect(describeDays(work, 'en')).toBe('Mon–Fri');
		expect(describeDays(slot({ days: ['saturday', 'sunday'] }), 'en')).toBe('Sat, Sun');
		const mixed = slot({ days: ['monday', 'wednesday', 'thursday', 'friday'] });
		expect(describeDays(mixed, 'fr')).toBe('lun., mer.–ven.');
		expect(describeDays(slot({ date: '2026-10-15', days: [] }), 'en')).toBe('Thu, Oct 15');
		expect(dayName('sunday', 'long', 'fr')).toBe('dimanche');
	});

	it('say their hours, and when they run into the next day', () => {
		expect(plain(describeHours(slot(), 'en'))).toBe('11:00 PM – 7:00 AM (next day)');
		expect(describeHours(slot(), 'fr')).toBe('23:00 – 07:00 (le lendemain)');
		expect(describeHours(slot({ from: '09:00', to: '18:00' }), 'fr')).toBe('09:00 – 18:00');
		expect(describeHours(slot({ from: '00:00', to: '00:00' }), 'en')).toBe('All day');
	});

	it('say what they apply to and what they do', () => {
		const categories = { 4: category({ id: 4, name: 'Videos' }) };
		expect(describeTarget(slot(), categories, 'en')).toBe('All downloads');
		expect(describeTarget(slot({ category: 4 }), categories, 'en')).toBe('Videos');
		expect(describeEffects(slot(), 'en')).toEqual(['Stops downloads']);
		const busy = slot({ downloads: null, speedLimit: 2 * MB, maxConcurrent: 2 });
		expect(describeEffects(busy, 'en').map(plain)).toEqual(['2 MB/s at most', '2 at once']);
		expect(describeEffects(slot({ letFinish: true, stayPaused: true }), 'fr')).toEqual([
			'Arrête les nouveaux téléchargements',
			'Puis restent en pause'
		]);
		expect(describeEffects(slot({ downloads: 'allow', speedLimit: 0 }), 'fr')).toEqual([
			'Autorise les téléchargements',
			'Pas de limite de vitesse'
		]);
		expect(shortLabel(slot({ name: '', from: '09:00', to: '18:00' }), 'fr')).toBe('09:00 – 18:00');
		// Without a name, when it is.
		expect(slotLabel(slot({ name: ' ', from: '09:00', to: '18:00' }), 'fr')).toBe(
			'Tous les jours · 09:00 – 18:00'
		);
	});

	it('sit in the week, a night running into the next day', () => {
		const night = slot({ days: ['sunday'] });
		expect(weekBlocks(night, MONDAY)).toEqual([
			{ slot: 1, day: 6, start: 23 * 60, end: 24 * 60, first: true, last: false },
			{ slot: 1, day: 0, start: 0, end: 7 * 60, first: false, last: true }
		]);
		// Once: only in its week, and not into the next one.
		const once = slot({ days: [], date: '2026-10-18' });
		expect(weekBlocks(once, MONDAY)).toEqual([
			{ slot: 1, day: 6, start: 23 * 60, end: 24 * 60, first: true, last: false }
		]);
		const evening = slot({ from: '18:00', to: '00:00', days: ['monday'] });
		expect(weekBlocks(evening, MONDAY)).toEqual([
			{ slot: 1, day: 0, start: 18 * 60, end: 24 * 60, first: true, last: true }
		]);
		expect(weekBlocks(slot({ days: [], date: '2026-10-25' }), MONDAY)).toEqual([]);
		expect(mondayOf(new Date(2026, 9, 18)).getDate()).toBe(12);
	});

	it('move and stretch when dragged, by quarters of an hour', () => {
		const night = slot({ days: ['monday', 'sunday'] });
		const when = (each: Slot) => [each.from, each.to, each.days, each.date];
		expect(when(dragged(night, 'move', -62, 0))).toEqual(['22:00', '06:00', night.days, null]);
		// A day to the right: Sunday's night goes to Monday's.
		expect(when(dragged(night, 'move', -7, 1))).toEqual([
			'23:00',
			'07:00',
			['monday', 'tuesday'],
			null
		]);
		expect(dragged(night, 'move', 0, -1).days).toEqual(['saturday', 'sunday']);
		// Its start stays on its day.
		expect(dragged(night, 'move', -30 * 60, 0).from).toBe('00:00');
		expect(dragged(night, 'move', 30 * 60, 0).from).toBe('23:45');
		const once = slot({ days: [], date: '2026-10-31' });
		expect(dragged(once, 'move', 0, 2).date).toBe('2026-11-02');
		// Only the start or the end; days stay put.
		expect(when(dragged(night, 'start', -120, 3))).toEqual(['21:00', '07:00', night.days, null]);
		expect(dragged(night, 'end', 90, 0).to).toBe('08:30');
		// From a quarter of an hour to a whole day.
		expect(dragged(night, 'end', -24 * 60, 0).to).toBe('23:15');
		expect(dragged(night, 'start', 24 * 60, 0).from).toBe('23:45');
		const work = slot({ from: '09:00', to: '18:00' });
		expect(dragged(work, 'start', 24 * 60, 0).from).toBe('17:45');
		const whole = dragged(night, 'end', 24 * 60, 0);
		expect([whole.from, whole.to, lengthOf(whole)]).toEqual(['23:00', '23:00', 24 * 60]);
		expect(dragged(work, 'start', -12 * 60, 0).from).toBe('00:00');
	});

	it('share the width only where blocks overlap', () => {
		const night = slot({ id: 1, days: ['monday'] });
		const work = slot({ id: 2, days: ['monday', 'tuesday'], from: '09:00', to: '18:00' });
		const off = slot({ id: 3, enabled: false });
		const lanes = (list: Slot[]) =>
			weekLayout(list, MONDAY)
				.filter((block) => block.day === 0)
				.map(({ slot, lane, lanes }) => [slot, lane, lanes]);
		// The night and the work hours never meet: each takes the whole width.
		expect(lanes([night, work, off])).toEqual([
			[1, 0, 1],
			[2, 0, 1]
		]);
		// The evening meets both: the three share it, the evening on the right.
		const evening = slot({ id: 4, days: ['monday'], from: '17:00', to: '23:30' });
		expect(lanes([night, work, evening])).toEqual([
			[1, 0, 2],
			[2, 0, 2],
			[4, 1, 2]
		]);
	});

	it('find the slots that overlap, whatever they do', () => {
		const night = slot({ id: 1 });
		const late = slot({ id: 2, from: '06:00', to: '09:00' });
		const morning = slot({ id: 3, from: '07:00', to: '12:00' });
		const videos = slot({ id: 4, category: 4, from: '08:00', to: '10:00' });
		const music = slot({ id: 5, category: 5, from: '08:00', to: '10:00' });
		const off = slot({ id: 6, enabled: false });
		// The night ends at 07:00: it meets the late one, not the morning.
		expect(overlaps([night, late, morning, videos, music, off])).toEqual({
			1: [2],
			2: [1, 3, 4, 5],
			3: [2, 4, 5],
			4: [2, 3],
			5: [2, 3]
		});
		// The one being edited, before it's saved.
		const draft = slotOfBody({ from: '05:00', to: '06:30', days: ['monday'] }, null);
		expect(meeting(draft, [night, late, morning]).map((each) => each.id)).toEqual([1, 2]);
		expect(meeting({ ...draft, enabled: false }, [night])).toEqual([]);
		// Itself doesn't count.
		expect(meeting(slotOfBody(bodyFromSlot(night), 1), [night])).toEqual([]);
	});

	it('find the lower slots that higher ones override', () => {
		const stop = slot({ id: 1 });
		const allow = slot({ id: 2, downloads: 'allow', from: '06:00', to: '08:00' });
		const later = slot({ id: 3, downloads: 'allow', from: '08:00', to: '09:00' });
		const videos = slot({ id: 4, category: 4, downloads: 'allow' });
		const limit = slot({ id: 5, downloads: null, speedLimit: MB });
		expect(conflicts([stop, allow, later, videos, limit])).toEqual({ 2: [1] });
		// Off, it overrides nothing.
		expect(conflicts([{ ...stop, enabled: false }, allow])).toEqual({});
		// Once on a Sunday night meets every Monday morning.
		const once = slot({ id: 6, days: [], date: '2026-10-18', downloads: 'allow' });
		const monday = slot({ id: 7, days: ['monday'], from: '05:00', to: '06:00' });
		expect(conflicts([once, monday])).toEqual({ 7: [6] });
	});

	it('tell what applies to all downloads now', () => {
		const night = slot({ id: 1, active: true });
		const limit = slot({ id: 2, downloads: null, speedLimit: MB, active: true });
		const videos = slot({ id: 3, category: 4, active: true });
		expect(queueNow([videos, night, limit])).toEqual({
			stoppedBy: night,
			limit: { slot: limit, bytesPerSec: MB }
		});
		const allow = slot({ id: 4, downloads: 'allow', active: true });
		expect(queueNow([allow, night]).stoppedBy).toBeNull();
		expect(queueNow([{ ...night, active: false }])).toEqual({ stoppedBy: null, limit: null });
	});

	it('edit through a draft, saying what is missing', () => {
		const fresh = draftOf(null, '2026-10-14', 'en');
		expect(bodyOf(fresh, 'en')).toEqual({
			body: {
				name: '',
				enabled: true,
				from: '23:00',
				to: '07:00',
				days: DAYS,
				downloads: 'stop',
				letFinish: false,
				stayPaused: false
			}
		});
		// A stop sends no limits: the editor hides them.
		const stop = { ...fresh, stayPaused: true, limits: true, speed: '1', caps: true, max: 'x' };
		expect(bodyOf(stop, 'en')).toEqual({
			body: {
				name: '',
				enabled: true,
				from: '23:00',
				to: '07:00',
				days: DAYS,
				downloads: 'stop',
				letFinish: false,
				stayPaused: true
			}
		});
		const once = { ...fresh, once: true, downloads: null, limits: true, speed: '1.5', caps: true };
		expect(bodyOf(once, 'en')).toEqual({
			body: {
				name: '',
				enabled: true,
				from: '23:00',
				to: '07:00',
				date: '2026-10-14',
				speedLimit: 1.5 * MB,
				maxConcurrent: 1
			}
		});
		const problem = (draft: typeof fresh) => {
			const made = bodyOf(draft, 'en');
			return 'problem' in made ? made.problem : null;
		};
		expect(problem({ ...fresh, days: [] })).toBe('Pick at least one day.');
		expect(problem({ ...fresh, once: true, date: '' })).toBe('Pick the date.');
		expect(problem({ ...fresh, downloads: null })).toBe(
			'Turn on a speed limit or how many at once.'
		);
		expect(problem({ ...fresh, downloads: 'allow', caps: true, max: '0' })).toBe(
			'How many at once: from 1 to 50.'
		);
	});

	it('come back from a draft as they were', () => {
		const busy = slot({ category: 4, downloads: 'allow', speedLimit: 2 * MB, maxConcurrent: 3 });
		const made = bodyOf(draftOf(busy, '2026-10-14', 'en'), 'en');
		expect(made).toEqual({ body: bodyFromSlot(busy) });
		const strict = slot({ letFinish: true, stayPaused: true });
		expect(bodyOf(draftOf(strict, '2026-10-14', 'en'), 'en')).toEqual({
			body: bodyFromSlot(strict)
		});
		expect(bodyFromSlot(busy, { enabled: false }).enabled).toBe(false);
	});

	it('move one place up or down', () => {
		const list = [slot({ id: 1 }), slot({ id: 2 }), slot({ id: 3 })];
		expect(moved(list, 3, true)).toEqual([1, 3, 2]);
		expect(moved(list, 1, false)).toEqual([2, 1, 3]);
		expect(moved(list, 1, true)).toBeNull();
		expect(moved(list, 3, false)).toBeNull();
	});
});

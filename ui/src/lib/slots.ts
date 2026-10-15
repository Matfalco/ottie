// Time slots (docs/decisions/0040): the list in its order, what each says in
// words, where it sits in the week, which ones conflict, what applies to the
// whole queue now, and the editor's fields ↔ the body sent to the daemon.

import type { Day } from '@ottie/api-types/Day';
import type { Event as ApiEvent } from '@ottie/api-types/Event';
import { RANGES } from '@ottie/api-types/Ranges';
import type { Slot } from '@ottie/api-types/Slot';
import type { SlotBody } from '@ottie/api-types/SlotBody';
import { categoryName, type Categories } from './categories';
import { formatCount, formatLimit } from './format';
import { m } from './paraglide/messages.js';
import { getLocale, type Locale } from './paraglide/runtime.js';
import { fieldToLimit, limitToField, parseWhole, type SpeedUnit } from './settings';

export type Slots = Record<number, Slot>;

/** Monday first, as the daemon keeps them. */
export const DAYS: Day[] = [
	'monday',
	'tuesday',
	'wednesday',
	'thursday',
	'friday',
	'saturday',
	'sunday'
];

const DAY_MINUTES = 24 * 60;

export function applySlotEvent(slots: Slots, event: ApiEvent): void {
	if (event.type === 'slotChanged') {
		slots[event.slot.id] = event.slot;
	} else if (event.type === 'slotRemoved') {
		delete slots[event.id];
	}
}

/** In the order of the list: the higher one wins. */
export function sortedSlots(slots: Slots): Slot[] {
	return Object.values(slots).sort((a, b) => a.place - b.place || a.id - b.id);
}

/** "Monday" or "Mon", in the page's language. */
export function dayName(day: Day, style: 'long' | 'short', locale: Locale = getLocale()): string {
	// 2024-01-01 was a Monday.
	const date = new Date(2024, 0, 1 + DAYS.indexOf(day));
	return new Intl.DateTimeFormat(locale, { weekday: style }).format(date);
}

/** "HH:MM" as the page shows times: "23:00", or "11:00 PM". */
export function formatClock(hhmm: string, locale: Locale = getLocale()): string {
	const minutes = minutesOf(hhmm);
	const at = new Date(2024, 0, 1, Math.floor(minutes / 60), minutes % 60);
	return new Intl.DateTimeFormat(locale, { timeStyle: 'short' }).format(at);
}

/** An hour on the week view's side: "6 AM", "18 h". */
export function formatHour(hour: number, locale: Locale = getLocale()): string {
	return new Intl.DateTimeFormat(locale, { hour: 'numeric' }).format(new Date(2024, 0, 1, hour));
}

/** Minutes from midnight of "HH:MM". */
export function minutesOf(hhmm: string): number {
	const [hours, minutes] = hhmm.split(':').map(Number);
	return (hours ?? 0) * 60 + (minutes ?? 0);
}

/** A "YYYY-MM-DD" date at midnight in the page's time zone. */
function dateOf(text: string): Date {
	const [year, month, day] = text.split('-').map(Number);
	return new Date(year ?? 1970, (month ?? 1) - 1, day ?? 1);
}

/** "Every day", "Mon–Fri", "Sat, Sun", "Mon, Wed–Fri"; for a slot that
 * happens once, its date ("Thu, Oct 15"). */
export function describeDays(slot: Slot, locale: Locale = getLocale()): string {
	const options = { locale };
	if (slot.date !== null) {
		return new Intl.DateTimeFormat(locale, {
			weekday: 'short',
			day: 'numeric',
			month: 'short'
		}).format(dateOf(slot.date));
	}
	if (slot.days.length === DAYS.length) {
		return m.slot_every_day({}, options);
	}
	// Runs of three days or more read as a range.
	const runs: Day[][] = [];
	for (const day of DAYS.filter((day) => slot.days.includes(day))) {
		const run = runs.at(-1);
		const last = run?.at(-1);
		if (run && last && DAYS.indexOf(day) === DAYS.indexOf(last) + 1) {
			run.push(day);
		} else {
			runs.push([day]);
		}
	}
	const short = (day: Day) => dayName(day, 'short', locale);
	return runs
		.flatMap((run) => {
			const [first] = run;
			const last = run.at(-1);
			return run.length >= 3 && first && last ? [`${short(first)}–${short(last)}`] : run.map(short);
		})
		.join(', ');
}

/** "01:00 – 07:00", "23:00 – 07:00 (next day)", or "All day". */
export function describeHours(slot: Slot, locale: Locale = getLocale()): string {
	const options = { locale };
	if (slot.from === slot.to && slot.from === '00:00') {
		return m.slot_all_day({}, options);
	}
	const hours = `${formatClock(slot.from, locale)} – ${formatClock(slot.to, locale)}`;
	return minutesOf(slot.to) <= minutesOf(slot.from) ? m.slot_next_day({ hours }, options) : hours;
}

/** "Whole queue", or the category's name. */
export function describeTarget(
	slot: Slot,
	categories: Categories,
	locale: Locale = getLocale()
): string {
	const options = { locale };
	const category = slot.category === null ? undefined : categories[slot.category];
	if (slot.category === null) {
		return m.slot_whole_queue({}, options);
	}
	return category ? categoryName(category, locale) : m.slot_unknown_category({}, options);
}

/** What it does, one part each: "Stops downloads", "Then they stay
 * paused", "Allows downloads", "1 MB/s", "No limit", "2 at once". */
export function describeEffects(slot: Slot, locale: Locale = getLocale()): string[] {
	const options = { locale };
	const parts: string[] = [];
	if (slot.downloads === 'allow') {
		parts.push(m.slot_allows({}, options));
	} else if (slot.downloads === 'stop') {
		parts.push(slot.letFinish ? m.slot_stops_new({}, options) : m.slot_stops({}, options));
		if (slot.stayPaused) {
			parts.push(m.slot_stay_paused({}, options));
		}
	}
	if (slot.speedLimit !== null) {
		parts.push(
			slot.speedLimit === 0
				? m.slot_no_limit({}, options)
				: m.slot_limit({ speed: formatLimit(slot.speedLimit, locale) }, options)
		);
	}
	if (slot.maxConcurrent !== null) {
		parts.push(m.slot_at_once({ count: formatCount(slot.maxConcurrent, locale) }, options));
	}
	return parts;
}

/** Its name, or when it is if it has none ("Mon–Fri · 09:00 – 18:00"). */
export function slotLabel(slot: Slot, locale: Locale = getLocale()): string {
	return slot.name.trim() || `${describeDays(slot, locale)} · ${describeHours(slot, locale)}`;
}

/** Its name, or its hours if it has none: short enough for the title bar. */
export function shortLabel(slot: Slot, locale: Locale = getLocale()): string {
	return slot.name.trim() || describeHours(slot, locale);
}

/** Labels by ID, for "Held by Night". */
export function slotNames(slots: Slot[], locale: Locale = getLocale()): Record<number, string> {
	return Object.fromEntries(slots.map((slot) => [slot.id, slotLabel(slot, locale)]));
}

/** A part of a day a slot covers, in minutes from midnight; `day` is 0 for
 * Monday. A night is two parts: `first` holds its start, `last` its end. */
export interface Block {
	slot: number;
	day: number;
	start: number;
	end: number;
	first: boolean;
	last: boolean;
}

/** Where a slot sits in the week starting on `monday`: a block for each day
 * it covers. A night runs into the next day (Sunday's into Monday's, for a
 * weekly one); a slot that happens once shows only in its own week. */
export function weekBlocks(slot: Slot, monday: Date): Block[] {
	if (slot.date === null) {
		return slot.days.flatMap((day) => dayBlocks(slot, DAYS.indexOf(day), true));
	}
	const offset = Math.round((dateOf(slot.date).getTime() - monday.getTime()) / 86_400_000);
	return offset >= 0 && offset < 7 ? dayBlocks(slot, offset, false) : [];
}

/** The blocks of one occurrence starting on `day`. */
function dayBlocks(slot: Slot, day: number, wrap: boolean): Block[] {
	const from = minutesOf(slot.from);
	const to = minutesOf(slot.to);
	if (to > from) {
		return [{ slot: slot.id, day, start: from, end: to, first: true, last: true }];
	}
	const blocks = [
		{ slot: slot.id, day, start: from, end: DAY_MINUTES, first: true, last: to === 0 }
	];
	const next = wrap ? (day + 1) % 7 : day + 1;
	if (to > 0 && next < 7) {
		blocks.push({ slot: slot.id, day: next, start: 0, end: to, first: false, last: true });
	}
	return blocks;
}

/** What a slot mostly does, for its colour: stops, allows, or only limits. */
export type SlotKind = 'stop' | 'allow' | 'limit';

export function slotKind(slot: Slot): SlotKind {
	return slot.downloads ?? 'limit';
}

/** A block placed in the week view: blocks that overlap sit side by side,
 * `lane` of `lanes`, the higher slot in the list on the left. */
export interface PlacedBlock extends Block {
	lane: number;
	lanes: number;
}

/** The blocks of the slots that are on, in the week starting on `monday`.
 * On each day, blocks that overlap (directly or through others) share the
 * width; each takes the first lane free at its time. */
export function weekLayout(list: Slot[], monday: Date): PlacedBlock[] {
	const meets = (a: Block, b: Block) => a.start < b.end && b.start < a.end;
	const enabled = list.filter((slot) => slot.enabled);
	const placed: PlacedBlock[] = [];
	for (let day = 0; day < 7; day++) {
		// In list order, so the higher slot gets the left lane.
		const blocks = enabled.flatMap((slot) =>
			weekBlocks(slot, monday).filter((block) => block.day === day)
		);
		const lanes: number[] = [];
		blocks.forEach((block, i) => {
			const used = blocks.slice(0, i).flatMap((other, j) => (meets(other, block) ? lanes[j] : []));
			let lane = 0;
			while (used.includes(lane)) {
				lane++;
			}
			lanes.push(lane);
		});
		// Blocks that meet, directly or through others, share one width.
		const group = blocks.map((_, i) => i);
		const root = (i: number): number => {
			const parent = group[i] ?? i;
			return parent === i ? i : root(parent);
		};
		blocks.forEach((a, i) =>
			blocks.forEach((b, j) => {
				if (j > i && meets(a, b)) {
					group[root(j)] = root(i);
				}
			})
		);
		blocks.forEach((block, i) => {
			const width = Math.max(
				...blocks.map((_, j) => (root(j) === root(i) ? (lanes[j] ?? 0) + 1 : 1))
			);
			placed.push({ ...block, lane: lanes[i] ?? 0, lanes: width });
		});
	}
	return placed;
}

/** What dragging a block in the week view does: move the whole slot, or
 * only its start or its end. */
export type Drag = 'move' | 'start' | 'end';

/** Drags snap to a quarter of an hour. */
export const SNAP = 15;

/** How long it lasts, in minutes (the same time twice: a whole day). */
export function lengthOf(slot: Slot): number {
	const length = (minutesOf(slot.to) - minutesOf(slot.from) + DAY_MINUTES) % DAY_MINUTES;
	return length === 0 ? DAY_MINUTES : length;
}

/** `slot` after a drag of `minutes` down and `days` to the right (either
 * may be negative), by steps of `SNAP`. Its start stays on its day: a
 * slot's days move only sideways. It lasts from `SNAP` to a whole day. */
export function dragged(slot: Slot, drag: Drag, minutes: number, days: number): Slot {
	const step = Math.round(minutes / SNAP) * SNAP;
	const clamp = (value: number, low: number, high: number) => Math.min(Math.max(value, low), high);
	const from = minutesOf(slot.from);
	const length = lengthOf(slot);
	let start = from;
	let end = from + length;
	if (drag === 'move') {
		start = clamp(from + step, 0, DAY_MINUTES - SNAP);
		end = start + length;
	} else if (drag === 'start') {
		const latest = Math.min(end, DAY_MINUTES) - SNAP;
		start = clamp(from + step, Math.max(0, end - DAY_MINUTES), latest);
	} else {
		end = clamp(end + step, start + SNAP, start + DAY_MINUTES);
	}
	const shift = drag === 'move' ? days : 0;
	return {
		...slot,
		from: clockOf(start),
		to: clockOf(end % DAY_MINUTES),
		days: DAYS.filter((day) =>
			slot.days.some((each) => DAYS.indexOf(each) === (DAYS.indexOf(day) - shift + 70) % 7)
		),
		date: slot.date === null ? null : shiftDate(slot.date, shift)
	};
}

/** "HH:MM" of minutes from midnight. */
function clockOf(minutes: number): string {
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${pad(Math.floor(minutes / 60))}:${pad(minutes % 60)}`;
}

/** A "YYYY-MM-DD" date `days` later. */
function shiftDate(text: string, days: number): string {
	const date = dateOf(text);
	date.setDate(date.getDate() + days);
	const pad = (n: number) => String(n).padStart(2, '0');
	return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** The Monday that starts the week of `date`, at midnight. */
export function mondayOf(date: Date): Date {
	const monday = new Date(date.getFullYear(), date.getMonth(), date.getDate());
	monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7));
	return monday;
}

/** Slots that a higher one overrides somewhere: both on, for the same
 * target, at times that overlap, setting the same thing differently. By
 * slot ID, the higher slots it conflicts with. */
export function conflicts(list: Slot[]): Record<number, number[]> {
	const found: Record<number, number[]> = {};
	list.forEach((higher, i) => {
		for (const lower of list.slice(i + 1)) {
			if (
				higher.enabled &&
				lower.enabled &&
				higher.category === lower.category &&
				overlap(higher, lower) &&
				disagree(higher, lower)
			) {
				(found[lower.id] ??= []).push(higher.id);
			}
		}
	});
	return found;
}

/** Slots that overlap another one: both on, at times that overlap, for
 * downloads they share (the same category, or one is for all of them). By
 * slot ID, the others it overlaps, in list order. */
export function overlaps(list: Slot[]): Record<number, number[]> {
	const found: Record<number, number[]> = {};
	for (const slot of list) {
		const others = meeting(slot, list);
		if (others.length > 0) {
			found[slot.id] = others.map((other) => other.id);
		}
	}
	return found;
}

/** The slots of `list` (but itself) that `slot` overlaps, if it's on. */
export function meeting(slot: Slot, list: Slot[]): Slot[] {
	const shared = (other: Slot) =>
		slot.category === null || other.category === null || slot.category === other.category;
	return slot.enabled
		? list.filter(
				(other) => other.id !== slot.id && other.enabled && shared(other) && overlap(slot, other)
			)
		: [];
}

/** A slot being edited as it would be saved (`id` null for a new one),
 * to see what it overlaps. */
export function slotOfBody(body: SlotBody, id: number | null): Slot {
	return {
		id: id ?? -1,
		place: 0,
		name: body.name ?? '',
		enabled: body.enabled ?? true,
		category: body.category ?? null,
		days: body.days ?? [],
		date: body.date ?? null,
		from: body.from,
		to: body.to,
		downloads: body.downloads ?? null,
		letFinish: body.letFinish ?? false,
		stayPaused: body.stayPaused ?? false,
		speedLimit: body.speedLimit ?? null,
		maxConcurrent: body.maxConcurrent ?? null,
		active: false
	};
}

/** Whether they can apply at the same time: blocks that meet on a weekday
 * (for one that happens once, its weekday; two of those, the same date). */
function overlap(a: Slot, b: Slot): boolean {
	if (a.date !== null && b.date !== null && a.date !== b.date) {
		return false;
	}
	const blocks = (slot: Slot) => {
		const days = slot.date === null ? slot.days : [DAYS[(dateOf(slot.date).getDay() + 6) % 7]];
		return days.flatMap((day) => (day ? dayBlocks(slot, DAYS.indexOf(day), true) : []));
	};
	const theirs = blocks(b);
	return blocks(a).some((x) =>
		theirs.some((y) => x.day === y.day && x.start < y.end && y.start < x.end)
	);
}

/** Whether they set the same thing to different values. */
function disagree(a: Slot, b: Slot): boolean {
	const both = <T>(x: T | null, y: T | null) => x !== null && y !== null && x !== y;
	const stopsDiffer =
		a.downloads !== null &&
		b.downloads !== null &&
		(a.downloads !== b.downloads ||
			(a.downloads === 'stop' && (a.letFinish !== b.letFinish || a.stayPaused !== b.stayPaused)));
	return stopsDiffer || both(a.speedLimit, b.speedLimit) || both(a.maxConcurrent, b.maxConcurrent);
}

/** What the slots do to the whole queue right now, for the title bar: the
 * one stopping downloads, and the one setting the total limit. */
export function queueNow(list: Slot[]): {
	stoppedBy: Slot | null;
	limit: { slot: Slot; bytesPerSec: number } | null;
} {
	const active = list.filter((slot) => slot.active && slot.category === null);
	const deciding = active.find((slot) => slot.downloads !== null);
	const limiting = active.find((slot) => slot.speedLimit !== null);
	return {
		stoppedBy: deciding?.downloads === 'stop' ? deciding : null,
		limit:
			limiting && limiting.speedLimit !== null
				? { slot: limiting, bytesPerSec: limiting.speedLimit }
				: null
	};
}

/** The editor's fields. */
export interface Draft {
	name: string;
	enabled: boolean;
	/** A category's ID; null: the whole queue. */
	category: number | null;
	/** Once, on `date`; else every week on `days`. */
	once: boolean;
	days: Day[];
	/** "YYYY-MM-DD". */
	date: string;
	/** "HH:MM". */
	from: string;
	to: string;
	/** null: only limits. */
	downloads: 'allow' | 'stop' | null;
	/** With a stop: running ones finish. */
	letFinish: boolean;
	/** With a stop: what it held stays paused once it ends. */
	stayPaused: boolean;
	/** A speed limit is set (`speed` in `unit`; empty: no limit); not with
	 * a stop. */
	limits: boolean;
	speed: string;
	unit: SpeedUnit;
	/** How many at once is set. */
	caps: boolean;
	max: string;
}

/** The fields for `slot`, or for a new one (every night, stopping
 * downloads; `today` for the date if made to happen once). */
export function draftOf(slot: Slot | null, today: string, locale: Locale = getLocale()): Draft {
	const limit = limitToField(slot?.speedLimit ?? 0, locale);
	return {
		name: slot?.name ?? '',
		enabled: slot?.enabled ?? true,
		category: slot?.category ?? null,
		once: slot !== null && slot.date !== null,
		days: slot?.date === null ? slot.days : slot ? [] : [...DAYS],
		date: slot?.date ?? today,
		from: slot?.from ?? '23:00',
		to: slot?.to ?? '07:00',
		downloads: slot ? slot.downloads : 'stop',
		letFinish: slot?.letFinish ?? false,
		stayPaused: slot?.stayPaused ?? false,
		limits: slot?.speedLimit != null,
		speed: limit.value,
		unit: limit.unit,
		caps: slot?.maxConcurrent != null,
		max: String(slot?.maxConcurrent ?? 1)
	};
}

/** The body to send, or what's missing (in the page's language). */
export function bodyOf(
	draft: Draft,
	locale: Locale = getLocale()
): { body: SlotBody } | { problem: string } {
	const options = { locale };
	if (draft.once ? draft.date === '' : draft.days.length === 0) {
		return {
			problem: draft.once ? m.slot_need_date({}, options) : m.reason_slot_without_days({}, options)
		};
	}
	if (draft.from === '' || draft.to === '') {
		return { problem: m.slot_need_hours({}, options) };
	}
	// A stop leaves the limits out: the editor hides them.
	const stops = draft.downloads === 'stop';
	const caps = draft.caps && !stops;
	const max = caps ? parseWhole(draft.max, RANGES.maxConcurrent) : null;
	if (caps && max === null) {
		const { min, max: most } = RANGES.maxConcurrent;
		return {
			problem: m.slot_bad_max(
				{ min: formatCount(min, locale), max: formatCount(most, locale) },
				options
			)
		};
	}
	if (draft.downloads === null && !draft.limits && !draft.caps) {
		return { problem: m.slot_need_limit({}, options) };
	}
	const body: SlotBody = {
		name: draft.name.trim(),
		enabled: draft.enabled,
		from: draft.from,
		to: draft.to
	};
	if (draft.category !== null) {
		body.category = draft.category;
	}
	if (draft.once) {
		body.date = draft.date;
	} else {
		body.days = DAYS.filter((day) => draft.days.includes(day));
	}
	if (draft.downloads !== null) {
		body.downloads = draft.downloads;
		body.letFinish = stops && draft.letFinish;
		body.stayPaused = stops && draft.stayPaused;
	}
	if (draft.limits && !stops) {
		// An empty or zero speed: no limit while it lasts.
		body.speedLimit = fieldToLimit(draft.speed, draft.unit) ?? 0;
	}
	if (max !== null) {
		body.maxConcurrent = max;
	}
	return { body };
}

/** The body that saves `slot` as it is, but for the changes given (to
 * switch it on or off from the list). */
export function bodyFromSlot(slot: Slot, change: Partial<SlotBody> = {}): SlotBody {
	const body: SlotBody = {
		name: slot.name,
		enabled: slot.enabled,
		from: slot.from,
		to: slot.to,
		letFinish: slot.letFinish,
		stayPaused: slot.stayPaused
	};
	if (slot.category !== null) {
		body.category = slot.category;
	}
	if (slot.date !== null) {
		body.date = slot.date;
	} else {
		body.days = slot.days;
	}
	if (slot.downloads !== null) {
		body.downloads = slot.downloads;
	}
	if (slot.speedLimit !== null) {
		body.speedLimit = slot.speedLimit;
	}
	if (slot.maxConcurrent !== null) {
		body.maxConcurrent = slot.maxConcurrent;
	}
	return { ...body, ...change };
}

/** The list's order after moving `id` one place `up` (or down); null if it
 * can't move that way. */
export function moved(list: Slot[], id: number, up: boolean): number[] | null {
	const ids = list.map((slot) => slot.id);
	const from = ids.indexOf(id);
	const to = up ? from - 1 : from + 1;
	if (from < 0 || to < 0 || to >= ids.length) {
		return null;
	}
	[ids[from], ids[to]] = [ids[to] ?? id, ids[from] ?? id];
	return ids;
}

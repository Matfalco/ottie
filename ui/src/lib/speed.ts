// The total download speed over the last hour, for the dashboard's graph:
// one sample per second (the daemon's, then the page's own), how much of it
// the graph shows, and the shapes to draw (docs/decisions/0044).

import { currentSpeed, type Rate } from './jobs';

/** An hour of one-per-second samples: the most the graph can show. */
export const HISTORY_SECONDS = 3600;

/** How far back the graph can look, in seconds. */
export const SPANS = [300, 900, 1800, 3600] as const;
export type Span = (typeof SPANS)[number];
export const DEFAULT_SPAN: Span = 300;

/** The most points the graph draws: a longer span averages neighbouring
 * seconds into one. */
export const POINTS = 300;

const SPAN_KEY = 'ottie.speedSpan';

/** Every download's speed added up, at `now`. */
export function totalSpeed(rates: Record<string, Rate>, now: number): number {
	return Object.values(rates).reduce((sum, rate) => sum + currentSpeed(rate, now), 0);
}

/** How long after the daemon's history comes the page's own rates are
 * still being measured (two progress reports each). */
export const MEASURING_MS = 2500;

/** The page's next sample, `ownSpeed` (its rates added up), `sinceHistory`
 * ms after the daemon's history came: until its rates are measured it
 * carries the daemon's last sample on, rather than dropping to nothing. */
export function nextSample(history: number[], ownSpeed: number, sinceHistory: number): number {
	return sinceHistory < MEASURING_MS ? (history.at(-1) ?? ownSpeed) : ownSpeed;
}

/** `history` with `sample` added at the end, keeping the last `keep`. */
export function addSample(history: number[], sample: number, keep = HISTORY_SECONDS): number[] {
	const next = [...history, sample];
	return next.length > keep ? next.slice(next.length - keep) : next;
}

/** A round top for the graph's scale, at or above `peak`: 1, 2 or 5 times a
 * power of ten in the unit the labels use (kB, MB…, which count in 1024s),
 * so the top label reads "5 MB/s" rather than "4.8 MB/s". Never below
 * 100 kB/s, so a trickle doesn't fill the graph. */
export function niceTop(peak: number): number {
	const value = Math.max(peak, 100 * 1024);
	let unit = 1;
	// From 500 of a unit on, count in the next one: 640 kB tops out at 1 MB.
	while (value / unit >= 500) {
		unit *= 1024;
	}
	const inUnit = value / unit;
	const power = 10 ** Math.floor(Math.log10(inUnit));
	const step = [1, 2, 5, 10].find((m) => m * power >= inUnit) ?? 10;
	return step * power * unit;
}

/** The last `span` seconds of `history` as at most `most` points, each the
 * average of `step` seconds, the newest last. Counted back from now, so the
 * newest point always holds the latest seconds. */
export function overSpan(
	history: number[],
	span: number,
	most = POINTS
): { samples: number[]; step: number } {
	const step = Math.max(1, Math.ceil(span / most));
	const recent = history.slice(-span);
	const samples: number[] = [];
	for (let end = recent.length; end > 0; end -= step) {
		const group = recent.slice(Math.max(0, end - step), end);
		samples.push(group.reduce((sum, value) => sum + value, 0) / group.length);
	}
	return { samples: samples.reverse(), step };
}

/** The span this device last showed; five minutes at first. */
export function loadSpan(): Span {
	try {
		return parseSpan(localStorage.getItem(SPAN_KEY));
	} catch {
		return DEFAULT_SPAN; // Storage blocked.
	}
}

export function saveSpan(span: Span): void {
	try {
		localStorage.setItem(SPAN_KEY, String(span));
	} catch {
		// Storage blocked: five minutes next time.
	}
}

/** Reads what `saveSpan` wrote; anything else is the default. */
export function parseSpan(stored: string | null): Span {
	return SPANS.find((span) => String(span) === stored) ?? DEFAULT_SPAN;
}

/** Where the samples go in a `width` × `height` box: the newest at the right
 * edge, one point per step of `width / (count - 1)`, 0 at the bottom and
 * `top` at the top. */
export function points(
	history: number[],
	width: number,
	height: number,
	top: number,
	count = POINTS
): { x: number; y: number }[] {
	const step = width / (count - 1);
	const last = history.length - 1;
	return history.map((value, i) => ({
		x: width - (last - i) * step,
		y: height - (Math.min(value, top) / top) * height
	}));
}

/** SVG path data: the line through the points, and the area under it. */
export function paths(
	at: { x: number; y: number }[],
	height: number
): { line: string; area: string } {
	if (at.length === 0) {
		return { line: '', area: '' };
	}
	const line = at
		.map(({ x, y }, i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`)
		.join('');
	const first = at[0];
	const last = at[at.length - 1];
	const area = `${line}L${last.x.toFixed(1)},${height}L${first.x.toFixed(1)},${height}Z`;
	return { line, area };
}

/** The point under a pointer at `x` on a chart of `count` points spanning
 * `left` to `left + width`, as points before the newest. The newest is at the
 * end of the line: the right, or the left on a right-to-left page (`rtl`). */
export function pointsAgo(
	x: number,
	left: number,
	width: number,
	rtl: boolean,
	count = POINTS
): number {
	const fromLeft = Math.min(1, Math.max(0, (x - left) / width));
	const towardNow = rtl ? 1 - fromLeft : fromLeft;
	// The crosshair snaps to the nearest point.
	return Math.round((1 - towardNow) * (count - 1));
}

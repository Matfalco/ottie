// The total download speed over the last minutes, for the dashboard's graph:
// one sample per second, and the shapes to draw from them.

import { currentSpeed, type Rate } from './jobs';

/** Five minutes of one-per-second samples. */
export const HISTORY_SECONDS = 300;

/** Every download's speed added up, at `now`. */
export function totalSpeed(rates: Record<string, Rate>, now: number): number {
	return Object.values(rates).reduce((sum, rate) => sum + currentSpeed(rate, now), 0);
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

/** Where the samples go in a `width` × `height` box: the newest at the right
 * edge, one second per step of `width / (seconds - 1)`, 0 at the bottom and
 * `top` at the top. */
export function points(
	history: number[],
	width: number,
	height: number,
	top: number,
	seconds = HISTORY_SECONDS
): { x: number; y: number }[] {
	const step = width / (seconds - 1);
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

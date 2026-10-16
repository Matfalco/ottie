// Animations (docs/decisions/0048): short and quiet, there to show where
// things come from and go, never waited for. None when the system asks for
// less motion, unless this device says otherwise in Settings.

export type Motion = 'system' | 'on' | 'off';

export const MOTIONS: Motion[] = ['system', 'on', 'off'];

/** Whether the page moves: the device's choice, else the system's. */
export function moves(choice: Motion, systemReduced: boolean): boolean {
	return choice === 'on' || (choice === 'system' && !systemReduced);
}

/** Durations, in milliseconds: things going (and menus) are quicker than
 * things coming, so nothing is in the way for long. */
export const QUICK = 120;
export const STEADY = 180;
/** Starts fast and settles: things arriving. */
export const SETTLE = 'cubic-bezier(0.2, 0, 0, 1)';

/** More rows than this coming or going at once (a filter, a page of links,
 * clearing the finished ones) appear and go at once rather than fade: a
 * crowd of fades is noise. */
export const MANY = 8;

/** More rows than this in a list and it doesn't move at all. */
export const CROWD = 400;

/** Where a row is on the screen. */
export interface Spot {
	left: number;
	top: number;
	width: number;
	height: number;
}

/** What happens to a list's rows after a change, each named by its key. */
export interface Plan {
	/** Still listed but elsewhere: each starts where it was (`dx`, `dy` back)
	 * and slides into its place; `sx`, `sy` scale it back to its old size. */
	moved: { key: string; dx: number; dy: number; sx: number; sy: number }[];
	/** Newly listed: they fade in. */
	entered: string[];
	/** No longer listed: they fade out where they were. */
	fading: string[];
	/** No longer listed, and gone at once (too many at once). */
	hidden: string[];
}

/** What each row does, from where the rows were (`before`) and are now
 * (`after`). A row that only moves out of sight (both spots outside the
 * window's `height`) doesn't move. */
export function plan(before: Map<string, Spot>, after: Map<string, Spot>, height: number): Plan {
	const moved: Plan['moved'] = [];
	const entered: string[] = [];
	const left: string[] = [];
	const seen = (spot: Spot) => spot.top + spot.height > 0 && spot.top < height;
	for (const [key, now] of after) {
		const was = before.get(key);
		if (was === undefined) {
			entered.push(key);
			continue;
		}
		const dx = was.left - now.left;
		const dy = was.top - now.top;
		const sx = now.width > 0 ? was.width / now.width : 1;
		const sy = now.height > 0 ? was.height / now.height : 1;
		const still = Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5;
		if ((!still || sx !== 1 || sy !== 1) && (seen(was) || seen(now))) {
			moved.push({ key, dx, dy, sx, sy });
		}
	}
	for (const key of before.keys()) {
		if (!after.has(key)) {
			left.push(key);
		}
	}
	return {
		moved,
		entered: entered.length <= MANY ? entered : [],
		fading: left.length <= MANY ? left : [],
		hidden: left.length <= MANY ? [] : left
	};
}

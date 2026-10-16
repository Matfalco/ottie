// Where a hint's bubble goes (the (i) next to a setting): above its button,
// starting where the button starts, kept inside the window. Fixed to the
// window, so a scrolling panel can't cut it off.

export interface Box {
	left: number;
	top: number;
	right: number;
	bottom: number;
}

export interface Size {
	width: number;
	height: number;
}

/** Between the bubble and its button. */
const GAP = 6;
/** Kept free along the window's edges. */
const MARGIN = 8;

/** The bubble's top-left corner, for a button at `anchor` in a window of
 * `view`; `rtl` starts it at the button's right edge instead. Below the
 * button when there's no room above it. */
export function bubbleAt(anchor: Box, bubble: Size, view: Size, rtl: boolean): Box {
	const start = rtl ? anchor.right + MARGIN - bubble.width : anchor.left - MARGIN;
	const left = Math.max(MARGIN, Math.min(start, view.width - MARGIN - bubble.width));
	const above = anchor.top - GAP - bubble.height;
	const top = above >= MARGIN ? above : anchor.bottom + GAP;
	return { left, top, right: left + bubble.width, bottom: top + bubble.height };
}

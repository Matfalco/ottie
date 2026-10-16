import { describe, expect, it } from 'vitest';
import { bubbleAt } from './hint';

const view = { width: 960, height: 680 };
const bubble = { width: 270, height: 60 };
const button = (left: number, top: number) => ({
	left,
	top,
	right: left + 18,
	bottom: top + 18
});

describe('a hint’s bubble', () => {
	it('sits above its button, starting where the button starts', () => {
		expect(bubbleAt(button(100, 400), bubble, view, false)).toEqual({
			left: 92,
			top: 334,
			right: 362,
			bottom: 394
		});
	});

	it('ends where the button ends in a page written right to left', () => {
		const at = bubbleAt(button(600, 400), bubble, view, true);
		expect(at.right).toBe(626);
	});

	it('stays inside the window at either edge', () => {
		expect(bubbleAt(button(900, 400), bubble, view, false).right).toBe(952);
		expect(bubbleAt(button(20, 400), bubble, view, true).left).toBe(8);
	});

	it('goes under its button when there is no room above', () => {
		expect(bubbleAt(button(100, 30), bubble, view, false).top).toBe(54);
	});

	it('starts at the margin when it is wider than the window', () => {
		expect(
			bubbleAt(button(100, 400), { width: 400, height: 60 }, { width: 360, height: 640 }, false)
				.left
		).toBe(8);
	});
});

import { describe, expect, it } from 'vitest';
import { byQueue, canDrop, canReorder, carried, upNext } from './queue';
import { job } from './testing';

const urgent = job({ id: 'u', priority: 'urgent', place: 9 });
const first = job({ id: 'a', place: 2 });
const paused = job({ id: 'b', place: 3, state: 'paused' });
const last = job({ id: 'c', place: 7 });
const low = job({ id: 'l', priority: 'low', place: 1 });
const running = job({ id: 'r', place: 5, state: 'downloading' });
const later = job({ id: 's', place: 8, state: 'scheduled', startAt: 1 });
const jobs = [last, low, running, paused, first, urgent, later];
const ids = (list: { id: string }[]) => list.map((each) => each.id);

describe('the queue', () => {
	it('goes by priority, then by place', () => {
		expect(ids([...jobs].sort(byQueue))).toEqual(['u', 'a', 'b', 'r', 'c', 's', 'l']);
	});

	it('lists what starts next, leaving out paused, scheduled and running downloads', () => {
		expect(ids(upNext(jobs))).toEqual(['u', 'a', 'c', 'l']);
	});

	it('can move a download past a waiting one of its priority', () => {
		expect(canReorder(jobs, [paused], 'up')).toBe(true);
		expect(canReorder(jobs, [paused], 'down')).toBe(true);
		// The running one in between doesn't count.
		expect(canReorder(jobs, [last], 'top')).toBe(true);
		// A scheduled one waits in the queue too.
		expect(canReorder(jobs, [last], 'down')).toBe(true);
		expect(canReorder(jobs, [later], 'down')).toBe(false);
	});

	it('cannot move past another priority, nor past the selection itself', () => {
		expect(canReorder(jobs, [first], 'up')).toBe(false);
		expect(canReorder(jobs, [urgent], 'down')).toBe(false);
		expect(canReorder(jobs, [low], 'up')).toBe(false);
		expect(canReorder(jobs, [first, paused, last], 'up')).toBe(false);
		expect(canReorder(jobs, [first, paused], 'down')).toBe(true);
	});

	it('cannot move downloads that are not waiting', () => {
		expect(canReorder(jobs, [running], 'up')).toBe(false);
		expect(canReorder(jobs, [], 'up')).toBe(false);
	});
});

describe('dragging in the queue', () => {
	const other = job({ id: 'd', place: 8 });
	it('carries the selection of its priority, or only the grabbed download', () => {
		expect(ids(carried(last, [last, paused, urgent, running]))).toEqual(['b', 'c']);
		expect(ids(carried(first, [last, paused]))).toEqual(['a']);
	});

	it('drops only next to another waiting download of the same priority', () => {
		expect(canDrop([last], first)).toBe(true);
		expect(canDrop([last, other], paused)).toBe(true);
		expect(canDrop([last], last)).toBe(false);
		expect(canDrop([last], urgent)).toBe(false);
		expect(canDrop([last], running)).toBe(false);
		expect(canDrop([], first)).toBe(false);
	});
});

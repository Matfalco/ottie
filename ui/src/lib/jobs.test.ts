import { describe, expect, it } from 'vitest';
import { job } from './format.test';
import { applyEvent, currentSpeed, newestFirst, nextRate, type Jobs } from './jobs';

describe('applyEvent', () => {
	it('upserts changed jobs and updates progress', () => {
		const jobs: Jobs = {};
		expect(applyEvent(jobs, { type: 'changed', job: job() })).toBe(false);
		applyEvent(jobs, { type: 'progress', id: job().id, downloaded: 500, total: 1000 });
		expect(jobs[job().id]).toMatchObject({ downloaded: 500, total: 1000 });
		applyEvent(jobs, { type: 'changed', job: job({ state: 'completed' }) });
		expect(jobs[job().id].state).toBe('completed');
	});

	it('ignores progress for unknown jobs and asks for a resync when told', () => {
		const jobs: Jobs = {};
		applyEvent(jobs, { type: 'progress', id: 'nope', downloaded: 1, total: null });
		expect(jobs).toEqual({});
		expect(applyEvent(jobs, { type: 'resync' })).toBe(true);
	});
});

describe('newestFirst', () => {
	it('sorts by ID, numbers included', () => {
		const ids = ['2026.10.12-14.03.07-9', '2026.10.12-14.03.07-10', '2026.10.11-23.59.59-1'];
		const jobs: Jobs = Object.fromEntries(ids.map((id) => [id, job({ id })]));
		expect(newestFirst(jobs).map((j) => j.id)).toEqual([
			'2026.10.12-14.03.07-10',
			'2026.10.12-14.03.07-9',
			'2026.10.11-23.59.59-1'
		]);
	});
});

describe('speed', () => {
	it('measures, then smooths', () => {
		const first = nextRate(undefined, 0, 0);
		const second = nextRate(first, 1000, 1000);
		expect(second.bytesPerSec).toBe(1000);
		const third = nextRate(second, 4000, 2000); // 3000 B/s measured
		expect(third.bytesPerSec).toBeCloseTo(1000 + 0.3 * 2000);
	});

	it('waits for samples far enough apart, and resets on a restart', () => {
		const rate = nextRate(nextRate(undefined, 0, 0), 1000, 1000);
		expect(nextRate(rate, 2000, 1100)).toBe(rate);
		expect(nextRate(rate, 10, 2000).bytesPerSec).toBe(0);
	});

	it('shows 0 once progress stops arriving', () => {
		const rate = { at: 1000, downloaded: 5, bytesPerSec: 800 };
		expect(currentSpeed(rate, 2000)).toBe(800);
		expect(currentSpeed(rate, 5000)).toBe(0);
		expect(currentSpeed(undefined, 5000)).toBe(0);
	});
});

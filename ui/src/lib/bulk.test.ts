import { describe, expect, it } from 'vitest';
import { confirmation, pauseSteps, resumeTargets, targets } from './bulk';
import { job } from './testing';

const paused = job({ id: '2026.10.12-14.03.07-1', state: 'paused', downloaded: 100 });
const done = job({ id: '2026.10.12-14.03.07-2', state: 'completed', fileName: 'a.zip' });
const queued = job({ id: '2026.10.12-14.03.07-3', state: 'queued' });

describe('bulk actions', () => {
	it('apply only where they make sense', () => {
		const all = [paused, done, queued];
		expect(targets(all, 'resume')).toEqual([paused]);
		expect(targets(all, 'pause')).toEqual([queued]);
		expect(targets(all, 'cancel')).toEqual([paused, queued]);
		expect(targets(all, 'remove')).toEqual(all);
	});

	it('ask first only when downloaded data would be deleted', () => {
		expect(confirmation('remove', [done, queued], 'en')).toBe(null);
		expect(confirmation('pause', [paused], 'en')).toBe(null);
		expect(confirmation('cancel', [paused], 'en')).toBe(
			'Cancel “movie night.mkv” and delete what’s downloaded so far?'
		);
		expect(confirmation('remove', [paused, done], 'en')).toBe(
			'Remove 2 downloads and delete what’s downloaded of the unfinished ones?'
		);
	});
});

describe('pause all and resume all', () => {
	const downloading = job({ id: 'a', state: 'downloading' });
	const retrying = job({ id: 'b', state: 'retrying' });
	const waiting = job({ id: 'c', state: 'queued' });
	const stopped = job({ id: 'd', state: 'paused' });
	const failed = job({ id: 'e', state: 'failed' });
	const moving = job({ id: 'f', state: 'paused', moving: true });

	it('pauses waiting downloads before running ones', () => {
		expect(pauseSteps([downloading, waiting, retrying, stopped])).toEqual({
			waiting: ['c'],
			running: ['a', 'b']
		});
	});

	it('resumes paused downloads only', () => {
		expect(resumeTargets([stopped, failed, moving, waiting])).toEqual(['d']);
	});
});

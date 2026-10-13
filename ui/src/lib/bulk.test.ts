import { describe, expect, it } from 'vitest';
import { confirmation, targets } from './bulk';
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

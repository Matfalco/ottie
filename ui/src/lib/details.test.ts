import { describe, expect, it } from 'vitest';
import type { Job } from '@ottie/api-types/Job';
import {
	PRIORITIES,
	asksAboutFile,
	canStream,
	facts,
	isPlayable,
	primaryAction,
	priorityName
} from './details';
import { job, plain } from './testing';

const MB = 1024 * 1024;
// The sample job's ID says it was added 2026-10-12 14:03:07 UTC; the same day.
const NOW = Date.parse('2026-10-12T15:00:00Z');
const labels = (list: { label: string }[]) => list.map((fact) => fact.label);

describe('details', () => {
	it('offer the likely next step as the main button', () => {
		expect(primaryAction(job({ state: 'downloading' }), 'en')).toEqual({
			action: 'pause',
			label: 'Pause'
		});
		expect(primaryAction(job({ state: 'failed' }), 'en')).toEqual({
			action: 'resume',
			label: 'Try again'
		});
		expect(primaryAction(job({ state: 'paused' }), 'fr')?.action).toBe('resume');
		// Its own choices instead: rename or replace (docs/decisions/0049).
		const asking = job({ state: 'failed', problem: { code: 'fileExists', name: 'a.zip' } });
		expect(asksAboutFile(asking)).toBe(true);
		expect(primaryAction(asking, 'en')).toBeNull();
		expect(asksAboutFile(job({ state: 'failed' }))).toBe(false);
		expect(primaryAction(job({ state: 'completed' }), 'en')).toBeNull();
		expect(primaryAction(job({ state: 'scheduled' }), 'en')).toEqual({
			action: 'resume',
			label: 'Start now'
		});
	});

	it('say when a scheduled download starts', () => {
		const tonight = new Date(2026, 9, 12, 23, 0).getTime();
		const later = job({ state: 'scheduled', startAt: tonight });
		const [starts] = facts(later, 0, new Date(2026, 9, 12, 9, 0).getTime(), null, 'en');
		expect(starts).toEqual({ label: 'Starts', value: '11:00 PM' });
	});

	it('list what matters while downloading', () => {
		const running = job({ state: 'downloading', downloaded: 2 * MB, total: 6 * MB });
		const list = facts(running, MB, NOW, 10, 'en');
		expect(labels(list)).toEqual(['Speed', 'Time left', 'Size', 'Added']);
		expect(list.map((fact) => plain(fact.value)).slice(0, 3)).toEqual([
			'1.0 MB/s',
			'4 sec',
			'6.0 MB'
		]);
		// No speed yet: nothing to estimate from.
		expect(facts(running, 0, NOW, 10, 'en')[1].value).toBe('—');
	});

	it('count retries against the setting', () => {
		const retrying = job({ state: 'retrying', retries: 2, retryAt: NOW + 8000 });
		const list = facts(retrying, 0, NOW, 10, 'en');
		expect(labels(list)).toEqual(['Next try', 'Retries', 'Size', 'Added']);
		expect(plain(list[0].value)).toBe('8 sec');
		expect(list[1].value).toBe('2 of 10');
		expect(facts(job({ state: 'failed', retries: 3 }), 0, NOW, null, 'fr')[0].value).toBe('3');
	});

	it('say whether a paused download can resume', () => {
		const paused = job({ state: 'paused', downloaded: MB, resumable: false });
		const list = facts(paused, 0, NOW, null, 'en');
		expect(labels(list)).toEqual(['Downloaded', 'Size', 'Can resume', 'Added']);
		expect(list[1].value).toBe('Unknown');
		expect(list[2].value).toBe('No, starts over');
	});

	it('know a finished download’s size', () => {
		const done = facts(job({ state: 'completed', downloaded: 3 * MB }), 0, NOW, null, 'en');
		expect(plain(done[0].value)).toBe('3.0 MB');
		expect(labels(facts(job({ state: 'queued' }), 0, NOW, null, 'en'))).toEqual([
			'Size',
			'Priority',
			'Added'
		]);
	});

	it('tell what can be played', () => {
		const named = (fileName: string, state: Job['state'] = 'downloading') =>
			job({ fileName, state });
		expect(isPlayable(named('movie night.mkv'))).toBe(true);
		expect(isPlayable(named('song.mp3', 'completed'))).toBe(true);
		expect(isPlayable(named('setup.exe'))).toBe(false);
		// No file yet (the URL looks like a video), or none left.
		expect(isPlayable(job())).toBe(false);
		expect(isPlayable(named('movie night.mkv', 'cancelled'))).toBe(false);
	});

	it('offer streaming mode until the download is over', () => {
		expect(canStream(job())).toBe(true);
		expect(canStream(job({ state: 'paused' }))).toBe(true);
		expect(canStream(job({ state: 'completed' }))).toBe(false);
		expect(canStream(job({ state: 'cancelled' }))).toBe(false);
	});

	it('name the priorities in each language', () => {
		expect(PRIORITIES.map((priority) => priorityName(priority, 'en'))).toEqual([
			'Low',
			'Normal',
			'High',
			'Urgent'
		]);
		expect(priorityName('urgent', 'fr')).toBe('Urgente');
	});
});

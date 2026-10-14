import { describe, expect, it } from 'vitest';
import { job, plain } from './testing';
import { pauseSteps, resumeTargets, trayStatus } from './tray';

const downloading = job({ id: 'a', state: 'downloading' });
const retrying = job({ id: 'b', state: 'retrying' });
const queued = job({ id: 'c', state: 'queued' });
const paused = job({ id: 'd', state: 'paused' });
const failed = job({ id: 'e', state: 'failed' });
const moving = job({ id: 'f', state: 'paused', moving: true });

describe('the tray menu', () => {
	it('says what runs, in both languages', () => {
		const all = [downloading, retrying, queued, paused];
		const speed = 1.5 * 1024 * 1024;
		expect(trayStatus(all, speed, 'live', 'en')).toBe('2 downloading, 1 waiting · 1.5 MB/s');
		expect(plain(trayStatus(all, speed, 'live', 'fr'))).toBe('2 en cours, 1 en attente · 1,5 Mo/s');
		expect(trayStatus([queued], 0, 'live', 'en')).toBe('1 waiting');
		expect(trayStatus([paused, failed], 0, 'live', 'en')).toBe('Nothing downloading');
		expect(trayStatus(all, speed, 'offline', 'en')).toBe(trayStatus([], 0, 'connecting', 'en'));
	});

	it('pauses waiting downloads before running ones', () => {
		expect(pauseSteps([downloading, queued, retrying, paused])).toEqual({
			waiting: ['c'],
			running: ['a', 'b']
		});
	});

	it('resumes paused downloads only', () => {
		expect(resumeTargets([paused, failed, moving, queued])).toEqual(['d']);
	});
});

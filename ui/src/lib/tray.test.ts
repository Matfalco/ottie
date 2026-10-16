import { describe, expect, it } from 'vitest';
import { job, plain } from './testing';
import { trayRunning, trayStatus } from './tray';

const downloading = job({ id: 'a', state: 'downloading' });
const retrying = job({ id: 'b', state: 'retrying' });
const queued = job({ id: 'c', state: 'queued' });
const paused = job({ id: 'd', state: 'paused' });
const failed = job({ id: 'e', state: 'failed' });

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

	it('shows a few running downloads, never just one more', () => {
		const running = (count: number) =>
			Array.from({ length: count }, (_, i) => job({ id: `r${i}`, state: 'connecting' }));
		expect(trayRunning([queued, downloading, paused, retrying])).toEqual({
			shown: [downloading, retrying],
			more: 0
		});
		expect(trayRunning(running(4)).shown).toHaveLength(4);
		const five = trayRunning(running(5));
		expect(five.shown.map((job) => job.id)).toEqual(['r0', 'r1', 'r2']);
		expect(five.more).toBe(2);
	});
});

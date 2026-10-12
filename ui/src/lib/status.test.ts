import { describe as group, expect, it } from 'vitest';
import { job } from './format.test';
import { canCancel, canPause, canResume, describe, fraction } from './status';

const MB = 1024 * 1024;

group('describe', () => {
	it('shows amount, speed and time left while downloading', () => {
		const downloading = job({ state: 'downloading', downloaded: 10 * MB, total: 30 * MB });
		expect(describe(downloading, 2 * MB, 0)).toBe('10 MB of 30 MB · 2.0 MB/s · 10 s left');
		expect(describe(downloading, 0, 0)).toBe('10 MB of 30 MB');
		const unknownSize = job({ state: 'downloading', downloaded: 3 * MB });
		expect(describe(unknownSize, MB, 0)).toBe('3.0 MB · 1.0 MB/s');
	});

	it('explains waiting, failing and retrying', () => {
		expect(describe(job(), 0, 0)).toBe('Waiting for a free slot');
		const failed = job({ state: 'failed', error: 'server answered 404 Not Found' });
		expect(describe(failed, 0, 0)).toBe('Failed: server answered 404 Not Found');
		const retrying = job({
			state: 'retrying',
			retries: 3,
			retryAt: 9_000,
			error: 'server answered 503'
		});
		expect(describe(retrying, 0, 1_000)).toBe('Retrying in 8 s (retry 3) · server answered 503');
		const gaveWay = job({ retries: 4, error: 'server answered 503' });
		expect(describe(gaveWay, 0, 0)).toBe('Waiting for a free slot to retry · server answered 503');
	});

	it('sums up finished jobs', () => {
		expect(describe(job({ state: 'completed', total: 5 * MB }), 0, 0)).toBe('5.0 MB · Done');
		expect(describe(job({ state: 'cancelled' }), 0, 0)).toBe('Cancelled');
	});
});

group('progress and buttons', () => {
	it('knows the fraction only with a size', () => {
		expect(fraction(job({ downloaded: 25, total: 100 }))).toBe(0.25);
		expect(fraction(job({ downloaded: 25 }))).toBeNull();
		expect(fraction(job({ state: 'completed' }))).toBe(1);
	});

	it('offers what the state allows', () => {
		expect(canPause(job({ state: 'retrying' }))).toBe(true);
		expect(canResume(job({ state: 'failed' }))).toBe(true);
		expect(canResume(job({ state: 'downloading' }))).toBe(false);
		expect(canCancel(job({ state: 'completed' }))).toBe(false);
		expect(canCancel(job({ state: 'paused' }))).toBe(true);
	});
});

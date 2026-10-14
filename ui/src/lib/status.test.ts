import { describe as group, expect, it } from 'vitest';
import {
	canCancel,
	canPause,
	canResume,
	describe,
	fraction,
	isFinished,
	motion,
	removeLosesData
} from './status';
import { job, plain } from './testing';

const MB = 1024 * 1024;

group('describe', () => {
	it('shows amount, speed and time left while downloading', () => {
		const downloading = job({ state: 'downloading', downloaded: 10 * MB, total: 30 * MB });
		expect(describe(downloading, 2 * MB, 0, 'en')).toBe('10 MB of 30 MB · 2.0 MB/s · 10 sec left');
		expect(describe(downloading, 0, 0, 'en')).toBe('10 MB of 30 MB');
		expect(plain(describe(downloading, 2 * MB, 0, 'fr'))).toBe(
			'10 Mo sur 30 Mo · 2,0 Mo/s · encore 10 s'
		);
		const unknownSize = job({ state: 'downloading', downloaded: 3 * MB });
		expect(describe(unknownSize, MB, 0, 'en')).toBe('3.0 MB · 1.0 MB/s');
	});

	it('says when the speed limit gives a download no bandwidth', () => {
		const held = job({
			state: 'downloading',
			downloaded: 10 * MB,
			total: 30 * MB,
			waitingForBandwidth: true
		});
		expect(describe(held, 0, 0, 'en')).toBe('10 MB of 30 MB · Waiting for bandwidth');
		expect(motion(held, 0, 0, 'fr')).toEqual({ status: 'En attente de bande passante' });
	});

	it('explains waiting, failing and retrying from the problem code', () => {
		expect(describe(job(), 0, 0, 'en')).toBe('Waiting for a free slot');
		const failed = job({
			state: 'failed',
			error: 'server answered 404 Not Found',
			problem: { code: 'httpStatus', status: 404 }
		});
		expect(describe(failed, 0, 0, 'en')).toBe('Failed: Not found on the server (404)');
		expect(plain(describe(failed, 0, 0, 'fr'))).toBe('Échec : Introuvable sur le serveur (404)');
		const retrying = job({
			state: 'retrying',
			retries: 3,
			retryAt: 9_000,
			error: 'server answered 503 Service Unavailable',
			problem: { code: 'httpStatus', status: 503 }
		});
		expect(describe(retrying, 0, 1_000, 'en')).toBe(
			'Retrying in 8 sec (retry 3) · The server has a problem (503)'
		);
		const gaveWay = job({ retries: 4, problem: { code: 'stalled', seconds: 60 } });
		expect(describe(gaveWay, 0, 0, 'en')).toBe(
			'Waiting for a free slot to retry · No data from the server for 1 min'
		);
	});

	it('sums up finished jobs', () => {
		expect(describe(job({ state: 'completed', total: 5 * MB }), 0, 0, 'en')).toBe('5.0 MB · Done');
		expect(describe(job({ state: 'cancelled' }), 0, 0, 'fr')).toBe('Annulé');
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

	it('offers nothing while the file moves, and says so', () => {
		const moving = job({ state: 'downloading', moving: true });
		expect([canPause(moving), canResume(moving), canCancel(moving)]).toEqual([false, false, false]);
		expect(describe(moving, MB, 0, 'en')).toBe('Moving to its new folder…');
		expect(motion(moving, MB, 0, 'fr')).toEqual({
			status: 'Déplacement vers son nouveau dossier…'
		});
	});
});

group('removing', () => {
	it('asks first only when downloaded data would be deleted', () => {
		expect(removeLosesData(job({ state: 'paused', downloaded: 10 }))).toBe(true);
		expect(removeLosesData(job({ state: 'queued' }))).toBe(false);
		expect(removeLosesData(job({ state: 'completed', downloaded: 10 }))).toBe(false);
	});

	it('counts completed and cancelled jobs as finished', () => {
		expect(isFinished(job({ state: 'cancelled' }))).toBe(true);
		expect(isFinished(job({ state: 'failed' }))).toBe(false);
	});
});

group('motion', () => {
	it('gives speed and time left while downloading, else a status', () => {
		const downloading = job({ state: 'downloading', downloaded: 10 * MB, total: 30 * MB });
		expect(motion(downloading, 2 * MB, 0, 'en')).toEqual({ speed: '2.0 MB/s', left: '10 sec' });
		expect(motion(downloading, 0, 0, 'en')).toEqual({ speed: '—', left: '—' });
		expect(motion(job({ state: 'paused', downloaded: MB }), 0, 0, 'en')).toEqual({
			status: 'Paused'
		});
		expect(motion(job(), 0, 0, 'en')).toEqual({ status: 'Waiting for a free slot' });
	});
});

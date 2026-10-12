import { describe, expect, it } from 'vitest';
import type { Job } from '@ottie/api-types/Job';
import { displayName, formatBytes, formatDuration, formatSpeed } from './format';

export function job(overrides: Partial<Job> = {}): Job {
	return {
		id: '2026.10.12-14.03.07-1',
		url: 'https://example.com/files/movie%20night.mkv?token=x',
		destDir: '/downloads',
		fileName: null,
		state: 'queued',
		downloaded: 0,
		total: null,
		resumable: false,
		error: null,
		priority: 'normal',
		speedLimit: 0,
		category: 1,
		retries: 0,
		retryAt: null,
		...overrides
	};
}

describe('formatBytes', () => {
	it('uses 1024 and drops decimals for big numbers', () => {
		expect(formatBytes(0)).toBe('0 B');
		expect(formatBytes(1023)).toBe('1023 B');
		expect(formatBytes(1536)).toBe('1.5 KB');
		expect(formatBytes(234 * 1024 * 1024)).toBe('234 MB');
		expect(formatBytes(5 * 1024 ** 5)).toBe('5120 TB');
	});

	it('adds /s for speeds', () => {
		expect(formatSpeed(2.5 * 1024 * 1024)).toBe('2.5 MB/s');
	});
});

describe('formatDuration', () => {
	it('picks the two largest units', () => {
		expect(formatDuration(44.6)).toBe('45 s');
		expect(formatDuration(200)).toBe('3 min 20 s');
		expect(formatDuration(180)).toBe('3 min');
		expect(formatDuration(7500)).toBe('2 h 5 min');
		expect(formatDuration(-3)).toBe('0 s');
	});
});

describe('displayName', () => {
	it('prefers the real file name, then the URL', () => {
		expect(displayName(job({ fileName: 'Movie.mkv' }))).toBe('Movie.mkv');
		expect(displayName(job())).toBe('movie night.mkv');
		expect(displayName(job({ url: 'https://example.com/' }))).toBe('example.com');
	});
});

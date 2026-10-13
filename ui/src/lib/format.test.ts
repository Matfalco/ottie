import { describe, expect, it } from 'vitest';
import {
	displayName,
	formatAdded,
	formatBytes,
	formatCount,
	formatDuration,
	formatPercent,
	formatSpeed
} from './format';
import { job, plain } from './testing';

const MB = 1024 * 1024;

describe('formatBytes', () => {
	it('uses 1024, one decimal below 10, and the language’s units', () => {
		expect(formatBytes(0, 'en')).toBe('0 bytes');
		expect(formatBytes(1023, 'en')).toBe('1,023 bytes');
		expect(formatBytes(1536, 'en')).toBe('1.5 kB');
		expect(formatBytes(234 * MB, 'en')).toBe('234 MB');
		expect(plain(formatBytes(1023, 'fr'))).toBe('1 023 octets');
		expect(plain(formatBytes(1536, 'fr'))).toBe('1,5 ko');
		expect(plain(formatBytes(5 * 1024 ** 5, 'fr'))).toBe('5 120 To');
	});

	it('speaks speeds per second', () => {
		expect(formatSpeed(2.5 * MB, 'en')).toBe('2.5 MB/s');
		expect(plain(formatSpeed(2.5 * MB, 'fr'))).toBe('2,5 Mo/s');
	});
});

describe('formatCount', () => {
	it('separates thousands the language’s way', () => {
		expect(formatCount(1234, 'en')).toBe('1,234');
		expect(plain(formatCount(1234, 'fr'))).toBe('1 234');
	});
});

describe('formatPercent', () => {
	it('shows whole percents, never rounding up to 100', () => {
		expect(formatPercent(0.8, 'en')).toBe('80%');
		expect(formatPercent(0.999, 'en')).toBe('99%');
		expect(plain(formatPercent(0.5, 'fr'))).toBe('50 %');
	});
});

describe('formatAdded', () => {
	const now = new Date(2026, 9, 12, 18, 0);
	it('shows the time for today, else the date', () => {
		expect(formatAdded(new Date(2026, 9, 12, 14, 3), now, 'fr')).toBe('14:03');
		expect(plain(formatAdded(new Date(2026, 9, 3, 9, 0), now, 'en'))).toBe('Oct 3');
		expect(plain(formatAdded(new Date(2025, 0, 5), now, 'fr'))).toBe('5 janv. 2025');
	});
});

describe('formatDuration', () => {
	it('picks the two largest units', () => {
		expect(formatDuration(44.6, 'en')).toBe('45 sec');
		expect(formatDuration(200, 'en')).toBe('3 min 20 sec');
		expect(formatDuration(180, 'en')).toBe('3 min');
		expect(formatDuration(-3, 'en')).toBe('0 sec');
		expect(plain(formatDuration(7500, 'fr'))).toBe('2 h 5 min');
	});
});

describe('displayName', () => {
	it('prefers the real file name, then the URL', () => {
		expect(displayName(job({ fileName: 'Movie.mkv' }))).toBe('Movie.mkv');
		expect(displayName(job())).toBe('movie night.mkv');
		expect(displayName(job({ url: 'https://example.com/' }))).toBe('example.com');
	});
});

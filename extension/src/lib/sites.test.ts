import { describe, expect, it } from 'vitest';
import { isExcluded, parseSites, siteOf } from './sites';

describe('sites', () => {
	it('keeps the host name of what was typed', () => {
		expect(siteOf(' Example.COM ')).toBe('example.com');
		expect(siteOf('https://www.example.com/files?x=1')).toBe('example.com');
		expect(siteOf('dl.example.com:8080')).toBe('dl.example.com');
		expect(siteOf('')).toBeNull();
		expect(siteOf('http://')).toBeNull();
	});

	it('reads one per line, or separated, each once', () => {
		expect(parseSites('example.com\n\nother.org, example.com  www.third.net')).toEqual([
			'example.com',
			'other.org',
			'third.net'
		]);
	});

	it('covers a site and its subdomains only', () => {
		const sites = ['example.com'];
		expect(isExcluded('https://example.com/a', sites)).toBe(true);
		expect(isExcluded('https://cdn.dl.example.com/a', sites)).toBe(true);
		expect(isExcluded('https://notexample.com/a', sites)).toBe(false);
		expect(isExcluded('https://example.com.evil.org/a', sites)).toBe(false);
		expect(isExcluded('nonsense', sites)).toBe(false);
	});
});

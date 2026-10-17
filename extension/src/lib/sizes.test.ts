import { describe, expect, it } from 'vitest';
import { megabytes, parseMegabytes, toMegabytes } from './sizes';

describe('sizes', () => {
	it('reads megabytes with a point or a comma', () => {
		expect(parseMegabytes('1,5')).toBe(1.5 * 1024 ** 2);
		expect(parseMegabytes(' 2 ')).toBe(2 * 1024 ** 2);
		expect(parseMegabytes('')).toBe(0);
		expect(parseMegabytes('-1')).toBeNull();
		expect(parseMegabytes('ten')).toBeNull();
	});

	it('shows bytes as megabytes, in the page’s language', () => {
		expect(toMegabytes(1.5 * 1024 ** 2)).toBe(1.5);
		expect(megabytes('en')).toBe('MB');
		expect(megabytes('fr')).toBe('Mo');
	});
});

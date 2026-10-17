import { describe, expect, it } from 'vitest';
import { isExtensionOrigin, popupRows, tokenIn } from './popup';
import { job } from './testing';

describe('popupRows', () => {
	it('lists running ones first, then failed, waiting and finished', () => {
		const jobs = [
			job({ id: '6', state: 'completed' }),
			job({ id: '5', state: 'queued' }),
			job({ id: '4', state: 'failed' }),
			job({ id: '3', state: 'downloading' })
		];
		expect(popupRows(jobs).rows.map((j) => j.id)).toEqual(['3', '4', '5', '6']);
	});

	it('shows six and counts the rest, but never leaves just one out', () => {
		const jobs = (n: number) =>
			Array.from({ length: n }, (_, i) => job({ id: String(i), state: 'queued' }));
		expect(popupRows(jobs(7))).toMatchObject({ more: 0 });
		expect(popupRows(jobs(7)).rows).toHaveLength(7);
		expect(popupRows(jobs(9)).rows).toHaveLength(6);
		expect(popupRows(jobs(9)).more).toBe(3);
	});
});

describe('the extension', () => {
	it('is recognised by its origin', () => {
		expect(isExtensionOrigin('chrome-extension://abcdefghijklmnop')).toBe(true);
		expect(isExtensionOrigin('moz-extension://0b1c-22')).toBe(true);
		expect(isExtensionOrigin('https://example.com')).toBe(false);
		expect(isExtensionOrigin('chrome-extension://abc/page.html')).toBe(false);
	});

	it('hands a token over, nothing else', () => {
		expect(tokenIn({ type: 'ottie-token', token: 'ab12' })).toBe('ab12');
		expect(tokenIn({ type: 'other', token: 'ab12' })).toBeNull();
		expect(tokenIn({ type: 'ottie-token', token: 'a b' })).toBeNull();
		expect(tokenIn('ottie-token')).toBeNull();
		expect(tokenIn(null)).toBeNull();
	});
});

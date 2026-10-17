import { describe, expect, it } from 'vitest';
import { parseSignInLink } from './signin';

describe('parseSignInLink', () => {
	it('reads the address and token', () => {
		expect(parseSignInLink(' http://127.0.0.1:24068/#token=0a1b2c ')).toEqual({
			address: 'http://127.0.0.1:24068',
			token: '0a1b2c'
		});
		expect(parseSignInLink('https://nas.example.ts.net/#token=xyz')).toEqual({
			address: 'https://nas.example.ts.net',
			token: 'xyz'
		});
	});

	it('refuses what isn’t one', () => {
		for (const text of [
			'',
			'http://127.0.0.1:24068/',
			'ftp://host/#token=abc',
			'http://host/#token=',
			'nonsense'
		]) {
			expect(parseSignInLink(text)).toBeNull();
		}
	});
});

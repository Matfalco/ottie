import { describe, expect, it } from 'vitest';
import { signInLink, tokenFromFragment } from './token';

describe('sign-in links', () => {
	it('carry the token where the page finds it again', () => {
		const link = signInLink('http://127.0.0.1:24068', 'abc123');
		expect(link).toBe('http://127.0.0.1:24068/#token=abc123');
		expect(tokenFromFragment(new URL(link).hash)).toBe('abc123');
	});
});

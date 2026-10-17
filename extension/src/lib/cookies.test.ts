import { describe, expect, it } from 'vitest';
import { cookieHeader } from './cookies';

describe('cookieHeader', () => {
	it('puts longer paths first, as browsers do', () => {
		const cookies = [
			{ name: 'a', value: '1', path: '/' },
			{ name: 'b', value: '2', path: '/files' },
			{ name: '', value: 'bare', path: '/' }
		];
		expect(cookieHeader(cookies)).toBe('b=2; a=1; bare');
	});

	it('is null without cookies', () => {
		expect(cookieHeader([])).toBeNull();
	});
});

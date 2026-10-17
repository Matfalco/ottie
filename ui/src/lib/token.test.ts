import { describe, expect, it } from 'vitest';
import { fromFragment, handLinks, signInLink, takeHandedLinks, tokenFromFragment } from './token';

describe('sign-in links', () => {
	it('carry the token where the page finds it again', () => {
		const link = signInLink('http://127.0.0.1:24068', 'abc123');
		expect(link).toBe('http://127.0.0.1:24068/#token=abc123');
		expect(tokenFromFragment(new URL(link).hash)).toBe('abc123');
	});
});

describe('fromFragment', () => {
	it('reads the token alone, or with links to add', () => {
		expect(fromFragment('#token=ab12')).toEqual({ token: 'ab12', links: null });
		const links = encodeURIComponent('https://a.example/1.zip\nhttps://b.example/2.zip');
		expect(fromFragment(`#token=ab12&links=${links}`)).toEqual({
			token: 'ab12',
			links: 'https://a.example/1.zip\nhttps://b.example/2.zip'
		});
	});

	it('takes nothing else', () => {
		expect(fromFragment('')).toEqual({ token: null, links: null });
		expect(fromFragment('#links=x')).toEqual({ token: null, links: null });
		expect(fromFragment('#token=a%20b&links=x')).toEqual({ token: null, links: 'x' });
		expect(fromFragment('#token=ab&links=%20')).toEqual({ token: 'ab', links: null });
	});

	it('hands the links over once', () => {
		handLinks('https://a.example/1.zip');
		expect(takeHandedLinks()).toBe('https://a.example/1.zip');
		expect(takeHandedLinks()).toBeNull();
	});
});

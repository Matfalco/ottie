import { describe, expect, it } from 'vitest';
import { fromFragment } from '#ui/token.js';
import { batchAddress } from './menus';

describe('batchAddress', () => {
	it('opens Ottie signed in with the links, as its page reads them', () => {
		const links = 'https://a.example/1.zip\nhttps://b.example/a b.zip';
		const address = batchAddress('http://127.0.0.1:24068', 'ab12', links);
		expect(address.startsWith('http://127.0.0.1:24068/#token=ab12&links=')).toBe(true);
		expect(fromFragment(new URL(address).hash)).toEqual({ token: 'ab12', links });
	});
});

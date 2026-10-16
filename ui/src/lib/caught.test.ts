import { describe, expect, it } from 'vitest';
import type { FoundLink } from '@ottie/api-types/FoundLink';
import { caughtRows, caughtTitle, othersText, outcome } from './caught';
import { category, plain } from './testing';

function link(name: string, categoryId = 2): FoundLink {
	return {
		kind: 'link',
		url: `https://example.com:8080/files/${name}`,
		name,
		matched: { category: categoryId, extension: 'zip', byFolder: false },
		known: false,
		exists: false
	};
}

const categories = {
	1: category(),
	2: category({ id: 2, name: 'Archives', extensions: ['zip'] })
};

describe('caught', () => {
	it('says where each link goes, and from where', () => {
		const invalid: FoundLink = {
			kind: 'invalid',
			text: 'ftp://example.com/a.zip',
			refused: { error: '…', code: 'unsupportedScheme', scheme: 'ftp' }
		};
		expect(
			caughtRows([link('a.zip'), invalid, link('b.zip', 1), link('c.zip', 9)], categories, 'fr')
		).toEqual([
			{
				url: 'https://example.com:8080/files/a.zip',
				name: 'a.zip',
				category: 'Archives',
				host: 'example.com:8080'
			},
			{
				url: 'https://example.com:8080/files/b.zip',
				name: 'b.zip',
				category: 'Général',
				host: 'example.com:8080'
			},
			{
				url: 'https://example.com:8080/files/c.zip',
				name: 'c.zip',
				category: null,
				host: 'example.com:8080'
			}
		]);
	});

	it('counts several links, and those not listed', () => {
		expect(caughtTitle(1, 'en')).toBe('Copied link');
		expect(caughtTitle(1234, 'en')).toBe('Copied links: 1,234');
		expect(plain(caughtTitle(1234, 'fr'))).toBe('Liens copiés : 1 234');
		expect(othersText(3, 'en')).toBeNull();
		expect(othersText(5, 'en')).toBe('2 more');
		expect(othersText(5, 'fr')).toBe('2 de plus');
	});

	it('tells what was added and why the others were not', () => {
		const sent = ['https://x.org/a.zip', 'https://x.org/b.zip', 'https://x.org/c.zip'];
		expect(
			outcome(
				sent,
				{
					ids: ['2026.10.15-10.00.00-1', null, '2026.10.15-10.00.00-2'],
					refused: [{ index: 1, error: 'no folder', code: 'noFolder', category: 'Archives' }]
				},
				'en'
			)
		).toEqual({
			added: ['https://x.org/a.zip', 'https://x.org/c.zip'],
			refused: { 'https://x.org/b.zip': expect.stringContaining('Archives') }
		});
	});
});
